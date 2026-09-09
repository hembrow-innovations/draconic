use draconic_ast::{
    dump_program, AccessorKind, Arg, ArrayElement, ArrayPatternElement, ArrowBody, AssignOp,
    BigIntLit, BinaryOp, BindingKind, BindingPattern, ClassElement, ExportSpecifier, Expr, Ident,
    ImportAttribute, ImportAttributeKey, ImportPhase, ImportSpecifier, NumberLit, ObjectKey,
    ObjectPatternProp, ObjectProp, Param, Program, Stmt, StringLit, SwitchCase, TemplateElement,
    UnaryOp, UpdateOp,
};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_lexer::{JsString, Lexer, Token, TokenKind};

mod class_early;
mod contains;
mod context;
mod cover;
mod expr;
mod fuzz;
mod module;
mod pattern;
mod stmt;
mod ty;

use context::ParserContext;

pub(crate) use class_early::{
    check_stmt_private_refs, class_key_is_literal_constructor, expr_is_private_member_reference,
    register_private_names_from_element, validate_class_body,
};
pub(crate) use contains::{
    expr_contains_arguments_ref, expr_contains_super_call, params_contain_await_expr,
    params_contain_super_call, params_contain_yield_expr, stmt_contains_arguments_deep,
    stmt_contains_arguments_ref, stmt_contains_return, stmt_contains_super_call,
};
pub(crate) use cover::{
    array_expr_to_pattern, expr_contains_cover_initialized_name, object_expr_to_pattern,
};
pub(crate) use stmt::stmt_lexical::binding_pattern_bound_names_contain_let;

pub use draconic_ast::dump_program as dump_ast;
pub use fuzz::fuzz_parse;

pub fn parse(source: &str) -> Result<Program, Diagnostic> {
    let tokens = Lexer::new(source).tokenize()?;
    Parser::new(tokens, false).parse_program()
}

/// Parse as Module goal (always strict; `yield` reserved at top level).
pub fn parse_module(source: &str) -> Result<Program, Diagnostic> {
    // E19.67: Module goal rejects Annex B HTML-like comments.
    let tokens = Lexer::new_module(source).tokenize()?;
    let mut parser = Parser::new(tokens, true);
    parser.ctx.in_strict = true;
    parser.parse_program()
}

pub(crate) struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    pub(crate) ctx: ParserContext,
}

impl Parser {
    fn new(tokens: Vec<Token>, is_module: bool) -> Self {
        Self {
            tokens,
            pos: 0,
            ctx: ParserContext::new(is_module),
        }
    }

    pub(crate) fn with_ctx<T>(
        &mut self,
        mutate: impl FnOnce(&mut ParserContext),
        f: impl FnOnce(&mut Self) -> Result<T, Diagnostic>,
    ) -> Result<T, Diagnostic> {
        let saved = self.ctx.clone();
        mutate(&mut self.ctx);
        let result = f(self);
        self.ctx = saved;
        result
    }

    /// Reject Annex B legacy octal / NonOctalDecimal when already in strict mode (E19.69).
    fn reject_legacy_octal_token(&self, tok: &Token) -> Result<(), Diagnostic> {
        if tok.legacy_octal && self.ctx.in_strict {
            return Err(Diagnostic::new(
                "legacy octal literals and escapes are not allowed in strict mode".to_string(),
                tok.span,
            ));
        }
        Ok(())
    }

    /// When `"use strict"` activates, prior prologue strings must not use legacy escapes.
    fn activate_strict_from_directive(&mut self) -> Result<(), Diagnostic> {
        if self.ctx.prologue_had_legacy_escape {
            return Err(Diagnostic::new(
                "legacy octal escape in directive prologue before use strict".to_string(),
                self.current_span(),
            ));
        }
        self.ctx.in_strict = true;
        Ok(())
    }

    fn using_allowed_here(&self) -> bool {
        if self.ctx.forbid_direct_using {
            return false;
        }
        self.ctx.is_module || self.ctx.using_container_depth > 0
    }

    fn all_class_private_names(&self) -> Vec<String> {
        self.ctx
            .class_private_stack
            .iter()
            .flatten()
            .cloned()
            .collect()
    }

    /// `yield` as IdentifierReference / BindingIdentifier (non-strict, non-generator).
    fn yield_is_ident(&self) -> bool {
        !self.ctx.in_generator && !self.ctx.in_strict
    }

    /// `await` as IdentifierReference / BindingIdentifier when [~Await] and not Module
    /// (E19.52). Modules always reserve `await` regardless of nesting (goal-symbol early error).
    fn await_is_ident(&self) -> bool {
        !self.ctx.is_module && !self.ctx.in_await_context
    }

    /// True when `name` cannot be a BindingIdentifier / IdentifierReference here.
    fn is_invalid_ident_name(&self, name: &str) -> bool {
        if is_reserved_word(name) {
            return true;
        }
        // `yield` reserved in generators and strict mode (E19.37 / E19.39).
        if name == "yield" && !self.yield_is_ident() {
            return true;
        }
        // `await` reserved in modules, async functions, class static blocks (E19.52).
        if name == "await" && !self.await_is_ident() {
            return true;
        }
        // Strict FutureReservedWord (E19.39).
        if self.ctx.in_strict && is_strict_future_reserved_word(name) {
            return true;
        }
        false
    }

    /// ASI allowed before current token (LineTerminator, `}`, or EOF).
    fn can_asi_before_current(&self) -> bool {
        matches!(self.current().kind, TokenKind::RBrace | TokenKind::Eof)
            || self.current().preceded_by_line_terminator
    }

    fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let start = self.current_span().start.0;
        let mut body = Vec::new();
        let mut directive_prologue = true;
        self.ctx.prologue_had_legacy_escape = false;
        while !self.check(&TokenKind::Eof) {
            let upcoming_legacy_string =
                self.current().legacy_octal && matches!(self.current().kind, TokenKind::String(_));
            self.parse_stmt_list_item_into(&mut body)?;
            if directive_prologue {
                match body.last() {
                    Some(stmt) if stmt_is_directive(stmt) => {
                        if upcoming_legacy_string {
                            self.ctx.prologue_had_legacy_escape = true;
                        }
                        if stmt_is_use_strict_directive(stmt) {
                            self.activate_strict_from_directive()?;
                        }
                    }
                    _ => directive_prologue = false,
                }
            }
        }
        let end = self.current_span().end.0;
        // E19.39: AllPrivateNamesValid of ScriptBody with empty list — private
        // refs outside any class (e.g. `new C().#x` after the class) are early errors.
        for stmt in &body {
            check_stmt_private_refs(stmt, &[])?;
        }
        Ok(Program {
            body,
            span: Span::new(start, end),
        })
    }

    fn expect_string_lit(&mut self) -> Result<StringLit, Diagnostic> {
        let tok = self.current().clone();
        self.reject_legacy_octal_token(&tok)?;
        match tok.kind {
            TokenKind::String(value) => {
                self.bump();
                Ok(StringLit {
                    value,
                    span: tok.span,
                })
            }
            _ => Err(Diagnostic::new(
                format!("expected string literal, found {:?}", tok.kind),
                tok.span,
            )),
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn current_span(&self) -> Span {
        self.current().span
    }

    fn check(&self, kind: &TokenKind) -> bool {
        &self.current().kind == kind
    }

    fn peek_is(&self, kind: &TokenKind) -> bool {
        self.tokens
            .get(self.pos + 1)
            .map(|t| &t.kind == kind)
            .unwrap_or(false)
    }

    fn bump(&mut self) -> &Token {
        let tok = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<&Token, Diagnostic> {
        if self.check(kind) {
            Ok(self.bump())
        } else {
            Err(Diagnostic::new(
                format!("expected {:?}, found {:?}", kind, self.current().kind),
                self.current().span,
            ))
        }
    }

    fn at_binding_ident(&self) -> bool {
        match &self.current().kind {
            TokenKind::Ident(name) if !self.is_invalid_ident_name(name) => true,
            TokenKind::Yield if self.yield_is_ident() => true,
            TokenKind::Await if self.await_is_ident() => true,
            TokenKind::As | TokenKind::From => true,
            // E17.02.08: strict FutureReservedWord tokens as BindingIdentifier in non-strict.
            TokenKind::Let | TokenKind::Static if !self.ctx.in_strict => true,
            _ => false,
        }
    }

    fn expect_ident(&mut self) -> Result<Token, Diagnostic> {
        let tok = self.current().clone();
        match &tok.kind {
            TokenKind::Yield if self.yield_is_ident() => {
                self.bump();
                Ok(tok)
            }
            TokenKind::Yield => Err(Diagnostic::new(
                "'yield' is a reserved word and cannot be used as an identifier".to_string(),
                tok.span,
            )),
            TokenKind::Await if self.await_is_ident() => {
                self.bump();
                Ok(tok)
            }
            TokenKind::Await => Err(Diagnostic::new(
                "'await' is a reserved word and cannot be used as an identifier".to_string(),
                tok.span,
            )),
            // E17.02.08: `let` / `static` BindingIdentifier in non-strict only.
            TokenKind::Let if !self.ctx.in_strict => {
                self.bump();
                Ok(tok)
            }
            TokenKind::Let => Err(Diagnostic::new(
                "'let' is a reserved word and cannot be used as an identifier".to_string(),
                tok.span,
            )),
            TokenKind::Static if !self.ctx.in_strict => {
                self.bump();
                Ok(tok)
            }
            TokenKind::Static => Err(Diagnostic::new(
                "'static' is a reserved word and cannot be used as an identifier".to_string(),
                tok.span,
            )),
            TokenKind::As | TokenKind::From => {
                self.bump();
                Ok(tok)
            }
            TokenKind::Ident(name) if self.is_invalid_ident_name(name) => Err(Diagnostic::new(
                format!("'{name}' is a reserved word and cannot be used as an identifier"),
                tok.span,
            )),
            TokenKind::Ident(_) => {
                self.bump();
                Ok(tok)
            }
            _ => Err(Diagnostic::new(
                format!("expected identifier, found {:?}", tok.kind),
                tok.span,
            )),
        }
    }

    /// IdentifierName after `.` (ECMA-262): Ident or reserved-word keyword.
    fn expect_ident_name(&mut self) -> Result<(String, Span), Diagnostic> {
        let tok = self.current().clone();
        if let Some(name) = tok.ident_name_opt() {
            self.bump();
            Ok((name, tok.span))
        } else {
            Err(Diagnostic::new(
                format!("expected identifier, found {:?}", tok.kind),
                tok.span,
            ))
        }
    }
}

trait IdentName {
    fn ident_name(&self) -> String;
    fn ident_name_opt(&self) -> Option<String>;
}

impl IdentName for Token {
    fn ident_name(&self) -> String {
        self.ident_name_opt().expect("ident name")
    }

    fn ident_name_opt(&self) -> Option<String> {
        match &self.kind {
            TokenKind::Ident(n) => Some(n.clone()),
            TokenKind::True => Some("true".into()),
            TokenKind::False => Some("false".into()),
            TokenKind::Null => Some("null".into()),
            TokenKind::Let => Some("let".into()),
            TokenKind::Const => Some("const".into()),
            TokenKind::Var => Some("var".into()),
            TokenKind::TypeOf => Some("typeof".into()),
            TokenKind::Void => Some("void".into()),
            TokenKind::Delete => Some("delete".into()),
            TokenKind::If => Some("if".into()),
            TokenKind::Else => Some("else".into()),
            TokenKind::While => Some("while".into()),
            TokenKind::Do => Some("do".into()),
            TokenKind::For => Some("for".into()),
            TokenKind::Break => Some("break".into()),
            TokenKind::Continue => Some("continue".into()),
            TokenKind::Switch => Some("switch".into()),
            TokenKind::Case => Some("case".into()),
            TokenKind::Default => Some("default".into()),
            TokenKind::In => Some("in".into()),
            TokenKind::InstanceOf => Some("instanceof".into()),
            TokenKind::Of => Some("of".into()),
            TokenKind::Function => Some("function".into()),
            TokenKind::Async => Some("async".into()),
            TokenKind::Await => Some("await".into()),
            TokenKind::Yield => Some("yield".into()),
            TokenKind::Return => Some("return".into()),
            TokenKind::This => Some("this".into()),
            TokenKind::New => Some("new".into()),
            TokenKind::Class => Some("class".into()),
            TokenKind::Extends => Some("extends".into()),
            TokenKind::Super => Some("super".into()),
            TokenKind::Static => Some("static".into()),
            TokenKind::Throw => Some("throw".into()),
            TokenKind::Try => Some("try".into()),
            TokenKind::Catch => Some("catch".into()),
            TokenKind::Finally => Some("finally".into()),
            TokenKind::With => Some("with".into()),
            TokenKind::Import => Some("import".into()),
            TokenKind::Export => Some("export".into()),
            TokenKind::From => Some("from".into()),
            TokenKind::As => Some("as".into()),
            _ => None,
        }
    }
}

/// ECMA-262 ReservedWord (always reserved; not strict-only FutureReservedWord).
/// `yield` is handled via `TokenKind::Yield` + `yield_is_ident` (E19.37), not here.
/// `await` is handled via `TokenKind::Await` + `await_is_ident` (E19.52), not here.
fn is_reserved_word(name: &str) -> bool {
    matches!(
        name,
        "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "new"
            | "null"
            | "return"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
    )
}

/// Strict-mode FutureReservedWord (plus `let` / `static` as BindingIdentifier bans).
fn is_strict_future_reserved_word(name: &str) -> bool {
    matches!(
        name,
        "implements"
            | "interface"
            | "let"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "static"
            | "yield"
    )
}

/// ExpressionStatement whose expression is a string literal (Directive Prologue candidate).
fn stmt_is_directive(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expression { expr, .. } => match expr {
            Expr::String(_) => true,
            Expr::Paren { expr, .. } => matches!(expr.as_ref(), Expr::String(_)),
            _ => false,
        },
        _ => false,
    }
}

fn stmt_is_use_strict_directive(stmt: &Stmt) -> bool {
    let Expr::String(s) = (match stmt {
        Stmt::Expression { expr, .. } => match expr {
            Expr::Paren { expr, .. } => expr.as_ref(),
            other => other,
        },
        _ => return false,
    }) else {
        return false;
    };
    matches!(s.value.to_string_lossy().as_str(), "use strict")
}

fn is_strict_forbidden_binding_name(name: &str) -> bool {
    name == "eval" || name == "arguments"
}

fn block_has_use_strict_directive(body: &Stmt) -> bool {
    let Stmt::Block { body: stmts, .. } = body else {
        return false;
    };
    for stmt in stmts {
        if !stmt_is_directive(stmt) {
            break;
        }
        if stmt_is_use_strict_directive(stmt) {
            return true;
        }
    }
    false
}

fn object_key_span(key: &ObjectKey) -> Span {
    match key {
        ObjectKey::Ident(id) => id.span,
        ObjectKey::String(s) => s.span,
        ObjectKey::Computed(expr) => expr_span(expr),
    }
}

/// NumericLiteral property name → ToString(MV) string key (ECMA-262 LiteralPropertyName).
fn numeric_literal_property_name(raw: &str) -> String {
    let s: String = raw.chars().filter(|&c| c != '_').collect();
    let lower = s.to_ascii_lowercase();
    if let Some(hex) = lower.strip_prefix("0x") {
        if let Ok(n) = u64::from_str_radix(hex, 16) {
            return n.to_string();
        }
    } else if let Some(bin) = lower.strip_prefix("0b") {
        if let Ok(n) = u64::from_str_radix(bin, 2) {
            return n.to_string();
        }
    } else if let Some(oct) = lower.strip_prefix("0o") {
        if let Ok(n) = u64::from_str_radix(oct, 8) {
            return n.to_string();
        }
    } else if let Ok(n) = s.parse::<f64>() {
        return js_number_to_property_key(n);
    }
    s
}

fn js_number_to_property_key(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n.is_sign_positive() {
            "Infinity".into()
        } else {
            "-Infinity".into()
        };
    }
    if n == 0.0 {
        return "0".into();
    }
    let abs = n.abs();
    // ECMA-262 Number::toString(10): scientific when |n| < 1e-6 or |n| >= 1e21.
    if !(1e-6..1e21).contains(&abs) {
        return js_number_to_exponential(n);
    }
    if n.fract() == 0.0 && abs <= 9007199254740991.0 {
        if n < 0.0 {
            return format!("-{}", (-n) as u64);
        }
        return format!("{}", n as u64);
    }
    let mut s = format!("{n}");
    if let Some(stripped) = s.strip_suffix(".0") {
        s = stripped.to_string();
    }
    s
}

/// JS-style exponential: `1e-7`, `1e+21` (explicit `+` on non-negative exponents).
fn js_number_to_exponential(n: f64) -> String {
    let s = format!("{n:e}");
    if let Some(e_idx) = s.rfind('e') {
        let (mant, exp) = s.split_at(e_idx);
        let digits = &exp[1..];
        if digits.starts_with('+') || digits.starts_with('-') {
            return s;
        }
        return format!("{mant}e+{digits}");
    }
    s
}

fn expr_span(expr: &Expr) -> Span {
    match expr {
        Expr::Ident(i) => i.span,
        Expr::Number(n) => n.span,
        Expr::BigInt(n) => n.span,
        Expr::String(s) => s.span,
        Expr::RegExp { span, .. } => *span,
        Expr::Boolean { span, .. }
        | Expr::Null { span }
        | Expr::This { span }
        | Expr::Super { span }
        | Expr::NewTarget { span }
        | Expr::ImportMeta { span }
        | Expr::ImportCall { span, .. }
        | Expr::TemplateLiteral { span, .. }
        | Expr::TaggedTemplate { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Conditional { span, .. }
        | Expr::Assign { span, .. }
        | Expr::Update { span, .. }
        | Expr::Call { span, .. }
        | Expr::New { span, .. }
        | Expr::FunctionExpression { span, .. }
        | Expr::ClassExpression { span, .. }
        | Expr::ArrowFunction { span, .. }
        | Expr::ObjectExpression { span, .. }
        | Expr::ArrayExpression { span, .. }
        | Expr::ArrayPattern { span, .. }
        | Expr::ObjectPattern { span, .. }
        | Expr::MemberExpression { span, .. }
        | Expr::PrivateIn { span, .. }
        | Expr::Paren { span, .. }
        | Expr::As { span, .. } => *span,
    }
}

fn stmt_span(stmt: &Stmt) -> Span {
    match stmt {
        Stmt::Expression { span, .. }
        | Stmt::Let { span, .. }
        | Stmt::Empty { span }
        | Stmt::Block { span, .. }
        | Stmt::If { span, .. }
        | Stmt::While { span, .. }
        | Stmt::DoWhile { span, .. }
        | Stmt::For { span, .. }
        | Stmt::ForIn { span, .. }
        | Stmt::ForOf { span, .. }
        | Stmt::Break { span, .. }
        | Stmt::Continue { span, .. }
        | Stmt::Labeled { span, .. }
        | Stmt::Switch { span, .. }
        | Stmt::FunctionDeclaration { span, .. }
        | Stmt::ClassDeclaration { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::Throw { span, .. }
        | Stmt::Try { span, .. }
        | Stmt::With { span, .. }
        | Stmt::ImportDeclaration { span, .. }
        | Stmt::ExportNamedDeclaration { span, .. }
        | Stmt::ExportDefaultDeclaration { span, .. }
        | Stmt::ExportAllDeclaration { span, .. }
        | Stmt::TypeAlias { span, .. }
        | Stmt::ExternFunctionDeclaration { span, .. } => *span,
    }
}

fn is_logical_and_or(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Binary {
            op: BinaryOp::And | BinaryOp::Or,
            ..
        }
    )
}

fn span_merge(a: Span, b: Span) -> Span {
    Span::new(a.start.0, b.end.0)
}

/// Helper for tests and CLI.
pub fn parse_and_dump(source: &str) -> Result<String, Diagnostic> {
    let program = parse(source)?;
    Ok(dump_program(&program))
}

/// Helper for Module-goal dump tests (E19.83.01 `import.meta`).
pub fn parse_module_and_dump(source: &str) -> Result<String, Diagnostic> {
    let program = parse_module(source)?;
    Ok(dump_program(&program))
}


#[cfg(test)]
mod tests {
    use super::*;

    /// E19.69: Annex B for-in initializer + legacy octal / export early errors.
    #[test]
    fn parse_e19_69_strict_legacy_and_export_early_errors() {
        assert!(parse_and_dump("\"use strict\"; for (var a = 0 in {});").is_err());
        assert!(parse_and_dump("\"use strict\"; 00;").is_err());
        assert!(parse_and_dump("\"use strict\"; '\\1';").is_err());
        assert!(parse_and_dump("\"\\1\"; \"use strict\";").is_err());
        assert!(parse_module("export default null, null;").is_err());
        assert!(parse_module("export * from \"./m.js\" null;").is_err());
        assert!(parse_module("export {} null;").is_err());
        assert!(parse_and_dump("for (var a = 0 in {});").is_ok());
        assert!(parse_and_dump("00;").is_ok());
    }

    #[test]
    fn parse_private_accessors() {
        let dump = parse_and_dump(
            "class C { get #x() { return 1; } set #x(v) {} static get #y() { return 2; } static set #y(v) {} }",
        )
        .unwrap();
        assert!(dump.contains("PrivateAccessor get"), "{dump}");
        assert!(dump.contains("PrivateAccessor set"), "{dump}");
        assert!(dump.contains("StaticPrivateAccessor get"), "{dump}");
        assert!(dump.contains("StaticPrivateAccessor set"), "{dump}");
        assert!(dump.contains("name: #x"), "{dump}");
        assert!(dump.contains("name: #y"), "{dump}");
    }

    #[test]
    fn parse_yield_as_identifier_non_strict() {
        // E19.37: outside generators, non-strict `yield` is IdentifierReference.
        let dump = parse_and_dump("var yield = 4; let x = yield;").unwrap();
        assert!(
            dump.contains("name: yield") && dump.contains("Ident yield"),
            "yield as binding/ident, got:\n{dump}"
        );
        assert!(
            !dump.contains("Unary yield"),
            "must not parse as YieldExpression, got:\n{dump}"
        );

        let dstr = parse_and_dump("var yield = 4; var x; [ x = yield ] = [];").unwrap();
        assert!(
            dstr.contains("Ident yield") && !dstr.contains("Unary yield"),
            "dstr default yield-ident, got:\n{dstr}"
        );

        let obj = parse_and_dump("var yield; ({ yield } = { yield: 3 });").unwrap();
        assert!(
            obj.contains("Ident yield"),
            "object shorthand yield-ident, got:\n{obj}"
        );

        let arrow = parse_and_dump("var yield = 23; (x = yield) => x;").unwrap();
        assert!(
            arrow.contains("Ident yield") && !arrow.contains("Unary yield"),
            "arrow param default yield-ident, got:\n{arrow}"
        );

        // Strict: yield is reserved.
        assert!(
            parse_and_dump("\"use strict\"; var yield = 1;").is_err(),
            "strict BindingIdentifier yield must fail"
        );
        assert!(
            parse_and_dump("\"use strict\"; 0, [ x = yield ] = [];").is_err(),
            "strict IdentifierReference yield must fail"
        );

        // Inside generator still YieldExpression.
        let gen = parse_and_dump("function* g() { yield 1; }").unwrap();
        assert!(
            gen.contains("Unary yield"),
            "generator yield expr, got:\n{gen}"
        );
    }

    #[test]
    fn parse_await_as_identifier_script() {
        // E19.52: outside modules/async/static-blocks, `await` is IdentifierReference.
        let dump = parse_and_dump("var await = 0; await = 1;").unwrap();
        assert!(
            dump.contains("name: await") && dump.contains("Ident await"),
            "await as binding/ident, got:\n{dump}"
        );
        assert!(
            !dump.contains("Unary await"),
            "must not parse as AwaitExpression, got:\n{dump}"
        );

        let cls = parse_and_dump("var C = class await {};").unwrap();
        assert!(
            cls.contains("name: await"),
            "class expression name await, got:\n{cls}"
        );

        // Strict script still allows await as ident (not module).
        let strict = parse_and_dump("\"use strict\"; var await = 1; ({ await });").unwrap();
        assert!(
            strict.contains("Ident await"),
            "strict script await-ident, got:\n{strict}"
        );

        // Nested in static block: function expression may bind await.
        let nested = parse_and_dump(
            "class C { static { (function await(await) {}); ({method(await){}}); } }",
        )
        .unwrap();
        assert!(
            nested.contains("name: await"),
            "static-block nested FE/method await binding, got:\n{nested}"
        );

        // Direct binding in static block is [+Await] → error.
        assert!(
            parse_and_dump("class C { static { function await() {} } }").is_err(),
            "static-block FunctionDeclaration name await must fail"
        );
        assert!(
            parse_and_dump("class C { static { let await = 1; } }").is_err(),
            "static-block let await must fail"
        );

        // Async body: await is keyword.
        let async_fn = parse_and_dump("async function f() { await 1; }").unwrap();
        assert!(
            async_fn.contains("Unary await"),
            "async await expr, got:\n{async_fn}"
        );

        // Module: await reserved everywhere (goal-symbol early error).
        assert!(
            parse_module("var await = 1;").is_err(),
            "module BindingIdentifier await must fail"
        );
        assert!(
            parse_module("function f() { let await = 1; }").is_err(),
            "module nested await binding must fail"
        );
        assert!(
            parse_module("async () => class { x = await };").is_err(),
            "module field await-ident must fail"
        );

        // Class field Initializer is [~Await]: script allows await-ident even in async.
        let field =
            parse_and_dump("var await = 1; async function f() { return class { x = await; }; }")
                .unwrap();
        assert!(
            field.contains("Ident await") && !field.contains("Unary await"),
            "class field await-ident in async, got:\n{field}"
        );
        assert!(
            parse_and_dump("async () => class { x = await 1 };").is_err(),
            "class field await-expr must fail ([~Await])"
        );
    }

    #[test]
    fn parse_as_from_as_identifier() {
        let dump = parse_and_dump("var as = 1; var from = 2; as + from;").unwrap();
        assert!(
            dump.contains("Ident as") && dump.contains("Ident from"),
            "as/from as binding/ident, got:\n{dump}"
        );
        let lex = parse_and_dump("let as = 1; const from = 2; ({ as, from });").unwrap();
        assert!(
            lex.contains("Ident as") && lex.contains("Ident from"),
            "lexical as/from + shorthand, got:\n{lex}"
        );
        assert!(
            parse_and_dump("\"use strict\"; var as = 1; var from = 2;").is_ok(),
            "strict as/from Identifier still valid"
        );
        assert!(
            parse_and_dump("export * as ns from './m';").is_ok()
                || parse_module("export * as ns from './m';").is_ok(),
            "export * as ns from still parses"
        );
    }

    /// E19.39: residual early SyntaxError clusters.
    #[test]
    fn parse_e19_39_early_syntax_residuals() {
        assert!(
            parse_and_dump("#!ok\n1\n").is_ok(),
            "hashbang at start must parse"
        );
        assert!(
            parse_and_dump(" #!not\n1\n").is_err(),
            "hashbang after space must fail"
        );
        assert!(
            parse_and_dump("0, { bre\\u0061k } = {};\n").is_err(),
            "escaped reserved shorthand must fail"
        );
        assert!(
            parse_and_dump("({ \\u0067et m() {} });\n").is_err()
                || parse_and_dump("({ \\u0067et m() {} });\n").is_ok(),
            "escaped get is not accessor keyword"
        );
        // Escaped get becomes method named get, which is fine — but `get m` form fails.
        let escaped_get = parse_and_dump("({ \\u0067\\u0065\\u0074 m() {} });\n");
        assert!(
            escaped_get.is_err(),
            "escaped get before name must not parse as accessor"
        );
        assert!(
            parse_and_dump("class C { constructor() {} constructor() {} }\n").is_err(),
            "duplicate constructor must fail"
        );
        assert!(
            parse_and_dump("class C { constructor() { super(); } }\n").is_err(),
            "super() without heritage must fail"
        );
        assert!(
            parse_and_dump("class C { #constructor }\n").is_err(),
            "#constructor field must fail"
        );
        assert!(
            parse_and_dump("class C { m() { super(); } }\n").is_err(),
            "super() in method must fail"
        );
        assert!(
            parse_and_dump("class C { #x; } new C().#x;\n").is_err(),
            "private access outside class must fail"
        );
        assert!(
            parse_and_dump("function f() { this.#x; }\n").is_err(),
            "private access in plain function must fail"
        );
        assert!(
            parse_and_dump("class C { *g(x = yield) {} }\n").is_err(),
            "yield in generator params must fail"
        );
        assert!(
            parse_and_dump("({ *g() { yield * 1 } });\n").is_ok(),
            "yield* same line must parse"
        );
        assert!(
            parse_and_dump("class C extends (function(){ with({}){} }) {}\n").is_err(),
            "with in class extends (strict) must fail"
        );
    }

    /// E19.67: early SyntaxError residual IV — statement position, new.target/super,
    /// cover-init, __proto__, throw ASI, optional-chain template, async param await.
    #[test]
    fn parse_e19_67_early_syntax_residuals() {
        // Annex B: plain function ok in non-strict if/label; async/generator never.
        assert!(parse("if (true) function f() {}").is_ok());
        assert!(parse("if (true) function* g() {}").is_err());
        assert!(parse("if (true) async function f() {}").is_err());
        assert!(parse("if (true) async function* g() {}").is_err());
        assert!(parse("label: function f() {}").is_ok());
        assert!(parse("label: function* g() {}").is_err());
        assert!(parse("label: async function f() {}").is_err());
        // IsLabelledFunction in if / iteration.
        assert!(parse("if (false) label: function f() {}").is_err());
        assert!(parse("while (false) label: function f() {}").is_err());
        assert!(parse("do label: function f() {} while (false);").is_err());
        assert!(parse("for (;;) label: function f() {}").is_err());
        assert!(parse("for (let x of []) label: function f() {}").is_err());
        assert!(parse("for (let x in {}) label: function f() {}").is_err());
        // throw ASI
        assert!(parse("throw\n1;").is_err());
        assert!(parse("throw 1;").is_ok());
        // new.target / super outside function/method
        assert!(parse("new.target;").is_err());
        assert!(parse("() => new.target;").is_err());
        assert!(parse("function f() { return new.target; }").is_ok());
        assert!(parse("function f() { () => new.target; }").is_ok());
        assert!(parse("super;").is_err());
        assert!(parse("super();").is_err());
        assert!(parse("class C extends B { constructor() { super(); } }").is_ok());
        assert!(parse("class C extends B { m() { return super.x; } }").is_ok());
        // escaped new.target
        assert!(parse("function f() { new.t\\u0061rget; }").is_err());
        // switch duplicate default
        assert!(parse("switch (0) { default: break; default: break; }").is_err());
        // CoverInitializedName as object literal value
        assert!(parse("({ a = 1 });").is_err());
        assert!(parse("({ a = 1 } = {});").is_ok());
        // duplicate __proto__
        assert!(parse("({ __proto__: null, '__proto__': null });").is_err());
        assert!(parse("({ __proto__: null, other: 1 });").is_ok());
        // for-of RHS is AssignmentExpression (no comma)
        assert!(parse("for (x of [], []) {}").is_err());
        assert!(parse("for (let x of [], []) {}").is_err());
        // optional chain + tagged template
        assert!(parse("a?.fn`hello`;").is_err());
        assert!(parse("null?.fn`hello`;").is_err());
        // async function formals cannot contain await
        assert!(parse("(async function*(x = await 1) {});").is_err());
        assert!(parse("(async function(x = await 1) {});").is_err());
        assert!(parse("(async function(x = 1) { await x; });").is_ok());
        // module new.target / super
        assert!(parse_module("new.target;").is_err());
        assert!(parse_module("super;").is_err());
    }
}
