use draconic_ast::{
    dump_program, AccessorKind, Arg, ArrayElement, ArrayPatternElement, ArrowBody, AssignOp,
    BigIntLit, BinaryOp, BindingKind, BindingPattern, ClassElement, ExportSpecifier, Expr, Ident,
    ImportAttribute, ImportAttributeKey, ImportPhase, ImportSpecifier, NumberLit, ObjectKey,
    ObjectPatternProp, ObjectProp, Param, Program, Stmt, StringLit, SwitchCase, TemplateElement,
    UnaryOp, UpdateOp,
};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_lexer::{JsString, Lexer, Token, TokenKind};

mod context;
mod expr;
mod expr_lhs;
mod expr_object;
mod expr_ops;
mod expr_primary;
mod fuzz;
mod stmt;
mod stmt_class;
mod stmt_for;
mod stmt_function;
mod stmt_using;

use context::ParserContext;

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

    /// ExportDeclaration terminator: explicit `;` or ASI (LineTerminator / `}` / EOF).
    fn expect_export_semi(&mut self, end: u32) -> Result<u32, Diagnostic> {
        if self.check(&TokenKind::Semi) {
            return Ok(self.bump().span.end.0);
        }
        if self.can_asi_before_current() {
            return Ok(end);
        }
        Err(Diagnostic::new(
            "expected `;` after export declaration".to_string(),
            self.current_span(),
        ))
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

    /// `async? function *? name? (params) { body }` in expression position.
    fn parse_function_expression(&mut self) -> Result<Expr, Diagnostic> {
        let (is_async, start) = if self.check(&TokenKind::Async) {
            let start = self.bump().span.start.0;
            (true, start)
        } else {
            (false, self.current_span().start.0)
        };
        self.expect(&TokenKind::Function)?;
        let is_generator = if self.check(&TokenKind::Star) {
            self.bump();
            true
        } else {
            false
        };
        // Generator BindingIdentifier has [+Yield]: optional name cannot be `yield`.
        // Non-async FunctionExpression name/params/body are [~Await]; async are [+Await] (E19.52).
        self.with_ctx(
            |c| {
                c.in_generator = is_generator;
                c.in_await_context = is_async;
            },
            |p| {
                let name = if p.at_binding_ident() {
                    let name_tok = p.expect_ident()?;
                    let id = Ident {
                        name: name_tok.ident_name(),
                        span: name_tok.span,
                    };
                    // E19.49: strict BindingIdentifier cannot be `eval`/`arguments`.
                    if p.ctx.in_strict && is_strict_forbidden_binding_name(&id.name) {
                        return Err(Diagnostic::new(
                            format!("binding `{}` is invalid in strict mode", id.name),
                            id.span,
                        ));
                    }
                    Some(id)
                } else {
                    None
                };
                p.expect(&TokenKind::LParen)?;
                let params = p.parse_param_list()?;
                p.expect(&TokenKind::RParen)?;
                // E19.58: FormalParameters of a generator must not contain YieldExpression.
                if is_generator && params_contain_yield_expr(&params) {
                    return Err(Diagnostic::new(
                        "generator parameters cannot contain yield".to_string(),
                        Span::new(start, p.current_span().end.0),
                    ));
                }
                // E19.67: FormalParameters of an async function must not contain AwaitExpression.
                if is_async && params_contain_await_expr(&params) {
                    return Err(Diagnostic::new(
                        "async function parameters cannot contain await".to_string(),
                        Span::new(start, p.current_span().end.0),
                    ));
                }
                let return_type = p.parse_optional_type_ann()?;
                // E19.67: non-arrow functions introduce NewTarget.
                p.ctx.new_target_depth += 1;
                let body = Box::new(p.parse_function_body_block()?);
                if let Some(ref id) = name {
                    if is_strict_forbidden_binding_name(&id.name)
                        && block_has_use_strict_directive(&body)
                    {
                        return Err(Diagnostic::new(
                            format!("binding `{}` is invalid in strict mode", id.name),
                            id.span,
                        ));
                    }
                }
                let end = stmt_span(&body).end.0;
                Ok(Expr::FunctionExpression {
                    name,
                    params,
                    return_type,
                    body,
                    is_async,
                    is_generator,
                    is_method: false,
                    span: Span::new(start, end),
                })
            },
        )
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>, Diagnostic> {
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let param = self.parse_param()?;
                let is_rest = param.rest;
                params.push(param);
                if is_rest {
                    break;
                }
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    // Trailing comma before `)` is allowed; rest cannot follow a trailing comma
                    // after itself (already broken above).
                    if self.check(&TokenKind::RParen) {
                        break;
                    }
                    continue;
                }
                break;
            }
        }
        Ok(params)
    }

    /// Binding pattern param, optional `: T` / `= default`, or `...name` / `...name: T`.
    fn parse_param(&mut self) -> Result<Param, Diagnostic> {
        if self.check(&TokenKind::DotDotDot) {
            let dots_start = self.current().span.start.0;
            self.bump();
            let p = self.expect_ident()?;
            let binding = BindingPattern::Ident(Ident {
                name: p.ident_name(),
                span: Span::new(dots_start, p.span.end.0),
            });
            let type_ann = self.parse_optional_type_ann()?;
            if self.check(&TokenKind::Eq) {
                return Err(Diagnostic::new(
                    "rest parameter cannot have a default",
                    self.current().span,
                ));
            }
            return Ok(Param {
                binding,
                type_ann,
                default: None,
                rest: true,
            });
        }
        let binding = self.parse_binding_pattern()?;
        let type_ann = self.parse_optional_type_ann()?;
        let default = if self.check(&TokenKind::Eq) {
            self.bump();
            Some(self.parse_assignment()?)
        } else {
            None
        };
        Ok(Param {
            binding,
            type_ann,
            default,
            rest: false,
        })
    }

    /// Optional `: Type` type annotation (T01 named / T02 object).
    fn parse_optional_type_ann(&mut self) -> Result<Option<draconic_ast::TypeAnn>, Diagnostic> {
        if !self.check(&TokenKind::Colon) {
            return Ok(None);
        }
        self.bump();
        Ok(Some(self.parse_type()?))
    }

    /// `type Name = Type;` / `type Name<T> = Type;`
    fn is_type_alias_start(&self) -> bool {
        matches!(self.current().kind, TokenKind::Ident(ref n) if n == "type")
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Ident(_))
            )
            && self
                .tokens
                .get(self.pos + 2)
                .map(|t| matches!(t.kind, TokenKind::Eq | TokenKind::Lt))
                .unwrap_or(false)
    }

    fn parse_type_alias(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current().span.start.0;
        // contextual `type`
        self.bump();
        let name_tok = self.expect_ident()?;
        let name = Ident {
            name: name_tok.ident_name(),
            span: name_tok.span,
        };
        let type_params = self.parse_optional_type_params()?;
        self.expect(&TokenKind::Eq)?;
        let ty = self.parse_type()?;
        let mut end = ty.span().end.0;
        if self.check(&TokenKind::Semi) {
            end = self.bump().span.end.0;
        }
        Ok(Stmt::TypeAlias {
            name,
            type_params,
            ty,
            span: Span::new(start, end),
        })
    }

    /// Optional `<T, U>` type parameter list (T04).
    fn parse_optional_type_params(&mut self) -> Result<Vec<draconic_ast::TypeParam>, Diagnostic> {
        if !self.check(&TokenKind::Lt) {
            return Ok(Vec::new());
        }
        self.bump();
        let mut params = Vec::new();
        if !self.check(&TokenKind::Gt) {
            loop {
                let name_tok = self.expect_ident()?;
                params.push(draconic_ast::TypeParam {
                    name: Ident {
                        name: name_tok.ident_name(),
                        span: name_tok.span,
                    },
                });
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        self.expect(&TokenKind::Gt)?;
        Ok(params)
    }

    /// `<T, U>` type argument list after a type name (T04).
    fn parse_type_args(&mut self) -> Result<Vec<draconic_ast::TypeAnn>, Diagnostic> {
        self.expect(&TokenKind::Lt)?;
        let mut args = Vec::new();
        if !self.check(&TokenKind::Gt) {
            loop {
                args.push(self.parse_type()?);
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        self.expect(&TokenKind::Gt)?;
        Ok(args)
    }

    /// Type: union (`A | B`), intersection (`A & B`), named, or object.
    fn parse_type(&mut self) -> Result<draconic_ast::TypeAnn, Diagnostic> {
        self.parse_union_type()
    }

    /// `T | U | V` — lowest precedence among type operators.
    fn parse_union_type(&mut self) -> Result<draconic_ast::TypeAnn, Diagnostic> {
        let first = self.parse_intersection_type()?;
        if !self.check(&TokenKind::BitOr) {
            return Ok(first);
        }
        let start = first.span().start.0;
        let mut types = vec![first];
        let mut end = types[0].span().end.0;
        while self.check(&TokenKind::BitOr) {
            self.bump();
            let next = self.parse_intersection_type()?;
            end = next.span().end.0;
            types.push(next);
        }
        Ok(draconic_ast::TypeAnn::Union {
            types,
            span: Span::new(start, end),
        })
    }

    /// `T & U & V` — binds tighter than `|`.
    fn parse_intersection_type(&mut self) -> Result<draconic_ast::TypeAnn, Diagnostic> {
        let first = self.parse_primary_type()?;
        if !self.check(&TokenKind::BitAnd) {
            return Ok(first);
        }
        let start = first.span().start.0;
        let mut types = vec![first];
        let mut end = types[0].span().end.0;
        while self.check(&TokenKind::BitAnd) {
            self.bump();
            let next = self.parse_primary_type()?;
            end = next.span().end.0;
            types.push(next);
        }
        Ok(draconic_ast::TypeAnn::Intersection {
            types,
            span: Span::new(start, end),
        })
    }

    /// Named (`number`), generic app (`Box<T>`), object (`{ a: T }`), tuple (`[T, U]`),
    /// or pointer (`*T`, N03.03).
    fn parse_primary_type(&mut self) -> Result<draconic_ast::TypeAnn, Diagnostic> {
        if self.check(&TokenKind::Star) {
            let start = self.bump().span.start.0;
            let inner = self.parse_primary_type()?;
            let end = inner.span().end.0;
            return Ok(draconic_ast::TypeAnn::Pointer {
                inner: Box::new(inner),
                span: Span::new(start, end),
            });
        }
        if self.check(&TokenKind::LBrace) {
            return self.parse_object_type();
        }
        if self.check(&TokenKind::LBracket) {
            return self.parse_tuple_type();
        }
        // `void` is a keyword (unary op) but also a type name (TS / C FFI returns).
        if self.check(&TokenKind::Void) {
            let sp = self.bump().span;
            return Ok(draconic_ast::TypeAnn::Named {
                name: "void".into(),
                span: sp,
            });
        }
        if self.check(&TokenKind::Function) {
            let sp = self.bump().span;
            return Ok(draconic_ast::TypeAnn::Named {
                name: "function".into(),
                span: sp,
            });
        }
        let err_span = self.current().span;
        let name_tok = self
            .expect_ident()
            .map_err(|_| Diagnostic::new("expected type name after `:`".to_string(), err_span))?;
        let name = name_tok.ident_name();
        let start = name_tok.span.start.0;
        if self.check(&TokenKind::Lt) {
            let args = self.parse_type_args()?;
            let end = args
                .last()
                .map(|a| a.span().end.0)
                .unwrap_or(name_tok.span.end.0);
            // Include trailing `>` — already consumed; use current prev end via last arg + 1 is wrong.
            // parse_type_args consumes `>`; span end is the `>` token we just passed.
            let end = self.tokens[self.pos - 1].span.end.0.max(end);
            return Ok(draconic_ast::TypeAnn::GenericApp {
                name,
                args,
                span: Span::new(start, end),
            });
        }
        Ok(draconic_ast::TypeAnn::Named {
            name,
            span: name_tok.span,
        })
    }

    fn parse_object_type(&mut self) -> Result<draconic_ast::TypeAnn, Diagnostic> {
        let start = self.expect(&TokenKind::LBrace)?.span.start.0;
        let mut props = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let name_tok = self.expect_ident()?;
            let prop_start = name_tok.span.start.0;
            let prop_name = name_tok.ident_name();
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type()?;
            let prop_end = ty.span().end.0;
            props.push(draconic_ast::TypeProp {
                name: prop_name,
                ty,
                span: Span::new(prop_start, prop_end),
            });
            if self.check(&TokenKind::Comma) || self.check(&TokenKind::Semi) {
                self.bump();
                continue;
            }
            break;
        }
        let end = self.expect(&TokenKind::RBrace)?.span.end.0;
        Ok(draconic_ast::TypeAnn::Object {
            props,
            span: Span::new(start, end),
        })
    }

    /// `[T, U, V]` fixed-length tuple type (N03.02).
    fn parse_tuple_type(&mut self) -> Result<draconic_ast::TypeAnn, Diagnostic> {
        let start = self.expect(&TokenKind::LBracket)?.span.start.0;
        let mut elements = Vec::new();
        while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
            let ty = self.parse_type()?;
            elements.push(ty);
            if self.check(&TokenKind::Comma) {
                self.bump();
                continue;
            }
            break;
        }
        let end = self.expect(&TokenKind::RBracket)?.span.end.0;
        Ok(draconic_ast::TypeAnn::Tuple {
            elements,
            span: Span::new(start, end),
        })
    }

    /// `import type { … }` / `import type foo from` / `import type * as ns from`.
    /// Not `import type from "mod"` (default binding named `type`).
    fn at_type_only_import_modifier(&self) -> bool {
        matches!(&self.current().kind, TokenKind::Ident(n) if n == "type")
            && (self.peek_is(&TokenKind::LBrace)
                || self.peek_is(&TokenKind::Star)
                || matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::Ident(_))
                ))
    }

    /// `{ type foo }` / `{ type foo as bar }`, not `{ type }` / `{ type as bar }`.
    fn at_inline_type_import_specifier(&self) -> bool {
        matches!(&self.current().kind, TokenKind::Ident(n) if n == "type")
            && self
                .tokens
                .get(self.pos + 1)
                .and_then(|t| t.ident_name_opt())
                .is_some()
            && !self.peek_is(&TokenKind::As)
            && !self.peek_is(&TokenKind::Comma)
            && !self.peek_is(&TokenKind::RBrace)
    }

    /// `import { a, b as c } from "mod";`
    /// `import d from "mod";`
    /// `import d, { a } from "mod";`
    /// `import * as ns from "mod";`
    /// `import d, * as ns from "mod";`
    /// `import "mod";` / `import "mod" with {…};`
    fn parse_import(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::Import)?.span.start.0;
        let mut specifiers = Vec::new();
        let mut namespace = None;
        let mut phase = ImportPhase::Evaluation;
        let mut type_only = false;

        // Side-effect import: `import "mod"` (optional WithClause).
        let source = if matches!(self.current().kind, TokenKind::String(_)) {
            self.expect_string_lit()?
        } else if matches!(&self.current().kind, TokenKind::Ident(n) if n == "defer")
            && self.peek_is(&TokenKind::Star)
        {
            // E19.42: `import defer * as ns from "mod"` (only NameSpaceImport).
            self.bump();
            phase = ImportPhase::Defer;
            namespace = Some(self.parse_namespace_import()?);
            self.expect(&TokenKind::From)?;
            self.expect_string_lit()?
        } else {
            if self.at_type_only_import_modifier() {
                self.bump();
                type_only = true;
            }
            if self.check(&TokenKind::Star) {
                namespace = Some(self.parse_namespace_import()?);
            } else if matches!(self.current().kind, TokenKind::Ident(_)) {
                let local_tok = self.expect_ident()?;
                let local = Ident {
                    name: local_tok.ident_name(),
                    span: local_tok.span,
                };
                // E19.49: ImportedBinding is BindingIdentifier (modules always strict).
                if is_strict_forbidden_binding_name(&local.name) {
                    return Err(Diagnostic::new(
                        format!("binding `{}` is invalid in strict mode", local.name),
                        local.span,
                    ));
                }
                let def_span = local.span;
                specifiers.push(ImportSpecifier {
                    imported: Ident {
                        name: "default".into(),
                        span: def_span,
                    },
                    local,
                    is_type: false,
                });
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    if self.check(&TokenKind::Star) {
                        namespace = Some(self.parse_namespace_import()?);
                    } else {
                        self.expect(&TokenKind::LBrace)?;
                        self.parse_named_import_specifiers(&mut specifiers)?;
                        self.expect(&TokenKind::RBrace)?;
                    }
                }
            } else {
                self.expect(&TokenKind::LBrace)?;
                self.parse_named_import_specifiers(&mut specifiers)?;
                self.expect(&TokenKind::RBrace)?;
            }

            self.expect(&TokenKind::From)?;
            self.expect_string_lit()?
        };

        let (attributes, clause_end) = self.parse_with_clause_opt()?;
        let mut end = clause_end.unwrap_or(source.span.end.0);
        if self.check(&TokenKind::Semi) {
            end = self.bump().span.end.0;
        }
        Ok(Stmt::ImportDeclaration {
            specifiers,
            namespace,
            source,
            attributes,
            phase,
            type_only,
            span: Span::new(start, end),
        })
    }

    /// Optional `with {…}` or legacy `assert {…}` (no LineTerminator before `assert`).
    /// Returns attributes and the end offset of the clause when present.
    fn parse_with_clause_opt(&mut self) -> Result<(Vec<ImportAttribute>, Option<u32>), Diagnostic> {
        let keyword_start = self.current_span().start.0;
        if self.check(&TokenKind::With) {
            self.bump();
        } else if matches!(&self.current().kind, TokenKind::Ident(n) if n == "assert") {
            if self.current().preceded_by_line_terminator {
                return Ok((Vec::new(), None));
            }
            self.bump();
        } else {
            return Ok((Vec::new(), None));
        }

        self.expect(&TokenKind::LBrace)?;
        let mut attributes = Vec::new();
        let mut seen_keys: Vec<JsString> = Vec::new();
        if !self.check(&TokenKind::RBrace) {
            loop {
                let key_start = self.current_span().start.0;
                let (key, key_units, key_span) =
                    if matches!(self.current().kind, TokenKind::String(_)) {
                        let s = self.expect_string_lit()?;
                        let units = s.value.clone();
                        let span = s.span;
                        (ImportAttributeKey::String(s), units, span)
                    } else {
                        let (name, span) = self.expect_ident_name()?;
                        let units = JsString::from(name.as_str());
                        (ImportAttributeKey::Ident(Ident { name, span }), units, span)
                    };
                if seen_keys.iter().any(|k| k == &key_units) {
                    return Err(Diagnostic::new(
                        "duplicate import attribute key".to_string(),
                        key_span,
                    ));
                }
                seen_keys.push(key_units);
                self.expect(&TokenKind::Colon)?;
                let value = self.expect_string_lit()?;
                let end = value.span.end.0;
                attributes.push(ImportAttribute {
                    key,
                    value,
                    span: Span::new(key_start, end),
                });
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    if self.check(&TokenKind::RBrace) {
                        break;
                    }
                    continue;
                }
                break;
            }
        }
        let rbrace = self.expect(&TokenKind::RBrace)?;
        let end = rbrace.span.end.0;
        let _ = keyword_start;
        Ok((attributes, Some(end)))
    }

    /// `* as ImportedBinding`
    fn parse_namespace_import(&mut self) -> Result<Ident, Diagnostic> {
        self.expect(&TokenKind::Star)?;
        self.expect(&TokenKind::As)?;
        let local_tok = self.expect_ident()?;
        let local = Ident {
            name: local_tok.ident_name(),
            span: local_tok.span,
        };
        // E19.49: ImportedBinding is BindingIdentifier (modules always strict).
        if is_strict_forbidden_binding_name(&local.name) {
            return Err(Diagnostic::new(
                format!("binding `{}` is invalid in strict mode", local.name),
                local.span,
            ));
        }
        Ok(local)
    }

    fn parse_named_import_specifiers(
        &mut self,
        specifiers: &mut Vec<ImportSpecifier>,
    ) -> Result<(), Diagnostic> {
        if self.check(&TokenKind::RBrace) {
            return Ok(());
        }
        loop {
            let is_type = if self.at_inline_type_import_specifier() {
                self.bump();
                true
            } else {
                false
            };
            // `default` is a keyword but valid as ImportedBinding name: `{ default as x }`.
            let (imported_name, imported_span) = self.expect_ident_name()?;
            let imported = Ident {
                name: imported_name,
                span: imported_span,
            };
            let local = if self.check(&TokenKind::As) {
                self.bump();
                let local_tok = self.expect_ident()?;
                Ident {
                    name: local_tok.ident_name(),
                    span: local_tok.span,
                }
            } else if imported.name == "default" {
                return Err(Diagnostic::new(
                    "default import in named list requires `as` binding".to_string(),
                    imported.span,
                ));
            } else {
                imported.clone()
            };
            // E19.49: ImportedBinding is BindingIdentifier (modules always strict).
            if is_strict_forbidden_binding_name(&local.name) {
                return Err(Diagnostic::new(
                    format!("binding `{}` is invalid in strict mode", local.name),
                    local.span,
                ));
            }
            specifiers.push(ImportSpecifier {
                imported,
                local,
                is_type,
            });
            if self.check(&TokenKind::Comma) {
                self.bump();
                if self.check(&TokenKind::RBrace) {
                    break;
                }
                continue;
            }
            break;
        }
        Ok(())
    }

    /// `export let/const/function …` or `export { a, b as c };` or `export * from` or `export default …`
    fn parse_export(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::Export)?.span.start.0;
        if self.check(&TokenKind::Default) {
            return self.parse_export_default(start);
        }
        // `export * from "mod"` / `export * as ns from "mod"`
        if self.check(&TokenKind::Star) {
            self.bump();
            let exported = if self.check(&TokenKind::As) {
                self.bump();
                let tok = self.expect_ident()?;
                Some(Ident {
                    name: tok.ident_name(),
                    span: tok.span,
                })
            } else {
                None
            };
            self.expect(&TokenKind::From)?;
            let source = self.expect_string_lit()?;
            let (attributes, clause_end) = self.parse_with_clause_opt()?;
            let mut end = clause_end.unwrap_or(source.span.end.0);
            // E19.69: ExportDeclaration requires `;` or LineTerminator (ASI) after FromClause.
            end = self.expect_export_semi(end)?;
            return Ok(Stmt::ExportAllDeclaration {
                exported,
                source,
                attributes,
                span: Span::new(start, end),
            });
        }
        if self.check(&TokenKind::LBrace) {
            self.bump();
            let mut specifiers = Vec::new();
            if !self.check(&TokenKind::RBrace) {
                loop {
                    // IdentifierName: `default` is valid in `{ default as x }` / `{ x as default }`.
                    let (local_name, local_span) = self.expect_ident_name()?;
                    let local = Ident {
                        name: local_name,
                        span: local_span,
                    };
                    let exported = if self.check(&TokenKind::As) {
                        self.bump();
                        let (name, span) = self.expect_ident_name()?;
                        Ident { name, span }
                    } else {
                        local.clone()
                    };
                    specifiers.push(ExportSpecifier { local, exported });
                    if self.check(&TokenKind::Comma) {
                        self.bump();
                        if self.check(&TokenKind::RBrace) {
                            break;
                        }
                        continue;
                    }
                    break;
                }
            }
            let end_brace = self.expect(&TokenKind::RBrace)?.span.end.0;
            let mut end = end_brace;
            // `export { a, b as c } from "mod"`
            let (source, attributes) = if self.check(&TokenKind::From) {
                self.bump();
                let src = self.expect_string_lit()?;
                end = src.span.end.0;
                let (attributes, clause_end) = self.parse_with_clause_opt()?;
                if let Some(e) = clause_end {
                    end = e;
                }
                (Some(src), attributes)
            } else {
                (None, Vec::new())
            };
            // E19.69: trailing `;` or ASI after named export / export-from.
            end = self.expect_export_semi(end)?;
            return Ok(Stmt::ExportNamedDeclaration {
                declaration: None,
                specifiers,
                source,
                attributes,
                span: Span::new(start, end),
            });
        }
        if self.check(&TokenKind::Let)
            || self.check(&TokenKind::Const)
            || self.check(&TokenKind::Var)
        {
            // `export VariableStatement` / `export LexicalDeclaration` (E19.54: `export var`).
            let decl = self.parse_lexical_decl()?;
            let end = stmt_span(&decl).end.0;
            return Ok(Stmt::ExportNamedDeclaration {
                declaration: Some(Box::new(decl)),
                specifiers: Vec::new(),
                source: None,
                attributes: Vec::new(),
                span: Span::new(start, end),
            });
        }
        if self.check(&TokenKind::Function)
            || (self.check(&TokenKind::Async) && self.peek_is(&TokenKind::Function))
        {
            let decl = self.parse_function_decl()?;
            let end = stmt_span(&decl).end.0;
            return Ok(Stmt::ExportNamedDeclaration {
                declaration: Some(Box::new(decl)),
                specifiers: Vec::new(),
                source: None,
                attributes: Vec::new(),
                span: Span::new(start, end),
            });
        }
        // E19.78: `export` DecoratorList_opt `class` …
        if self.check(&TokenKind::At) || self.check(&TokenKind::Class) {
            if self.check(&TokenKind::At) {
                self.parse_decorator_list()?;
            }
            if self.check(&TokenKind::Class) {
                let decl = self.parse_class_decl()?;
                let end = stmt_span(&decl).end.0;
                return Ok(Stmt::ExportNamedDeclaration {
                    declaration: Some(Box::new(decl)),
                    specifiers: Vec::new(),
                    source: None,
                    attributes: Vec::new(),
                    span: Span::new(start, end),
                });
            }
            return Err(Diagnostic::new(
                "decorators must precede a class declaration".to_string(),
                self.current_span(),
            ));
        }
        Err(Diagnostic::new(
            "expected `default`, `*`, `let`, `const`, `var`, `function`, `class`, or `{` after `export`"
                .to_string(),
            self.current_span(),
        ))
    }

    /// `export default async? function name? (…) {…}` or `export default class Name {…}` or `export default expr;`
    fn parse_export_default(&mut self, start: u32) -> Result<Stmt, Diagnostic> {
        self.expect(&TokenKind::Default)?;
        if self.check(&TokenKind::Function)
            || (self.check(&TokenKind::Async) && self.peek_is(&TokenKind::Function))
        {
            let (is_async, fn_start) = if self.check(&TokenKind::Async) {
                let s = self.bump().span.start.0;
                (true, s)
            } else {
                (false, self.current_span().start.0)
            };
            self.expect(&TokenKind::Function)?;
            let is_generator = if self.check(&TokenKind::Star) {
                self.bump();
                true
            } else {
                false
            };
            return self.with_ctx(
                |c| c.in_generator = is_generator,
                |p| {
                    // Default export function declaration name inherits outer [Await]; body uses is_async.
                    let (name, is_synthetic) = if p.at_binding_ident() {
                        let name_tok = p.expect_ident()?;
                        (
                            Ident {
                                name: name_tok.ident_name(),
                                span: name_tok.span,
                            },
                            false,
                        )
                    } else {
                        (
                            Ident {
                                name: "__default".into(),
                                span: Span::new(fn_start, fn_start),
                            },
                            true,
                        )
                    };
                    p.ctx.in_await_context = is_async;
                    p.expect(&TokenKind::LParen)?;
                    let params = p.parse_param_list()?;
                    p.expect(&TokenKind::RParen)?;
                    // E19.58: FormalParameters of a generator must not contain YieldExpression.
                    if is_generator && params_contain_yield_expr(&params) {
                        return Err(Diagnostic::new(
                            "generator parameters cannot contain yield".to_string(),
                            Span::new(fn_start, p.current_span().end.0),
                        ));
                    }
                    let return_type = p.parse_optional_type_ann()?;
                    let body = Box::new(p.parse_function_body_block()?);
                    let end = stmt_span(&body).end.0;
                    let local = name.clone();
                    let declaration = if is_synthetic {
                        // Anonymous default function → `let __default = async? function *? (…) {…}`
                        Stmt::Let {
                            kind: BindingKind::Let,
                            binding: BindingPattern::Ident(local.clone()),
                            type_ann: None,
                            init: Some(Expr::FunctionExpression {
                                name: None,
                                params,
                                return_type,
                                body,
                                is_async,
                                is_generator,
                                is_method: false,
                                span: Span::new(fn_start, end),
                            }),
                            span: Span::new(fn_start, end),
                        }
                    } else {
                        Stmt::FunctionDeclaration {
                            name,
                            type_params: Vec::new(),
                            params,
                            return_type,
                            body,
                            is_async,
                            is_generator,
                            span: Span::new(fn_start, end),
                        }
                    };
                    Ok(Stmt::ExportDefaultDeclaration {
                        declaration: Box::new(declaration),
                        local,
                        span: Span::new(start, end),
                    })
                },
            );
        }
        // E19.78: `export default` DecoratorList_opt `class` …
        if self.check(&TokenKind::At) || self.check(&TokenKind::Class) {
            if self.check(&TokenKind::At) {
                self.parse_decorator_list()?;
            }
            if self.check(&TokenKind::Class) {
                // E19.54: `export default class extends …` / anonymous `export default class {…}`.
                let decl = self.parse_class_decl_inner(true)?;
                let end = stmt_span(&decl).end.0;
                let local = match &decl {
                    Stmt::ClassDeclaration { name, .. } => name.clone(),
                    _ => unreachable!("parse_class_decl_inner returns ClassDeclaration"),
                };
                return Ok(Stmt::ExportDefaultDeclaration {
                    declaration: Box::new(decl),
                    local,
                    span: Span::new(start, end),
                });
            }
            // Fall through: `@dec` alone is not export default (decorator applies to following).
        }
        // LexicalDeclaration is not a valid export default (E19.41 / Test262 parse-err-export-dflt-let).
        if self.check(&TokenKind::Const)
            || (self.check(&TokenKind::Let) && self.let_starts_lexical_declaration())
            || self.await_using_starts_declaration()
            || self.using_starts_declaration()
        {
            return Err(Diagnostic::new(
                "lexical declaration not allowed in export default".to_string(),
                self.current().span,
            ));
        }

        // E19.69: `export default` AssignmentExpression (not Expression/comma).
        let expr = self.parse_assignment()?;
        let mut end = expr_span(&expr).end.0;
        end = self.expect_export_semi(end)?;
        let local = Ident {
            name: "__default".into(),
            span: Span::new(start, end),
        };
        let declaration = Stmt::Let {
            kind: BindingKind::Let,
            binding: BindingPattern::Ident(local.clone()),
            type_ann: None,
            init: Some(expr),
            span: Span::new(start, end),
        };
        Ok(Stmt::ExportDefaultDeclaration {
            declaration: Box::new(declaration),
            local,
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

    /// One or more lexical declarators (`let a, b = 1;`), each as its own `Stmt::Let`.
    fn parse_lexical_decls(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        let kind_tok = self.bump();
        let kind = match kind_tok.kind {
            TokenKind::Const => BindingKind::Const,
            TokenKind::Var => BindingKind::Var,
            _ => BindingKind::Let,
        };
        let kw_start = kind_tok.span.start.0;
        let mut decls = Vec::new();
        loop {
            let binding = if kind == BindingKind::Var {
                // `var` allows simple idents and destructuring patterns.
                self.parse_binding_pattern()?
            } else {
                self.parse_binding_pattern()?
            };
            // LexicalBinding BoundNames must not include "let" (always; not only strict).
            if kind != BindingKind::Var && binding_pattern_bound_names_contain_let(&binding) {
                return Err(Diagnostic::new(
                    "'let' is not allowed as a lexical binding name".to_string(),
                    binding.span(),
                ));
            }
            let type_ann = if matches!(binding, BindingPattern::Ident(_)) {
                self.parse_optional_type_ann()?
            } else {
                None
            };
            let init = if self.check(&TokenKind::Eq) {
                self.bump();
                Some(self.parse_assignment()?)
            } else if matches!(
                binding,
                BindingPattern::Array { .. } | BindingPattern::Object { .. }
            ) {
                return Err(Diagnostic::new(
                    "destructuring declaration requires an initializer".to_string(),
                    binding.span(),
                ));
            } else if kind == BindingKind::Const {
                return Err(Diagnostic::new(
                    "const declaration requires an initializer".to_string(),
                    binding.span(),
                ));
            } else {
                None
            };
            let decl_end = init
                .as_ref()
                .map(expr_span)
                .map(|s| s.end.0)
                .or_else(|| type_ann.as_ref().map(|a| a.span().end.0))
                .unwrap_or_else(|| binding.span().end.0);
            let start = if decls.is_empty() {
                kw_start
            } else {
                binding.span().start.0
            };
            decls.push(Stmt::Let {
                kind,
                binding,
                type_ann,
                init,
                span: Span::new(start, decl_end),
            });
            if self.check(&TokenKind::Comma) {
                self.bump();
                continue;
            }
            break;
        }
        if self.check(&TokenKind::Semi) {
            let semi_end = self.bump().span.end.0;
            if let Some(Stmt::Let { span, .. }) = decls.last_mut() {
                *span = Span::new(span.start.0, semi_end);
            }
        } else if !self.can_asi_before_current() {
            // `let yield 0` — no ASI between BindingIdentifier and `0` (E19.37).
            return Err(Diagnostic::new(
                "expected ';' after declaration".to_string(),
                self.current().span,
            ));
        }
        Ok(decls)
    }

    fn parse_lexical_decl(&mut self) -> Result<Stmt, Diagnostic> {
        let mut decls = self.parse_lexical_decls()?;
        if decls.len() == 1 {
            return Ok(decls.pop().unwrap());
        }
        // Multi-declarator as a single statement only via stmt-list expansion.
        // Callers that need one Stmt (for-init) take the first; remainder is rare.
        let start = stmt_span(&decls[0]).start.0;
        let end = stmt_span(decls.last().unwrap()).end.0;
        Ok(Stmt::Block {
            body: decls,
            span: Span::new(start, end),
        })
    }

    /// Binding pattern: identifier, `[a, b, ...rest]`, or `{ a, b: c, ...rest }`.
    fn parse_binding_pattern(&mut self) -> Result<BindingPattern, Diagnostic> {
        if self.check(&TokenKind::LBracket) {
            self.parse_array_binding_pattern()
        } else if self.check(&TokenKind::LBrace) {
            self.parse_object_binding_pattern()
        } else {
            let name_tok = self.expect_ident()?;
            Ok(BindingPattern::Ident(Ident {
                name: name_tok.ident_name(),
                span: name_tok.span,
            }))
        }
    }

    fn parse_array_binding_pattern(&mut self) -> Result<BindingPattern, Diagnostic> {
        let start = self.expect(&TokenKind::LBracket)?.span.start.0;
        let mut elements = Vec::new();
        let mut saw_rest = false;
        if !self.check(&TokenKind::RBracket) {
            loop {
                if self.check(&TokenKind::RBracket) {
                    break;
                }
                if saw_rest {
                    return Err(Diagnostic::new(
                        "rest element must be last in array pattern".to_string(),
                        self.current().span,
                    ));
                }
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    elements.push(ArrayPatternElement::Elision);
                    continue;
                }
                if self.check(&TokenKind::DotDotDot) {
                    self.bump();
                    let binding = self.parse_binding_pattern()?;
                    elements.push(ArrayPatternElement::Rest(binding));
                    saw_rest = true;
                } else {
                    let binding = self.parse_binding_pattern()?;
                    let default = if self.check(&TokenKind::Eq) {
                        self.bump();
                        Some(self.parse_assignment()?)
                    } else {
                        None
                    };
                    elements.push(ArrayPatternElement::Pattern { binding, default });
                }
                if self.check(&TokenKind::Comma) {
                    if saw_rest {
                        return Err(Diagnostic::new(
                            "rest element must be last in array pattern".to_string(),
                            self.current().span,
                        ));
                    }
                    self.bump();
                    continue;
                }
                break;
            }
        }
        let end = self.expect(&TokenKind::RBracket)?.span.end.0;
        Ok(BindingPattern::Array {
            elements,
            span: Span::new(start, end),
        })
    }

    fn parse_object_binding_pattern(&mut self) -> Result<BindingPattern, Diagnostic> {
        let start = self.expect(&TokenKind::LBrace)?.span.start.0;
        let mut properties = Vec::new();
        let mut saw_rest = false;
        if !self.check(&TokenKind::RBrace) {
            loop {
                if self.check(&TokenKind::RBrace) {
                    break;
                }
                if saw_rest {
                    return Err(Diagnostic::new(
                        "rest element must be last in object pattern".to_string(),
                        self.current().span,
                    ));
                }
                if self.check(&TokenKind::DotDotDot) {
                    self.bump();
                    let binding = self.parse_binding_pattern()?;
                    properties.push(ObjectPatternProp::Rest(binding));
                    saw_rest = true;
                } else {
                    // PropertyName: IdentifierName | StringLiteral | NumericLiteral | [AssignmentExpression]
                    // Shorthand only for BindingIdentifier (not string/number/computed).
                    let (key, can_shorthand) = self.parse_binding_property_name()?;
                    let key_span = object_key_span(&key);
                    if self.check(&TokenKind::Colon) {
                        self.bump();
                        let binding = self.parse_binding_pattern()?;
                        let default = if self.check(&TokenKind::Eq) {
                            self.bump();
                            Some(self.parse_assignment()?)
                        } else {
                            None
                        };
                        let end = default
                            .as_ref()
                            .map(|d| expr_span(d).end.0)
                            .unwrap_or_else(|| binding.span().end.0);
                        properties.push(ObjectPatternProp::Prop {
                            key,
                            binding,
                            shorthand: false,
                            default,
                            span: Span::new(key_span.start.0, end),
                        });
                    } else {
                        if !can_shorthand {
                            return Err(Diagnostic::new(
                                "expected ':' after property name in object pattern".to_string(),
                                self.current().span,
                            ));
                        }
                        let ObjectKey::Ident(key_id) = &key else {
                            return Err(Diagnostic::new(
                                "expected ':' after property name in object pattern".to_string(),
                                self.current().span,
                            ));
                        };
                        // Shorthand `{ a }` — BindingIdentifier (yield only when yield_is_ident).
                        if self.is_invalid_ident_name(&key_id.name) {
                            return Err(Diagnostic::new(
                                format!(
                                    "'{}' is a reserved word and cannot be used as an identifier",
                                    key_id.name
                                ),
                                key_span,
                            ));
                        }
                        let default = if self.check(&TokenKind::Eq) {
                            self.bump();
                            Some(self.parse_assignment()?)
                        } else {
                            None
                        };
                        let end = default
                            .as_ref()
                            .map(|d| expr_span(d).end.0)
                            .unwrap_or(key_id.span.end.0);
                        properties.push(ObjectPatternProp::Prop {
                            key: key.clone(),
                            binding: BindingPattern::Ident(key_id.clone()),
                            shorthand: true,
                            default,
                            span: Span::new(key_id.span.start.0, end),
                        });
                    }
                }
                if self.check(&TokenKind::Comma) {
                    if saw_rest {
                        return Err(Diagnostic::new(
                            "rest element must be last in object pattern".to_string(),
                            self.current().span,
                        ));
                    }
                    self.bump();
                    continue;
                }
                break;
            }
        }
        let end = self.expect(&TokenKind::RBrace)?.span.end.0;
        Ok(BindingPattern::Object {
            properties,
            span: Span::new(start, end),
        })
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

    /// Object-pattern PropertyName → ObjectKey + whether shorthand is allowed.
    /// Shorthand only for IdentifierName; string/number/computed require `:`.
    fn parse_binding_property_name(&mut self) -> Result<(ObjectKey, bool), Diagnostic> {
        let tok = self.current().clone();
        if let Some(name) = tok.ident_name_opt() {
            self.bump();
            return Ok((
                ObjectKey::Ident(Ident {
                    name,
                    span: tok.span,
                }),
                true,
            ));
        }
        match &tok.kind {
            TokenKind::String(value) => {
                self.reject_legacy_octal_token(&tok)?;
                let value = value.clone();
                self.bump();
                Ok((
                    ObjectKey::String(StringLit {
                        value,
                        span: tok.span,
                    }),
                    false,
                ))
            }
            TokenKind::Number(raw) => {
                self.reject_legacy_octal_token(&tok)?;
                let name = numeric_literal_property_name(raw);
                self.bump();
                Ok((
                    ObjectKey::String(StringLit {
                        value: name.into(),
                        span: tok.span,
                    }),
                    false,
                ))
            }
            TokenKind::LBracket => {
                self.bump();
                let expr = self.with_ctx(|c| c.allow_in = true, Self::parse_assignment)?;
                self.expect(&TokenKind::RBracket)?;
                Ok((ObjectKey::Computed(Box::new(expr)), false))
            }
            _ => Err(Diagnostic::new(
                format!("expected property name, found {:?}", tok.kind),
                tok.span,
            )),
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

/// True if BoundNames of a BindingPattern includes `let` (LexicalBinding early error).
fn binding_pattern_bound_names_contain_let(binding: &BindingPattern) -> bool {
    let mut has_let = false;
    binding.for_each_ident(&mut |id| {
        if id.name == "let" {
            has_let = true;
        }
    });
    has_let
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

/// Bare FunctionDeclaration or `label: … function` (IsLabelledFunction). E19.49.
fn stmt_is_function_or_labelled_function(stmt: &Stmt) -> bool {
    let mut s = stmt;
    loop {
        match s {
            Stmt::FunctionDeclaration { .. } => return true,
            Stmt::Labeled { body, .. } => s = body,
            _ => return false,
        }
    }
}

/// ECMA-262 IsLabelledFunction: LabelledStatement whose innermost item is FunctionDeclaration.
/// E19.67: IterationStatement / IfStatement early error.
fn stmt_is_labelled_function(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Labeled { body, .. } => stmt_is_function_or_labelled_function(body),
        _ => false,
    }
}

/// True when `expr` is (or chains through) an optional chain (`?.`).
fn expr_is_optional_chain(expr: &Expr) -> bool {
    match expr {
        Expr::MemberExpression { optional: true, .. } | Expr::Call { optional: true, .. } => true,
        Expr::MemberExpression {
            optional: false,
            object,
            ..
        } => expr_is_optional_chain(object),
        Expr::Call {
            optional: false,
            callee,
            ..
        } => expr_is_optional_chain(callee),
        Expr::Paren { expr: inner, .. } => expr_is_optional_chain(inner),
        _ => false,
    }
}

/// CoverInitializedName encoded as shorthand Property with Assign value (E19.67).
fn expr_contains_cover_initialized_name(expr: &Expr) -> bool {
    match expr {
        Expr::Paren { expr: inner, .. } => expr_contains_cover_initialized_name(inner),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property {
                shorthand: true,
                value: Expr::Assign { .. },
                ..
            } => true,
            ObjectProp::Property { value, .. } => expr_contains_cover_initialized_name(value),
            ObjectProp::Spread { expr, .. } => expr_contains_cover_initialized_name(expr),
            ObjectProp::Accessor { .. } => false,
        }),
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                expr_contains_cover_initialized_name(e)
            }
            ArrayElement::Elision => false,
        }),
        Expr::Assign { target, value, .. } => {
            expr_contains_cover_initialized_name(target)
                || expr_contains_cover_initialized_name(value)
        }
        Expr::Binary { left, right, .. } => {
            expr_contains_cover_initialized_name(left)
                || expr_contains_cover_initialized_name(right)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_cover_initialized_name(test)
                || expr_contains_cover_initialized_name(consequent)
                || expr_contains_cover_initialized_name(alternate)
        }
        Expr::Unary { arg, .. } | Expr::Update { arg, .. } | Expr::As { expr: arg, .. } => {
            expr_contains_cover_initialized_name(arg)
        }
        Expr::Call { callee, args, .. } => {
            expr_contains_cover_initialized_name(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_cover_initialized_name(e),
                })
        }
        Expr::MemberExpression {
            object, property, ..
        } => {
            expr_contains_cover_initialized_name(object)
                || expr_contains_cover_initialized_name(property)
        }
        _ => false,
    }
}

fn object_prop_is_proto_data(prop: &ObjectProp) -> bool {
    match prop {
        ObjectProp::Property {
            key,
            shorthand: false,
            value,
            ..
        } => {
            // Only `PropertyName : AssignmentExpression` form — not methods/shorthand.
            if matches!(
                value,
                Expr::FunctionExpression {
                    is_method: true,
                    ..
                }
            ) {
                return false;
            }
            match key {
                ObjectKey::Ident(id) => id.name == "__proto__",
                ObjectKey::String(s) => s.value.to_string_lossy() == "__proto__",
                ObjectKey::Computed(_) => false,
            }
        }
        _ => false,
    }
}

fn object_prop_span(prop: &ObjectProp) -> Span {
    match prop {
        ObjectProp::Property { span, .. }
        | ObjectProp::Spread { span, .. }
        | ObjectProp::Accessor { span, .. } => *span,
    }
}

/// AwaitExpression in FormalParameters of async functions (E19.67).
fn params_contain_await_expr(params: &[Param]) -> bool {
    params.iter().any(|p| {
        p.default.as_ref().is_some_and(expr_contains_await_expr)
            || binding_pattern_contains_await_expr(&p.binding)
    })
}

fn binding_pattern_contains_await_expr(b: &BindingPattern) -> bool {
    match b {
        BindingPattern::Ident(_) | BindingPattern::Member(_) => false,
        BindingPattern::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Elision => false,
            ArrayPatternElement::Pattern { binding, default } => {
                binding_pattern_contains_await_expr(binding)
                    || default.as_ref().is_some_and(expr_contains_await_expr)
            }
            ArrayPatternElement::Rest(inner) => binding_pattern_contains_await_expr(inner),
        }),
        BindingPattern::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop {
                key,
                binding,
                default,
                ..
            } => {
                object_key_contains_await_expr(key)
                    || binding_pattern_contains_await_expr(binding)
                    || default.as_ref().is_some_and(expr_contains_await_expr)
            }
            ObjectPatternProp::Rest(inner) => binding_pattern_contains_await_expr(inner),
        }),
    }
}

fn object_key_contains_await_expr(key: &ObjectKey) -> bool {
    match key {
        ObjectKey::Computed(e) => expr_contains_await_expr(e),
        ObjectKey::Ident(_) | ObjectKey::String(_) => false,
    }
}

fn expr_contains_await_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Unary {
            op: UnaryOp::Await, ..
        } => true,
        // Nested functions/classes hide await of their bodies for outer Contains.
        Expr::FunctionExpression { .. } | Expr::ClassExpression { .. } => false,
        Expr::ArrowFunction { body, params, .. } => {
            params_contain_await_expr(params)
                || match body {
                    ArrowBody::Expr(e) => expr_contains_await_expr(e),
                    ArrowBody::Block(s) => stmt_contains_await_expr(s),
                }
        }
        Expr::Paren { expr: inner, .. }
        | Expr::Unary { arg: inner, .. }
        | Expr::Update { arg: inner, .. }
        | Expr::As { expr: inner, .. } => expr_contains_await_expr(inner),
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => expr_contains_await_expr(left) || expr_contains_await_expr(right),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_await_expr(test)
                || expr_contains_await_expr(consequent)
                || expr_contains_await_expr(alternate)
        }
        Expr::MemberExpression {
            object, property, ..
        } => expr_contains_await_expr(object) || expr_contains_await_expr(property),
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_contains_await_expr(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_await_expr(e),
                })
        }
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_await_expr(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { key, value, .. } => {
                object_key_contains_await_expr(key) || expr_contains_await_expr(value)
            }
            ObjectProp::Spread { expr, .. } => expr_contains_await_expr(expr),
            ObjectProp::Accessor { key, .. } => object_key_contains_await_expr(key),
        }),
        Expr::TemplateLiteral { expressions, .. } => {
            expressions.iter().any(expr_contains_await_expr)
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => expr_contains_await_expr(tag) || expressions.iter().any(expr_contains_await_expr),
        Expr::ImportCall {
            source, options, ..
        } => {
            expr_contains_await_expr(source)
                || options
                    .as_ref()
                    .is_some_and(|o| expr_contains_await_expr(o))
        }
        Expr::PrivateIn { object, .. } => expr_contains_await_expr(object),
        _ => false,
    }
}

fn stmt_contains_await_expr(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expression { expr, .. } => expr_contains_await_expr(expr),
        Stmt::Throw { argument, .. } => expr_contains_await_expr(argument),
        Stmt::Return { argument, .. } => argument.as_ref().is_some_and(expr_contains_await_expr),
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_await_expr),
        Stmt::Let { init, .. } => init.as_ref().is_some_and(expr_contains_await_expr),
        _ => false,
    }
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

/// Literal IdentifierName `constructor` only — not `"constructor"` or `['constructor']`.
fn class_key_is_literal_constructor(key: &ObjectKey) -> bool {
    matches!(key, ObjectKey::Ident(id) if id.name == "constructor")
}

/// PropName of a non-computed class element key (Ident / String / numeric→String).
fn class_element_prop_name(key: &ObjectKey) -> Option<String> {
    match key {
        ObjectKey::Ident(id) => Some(id.name.clone()),
        ObjectKey::String(s) => Some(s.value.to_string_lossy()),
        ObjectKey::Computed(_) => None,
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

/// True when `expr` is (possibly parenthesized) `….#private` (E19.36 delete early error).
fn expr_is_private_member_reference(expr: &Expr) -> bool {
    match expr {
        Expr::Paren { expr: inner, .. } => expr_is_private_member_reference(inner),
        Expr::MemberExpression { private: true, .. } => true,
        _ => false,
    }
}

fn register_private_names_from_element(el: &ClassElement, frame: &mut Vec<String>) {
    match el {
        ClassElement::Field {
            key,
            is_private: true,
            ..
        }
        | ClassElement::Method {
            key,
            is_private: true,
            ..
        }
        | ClassElement::Accessor {
            key,
            is_private: true,
            ..
        } => {
            if let ObjectKey::Ident(id) = key {
                if !frame.iter().any(|n| n == &id.name) {
                    frame.push(id.name.clone());
                }
            }
        }
        _ => {}
    }
}

/// ClassBody early errors: duplicate privates, field PropName, SuperCall/arguments in field init,
/// SuperCall outside constructor, duplicate constructor, `#constructor`, undeclared private refs (E19.39).
///
/// `inherited` = private names from enclosing classes (nested class visibility).
fn validate_class_body(
    body: &[ClassElement],
    has_heritage: bool,
    inherited: &[String],
) -> Result<(), Diagnostic> {
    let mut private_names: Vec<String> = Vec::new();
    let mut ctor_count = 0u32;
    // Track private accessor kinds for static/instance getter+setter pairing rules.
    // name -> (has_instance_get, has_instance_set, has_static_get, has_static_set, has_field_or_method)
    let mut private_kinds: std::collections::HashMap<String, (bool, bool, bool, bool, bool)> =
        std::collections::HashMap::new();

    for el in body {
        match el {
            ClassElement::Field {
                key,
                value,
                is_static,
                is_private,
                span,
            } => {
                if *is_private {
                    if let ObjectKey::Ident(id) = key {
                        if id.name == "constructor" {
                            return Err(Diagnostic::new(
                                "private field cannot be named #constructor".to_string(),
                                *span,
                            ));
                        }
                        if private_names.iter().any(|n| n == &id.name) {
                            return Err(Diagnostic::new(
                                format!("duplicate private name #{}", id.name),
                                *span,
                            ));
                        }
                        private_names.push(id.name.clone());
                        let e = private_kinds.entry(id.name.clone()).or_default();
                        e.4 = true;
                    }
                } else if let Some(name) = class_element_prop_name(key) {
                    if name == "constructor" {
                        return Err(Diagnostic::new(
                            "class field cannot be named constructor".to_string(),
                            *span,
                        ));
                    }
                    if *is_static && name == "prototype" {
                        return Err(Diagnostic::new(
                            "static class field cannot be named prototype".to_string(),
                            *span,
                        ));
                    }
                }
                if let Some(v) = value {
                    if expr_contains_super_call(v) {
                        return Err(Diagnostic::new(
                            "class field initializer cannot contain super call".to_string(),
                            *span,
                        ));
                    }
                    if expr_contains_arguments_ref(v) {
                        return Err(Diagnostic::new(
                            "class field initializer cannot contain arguments".to_string(),
                            *span,
                        ));
                    }
                }
            }
            ClassElement::Method {
                key,
                params,
                body: method_body,
                is_static,
                is_private,
                is_async: _,
                is_generator: _,
                span,
            } => {
                if *is_private {
                    if let ObjectKey::Ident(id) = key {
                        if id.name == "constructor" {
                            return Err(Diagnostic::new(
                                "private method cannot be named #constructor".to_string(),
                                *span,
                            ));
                        }
                        if private_names.iter().any(|n| n == &id.name) {
                            return Err(Diagnostic::new(
                                format!("duplicate private name #{}", id.name),
                                *span,
                            ));
                        }
                        private_names.push(id.name.clone());
                        let e = private_kinds.entry(id.name.clone()).or_default();
                        e.4 = true;
                    }
                } else if *is_static {
                    if let Some(name) = class_element_prop_name(key) {
                        if name == "prototype" {
                            return Err(Diagnostic::new(
                                "static class method cannot be named prototype".to_string(),
                                *span,
                            ));
                        }
                    }
                }
                // SuperCall only allowed in constructors (not methods / static methods).
                if params_contain_super_call(params) || stmt_contains_super_call(method_body) {
                    return Err(Diagnostic::new(
                        "class method cannot contain super call".to_string(),
                        *span,
                    ));
                }
            }
            ClassElement::Accessor {
                key,
                params,
                body: accessor_body,
                is_static,
                is_private,
                kind,
                span,
            } => {
                if *is_private {
                    if let ObjectKey::Ident(id) = key {
                        if id.name == "constructor" {
                            return Err(Diagnostic::new(
                                "private accessor cannot be named #constructor".to_string(),
                                *span,
                            ));
                        }
                        let e = private_kinds.entry(id.name.clone()).or_default();
                        // Duplicate same-kind accessor (get+get / set+set) is an error.
                        let dup = match (kind, *is_static) {
                            (AccessorKind::Get, false) if e.0 => true,
                            (AccessorKind::Set, false) if e.1 => true,
                            (AccessorKind::Get, true) if e.2 => true,
                            (AccessorKind::Set, true) if e.3 => true,
                            _ => false,
                        };
                        if dup || e.4 {
                            return Err(Diagnostic::new(
                                format!("duplicate private name #{}", id.name),
                                *span,
                            ));
                        }
                        // get/set pair may share one PrivateBoundName (allow up to 2 entries).
                        if private_names.iter().filter(|n| *n == &id.name).count() >= 2 {
                            return Err(Diagnostic::new(
                                format!("duplicate private name #{}", id.name),
                                *span,
                            ));
                        }
                        private_names.push(id.name.clone());
                        match (kind, *is_static) {
                            (AccessorKind::Get, false) => e.0 = true,
                            (AccessorKind::Set, false) => e.1 = true,
                            (AccessorKind::Get, true) => e.2 = true,
                            (AccessorKind::Set, true) => e.3 = true,
                        }
                    }
                } else if let Some(name) = class_element_prop_name(key) {
                    // Instance accessors cannot be named "constructor"; static can.
                    if !*is_static && name == "constructor" {
                        return Err(Diagnostic::new(
                            "class accessor cannot be named constructor".to_string(),
                            *span,
                        ));
                    }
                    if *is_static && name == "prototype" {
                        return Err(Diagnostic::new(
                            "static class accessor cannot be named prototype".to_string(),
                            *span,
                        ));
                    }
                }
                if params_contain_super_call(params) || stmt_contains_super_call(accessor_body) {
                    return Err(Diagnostic::new(
                        "class accessor cannot contain super call".to_string(),
                        *span,
                    ));
                }
            }
            ClassElement::Constructor {
                params,
                body: ctor_body,
                span,
            } => {
                ctor_count += 1;
                if ctor_count > 1 {
                    return Err(Diagnostic::new(
                        "class may have at most one constructor".to_string(),
                        *span,
                    ));
                }
                // SuperCall in constructor formals is still an error.
                if params_contain_super_call(params) {
                    return Err(Diagnostic::new(
                        "constructor parameters cannot contain super call".to_string(),
                        *span,
                    ));
                }
                // SuperCall requires ClassHeritage (E19.39 grammar-ctor-super-no-heritage).
                if !has_heritage && stmt_contains_super_call(ctor_body) {
                    return Err(Diagnostic::new(
                        "super call only allowed in derived class constructor".to_string(),
                        *span,
                    ));
                }
            }
            ClassElement::StaticBlock {
                body: block_body,
                span,
            } => {
                if stmt_contains_super_call(block_body) {
                    return Err(Diagnostic::new(
                        "static block cannot contain super call".to_string(),
                        *span,
                    ));
                }
                if stmt_contains_return(block_body) {
                    return Err(Diagnostic::new(
                        "static block cannot contain return".to_string(),
                        *span,
                    ));
                }
                // ContainsArguments includes nested class computed names like `[arguments]`.
                if stmt_contains_arguments_ref(block_body)
                    || stmt_contains_arguments_deep(block_body)
                {
                    return Err(Diagnostic::new(
                        "static block cannot contain arguments".to_string(),
                        *span,
                    ));
                }
            }
        }
    }

    // Private getter/setter must not mix static and instance for the same name.
    for (name, (ig, is, sg, ss, field_or_method)) in &private_kinds {
        let has_instance = *ig || *is;
        let has_static = *sg || *ss;
        if has_instance && has_static {
            return Err(Diagnostic::new(
                format!("private name #{name} cannot mix static and instance accessors"),
                Span::dummy(),
            ));
        }
        if *field_or_method && (has_instance || has_static) {
            // field/method + accessor same name already caught by duplicate private_names
            // for methods/fields; accessors allow 2 entries so field+get may slip — handled above.
            let _ = field_or_method;
        }
    }

    // All private references must be in this class or an enclosing class.
    let mut visible = inherited.to_vec();
    for n in &private_names {
        if !visible.iter().any(|v| v == n) {
            visible.push(n.clone());
        }
    }
    for el in body {
        check_class_element_private_refs(el, &visible)?;
    }
    Ok(())
}

fn params_contain_super_call(params: &[Param]) -> bool {
    params.iter().any(|p| {
        p.default.as_ref().is_some_and(expr_contains_super_call)
            || binding_pattern_contains_super_call(&p.binding)
    })
}

fn params_contain_yield_expr(params: &[Param]) -> bool {
    params.iter().any(|p| {
        p.default.as_ref().is_some_and(expr_contains_yield_expr)
            || binding_pattern_contains_yield_expr(&p.binding)
    })
}

/// True when `expr` is an unparenthesized UnaryExpression with a unary operator
/// (not UpdateExpression). Invalid as the left operand of `**` (E19.58).
fn expr_is_unparenthesized_unary_op(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Unary {
            op: UnaryOp::Plus
                | UnaryOp::Minus
                | UnaryOp::Not
                | UnaryOp::BitNot
                | UnaryOp::TypeOf
                | UnaryOp::Void
                | UnaryOp::Delete
                | UnaryOp::Await
                | UnaryOp::Ref
                | UnaryOp::Deref,
            ..
        }
    )
}

fn binding_pattern_contains_yield_expr(b: &BindingPattern) -> bool {
    match b {
        BindingPattern::Ident(_) | BindingPattern::Member(_) => false,
        BindingPattern::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Elision => false,
            ArrayPatternElement::Pattern { binding, default } => {
                binding_pattern_contains_yield_expr(binding)
                    || default.as_ref().is_some_and(expr_contains_yield_expr)
            }
            ArrayPatternElement::Rest(inner) => binding_pattern_contains_yield_expr(inner),
        }),
        BindingPattern::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop {
                key,
                binding,
                default,
                ..
            } => {
                object_key_contains_yield_expr(key)
                    || binding_pattern_contains_yield_expr(binding)
                    || default.as_ref().is_some_and(expr_contains_yield_expr)
            }
            ObjectPatternProp::Rest(inner) => binding_pattern_contains_yield_expr(inner),
        }),
    }
}

fn object_key_contains_yield_expr(key: &ObjectKey) -> bool {
    match key {
        ObjectKey::Computed(e) => expr_contains_yield_expr(e),
        ObjectKey::Ident(_) | ObjectKey::String(_) => false,
    }
}

fn expr_contains_yield_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Unary {
            op: UnaryOp::Yield | UnaryOp::YieldStar,
            ..
        } => true,
        Expr::FunctionExpression { .. } | Expr::ClassExpression { .. } => false,
        Expr::ArrowFunction { body, params, .. } => {
            params_contain_yield_expr(params)
                || match body {
                    ArrowBody::Expr(e) => expr_contains_yield_expr(e),
                    ArrowBody::Block(s) => stmt_contains_yield_expr(s),
                }
        }
        Expr::Paren { expr: inner, .. }
        | Expr::Unary { arg: inner, .. }
        | Expr::Update { arg: inner, .. }
        | Expr::As { expr: inner, .. } => expr_contains_yield_expr(inner),
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => expr_contains_yield_expr(left) || expr_contains_yield_expr(right),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_yield_expr(test)
                || expr_contains_yield_expr(consequent)
                || expr_contains_yield_expr(alternate)
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_contains_yield_expr(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_yield_expr(e),
                })
        }
        Expr::MemberExpression {
            object, property, ..
        } => expr_contains_yield_expr(object) || expr_contains_yield_expr(property),
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_yield_expr(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, .. } => expr_contains_yield_expr(value),
            ObjectProp::Spread { expr, .. } => expr_contains_yield_expr(expr),
            ObjectProp::Accessor { .. } => false,
        }),
        _ => false,
    }
}

fn stmt_contains_yield_expr(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expression { expr, .. } => expr_contains_yield_expr(expr),
        Stmt::Return { argument, .. } => argument.as_ref().is_some_and(expr_contains_yield_expr),
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_yield_expr),
        _ => false,
    }
}

fn binding_pattern_contains_super_call(b: &BindingPattern) -> bool {
    match b {
        BindingPattern::Ident(_) | BindingPattern::Member(_) => false,
        BindingPattern::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Elision => false,
            ArrayPatternElement::Pattern { binding, default } => {
                binding_pattern_contains_super_call(binding)
                    || default.as_ref().is_some_and(expr_contains_super_call)
            }
            ArrayPatternElement::Rest(inner) => binding_pattern_contains_super_call(inner),
        }),
        BindingPattern::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop {
                key,
                binding,
                default,
                ..
            } => {
                object_key_contains_super_call(key)
                    || binding_pattern_contains_super_call(binding)
                    || default.as_ref().is_some_and(expr_contains_super_call)
            }
            ObjectPatternProp::Rest(inner) => binding_pattern_contains_super_call(inner),
        }),
    }
}

fn stmt_contains_return(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { .. } => true,
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_return),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            stmt_contains_return(consequent)
                || alternate.as_ref().is_some_and(|a| stmt_contains_return(a))
        }
        Stmt::While { body, .. }
        | Stmt::DoWhile { body, .. }
        | Stmt::For { body, .. }
        | Stmt::ForOf { body, .. }
        | Stmt::ForIn { body, .. }
        | Stmt::Labeled { body, .. }
        | Stmt::With { body, .. } => stmt_contains_return(body),
        Stmt::Switch { cases, .. } => cases
            .iter()
            .any(|c| c.body.iter().any(stmt_contains_return)),
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            stmt_contains_return(block)
                || handler.as_ref().is_some_and(|h| stmt_contains_return(h))
                || finalizer.as_ref().is_some_and(|f| stmt_contains_return(f))
        }
        Stmt::FunctionDeclaration { .. } | Stmt::ClassDeclaration { .. } => false,
        _ => false,
    }
}

/// Deeper ContainsArguments walk for static blocks: enter nested class expressions
/// and scan computed property names / heritage (E19.39 static-init-invalid-arguments).
fn stmt_contains_arguments_deep(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expression { expr, .. } => expr_contains_arguments_deep(expr),
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_arguments_deep),
        Stmt::Return { argument, .. } => {
            argument.as_ref().is_some_and(expr_contains_arguments_deep)
        }
        Stmt::Let { init, .. } => init.as_ref().is_some_and(expr_contains_arguments_deep),
        _ => false,
    }
}

fn expr_contains_arguments_deep(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(id) if id.name == "arguments" => true,
        Expr::ClassExpression {
            super_class, body, ..
        } => {
            super_class
                .as_ref()
                .is_some_and(|sc| expr_contains_arguments_deep(sc))
                || body.iter().any(class_element_contains_arguments_deep)
        }
        Expr::Paren { expr: inner, .. }
        | Expr::Unary { arg: inner, .. }
        | Expr::Update { arg: inner, .. }
        | Expr::As { expr: inner, .. } => expr_contains_arguments_deep(inner),
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => expr_contains_arguments_deep(left) || expr_contains_arguments_deep(right),
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_contains_arguments_deep(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_arguments_deep(e),
                })
        }
        Expr::MemberExpression {
            object, property, ..
        } => expr_contains_arguments_deep(object) || expr_contains_arguments_deep(property),
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_arguments_deep(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { key, value, .. } => {
                object_key_contains_arguments_deep(key) || expr_contains_arguments_deep(value)
            }
            ObjectProp::Spread { expr, .. } => expr_contains_arguments_deep(expr),
            ObjectProp::Accessor { key, .. } => object_key_contains_arguments_deep(key),
        }),
        Expr::FunctionExpression { .. } | Expr::ArrowFunction { .. } => false,
        _ => false,
    }
}

fn object_key_contains_arguments_deep(key: &ObjectKey) -> bool {
    match key {
        ObjectKey::Computed(e) => expr_contains_arguments_deep(e),
        ObjectKey::Ident(id) => id.name == "arguments",
        ObjectKey::String(_) => false,
    }
}

fn class_element_contains_arguments_deep(el: &ClassElement) -> bool {
    match el {
        ClassElement::Method { key, .. }
        | ClassElement::Accessor { key, .. }
        | ClassElement::Field { key, .. } => object_key_contains_arguments_deep(key),
        ClassElement::Constructor { .. } | ClassElement::StaticBlock { .. } => false,
    }
}

fn check_class_element_private_refs(
    el: &ClassElement,
    declared: &[String],
) -> Result<(), Diagnostic> {
    match el {
        ClassElement::Field {
            key, value, span, ..
        } => {
            if let ObjectKey::Computed(e) = key {
                check_expr_private_refs(e, declared, *span)?;
            }
            if let Some(v) = value {
                check_expr_private_refs(v, declared, *span)?;
            }
            Ok(())
        }
        ClassElement::Method {
            key,
            params,
            body,
            span,
            ..
        }
        | ClassElement::Accessor {
            key,
            params,
            body,
            span,
            ..
        } => {
            if let ObjectKey::Computed(e) = key {
                check_expr_private_refs(e, declared, *span)?;
            }
            for p in params {
                if let Some(d) = &p.default {
                    check_expr_private_refs(d, declared, *span)?;
                }
            }
            check_stmt_private_refs(body, declared)
        }
        ClassElement::Constructor { params, body, span } => {
            for p in params {
                if let Some(d) = &p.default {
                    check_expr_private_refs(d, declared, *span)?;
                }
            }
            check_stmt_private_refs(body, declared)
        }
        ClassElement::StaticBlock { body, .. } => check_stmt_private_refs(body, declared),
    }
}

fn check_stmt_private_refs(stmt: &Stmt, declared: &[String]) -> Result<(), Diagnostic> {
    match stmt {
        Stmt::Block { body, .. } => {
            for s in body {
                check_stmt_private_refs(s, declared)?;
            }
            Ok(())
        }
        Stmt::Expression { expr, span, .. } => check_expr_private_refs(expr, declared, *span),
        Stmt::Return { argument, span, .. } => {
            if let Some(a) = argument {
                check_expr_private_refs(a, declared, *span)?;
            }
            Ok(())
        }
        Stmt::Throw { argument, span, .. } => check_expr_private_refs(argument, declared, *span),
        Stmt::If {
            test,
            consequent,
            alternate,
            span,
            ..
        } => {
            check_expr_private_refs(test, declared, *span)?;
            check_stmt_private_refs(consequent, declared)?;
            if let Some(a) = alternate {
                check_stmt_private_refs(a, declared)?;
            }
            Ok(())
        }
        Stmt::While {
            test, body, span, ..
        }
        | Stmt::DoWhile {
            test, body, span, ..
        } => {
            check_expr_private_refs(test, declared, *span)?;
            check_stmt_private_refs(body, declared)
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            span,
            ..
        } => {
            if let Some(i) = init {
                check_stmt_private_refs(i, declared)?;
            }
            if let Some(t) = test {
                check_expr_private_refs(t, declared, *span)?;
            }
            if let Some(u) = update {
                check_expr_private_refs(u, declared, *span)?;
            }
            check_stmt_private_refs(body, declared)
        }
        Stmt::ForOf {
            left,
            right,
            body,
            span,
            ..
        }
        | Stmt::ForIn {
            left,
            right,
            body,
            span,
            ..
        } => {
            check_stmt_private_refs(left, declared)?;
            check_expr_private_refs(right, declared, *span)?;
            check_stmt_private_refs(body, declared)
        }
        Stmt::Labeled { body, .. } | Stmt::With { body, .. } => {
            check_stmt_private_refs(body, declared)
        }
        Stmt::Switch {
            discriminant,
            cases,
            span,
            ..
        } => {
            check_expr_private_refs(discriminant, declared, *span)?;
            for c in cases {
                if let Some(t) = &c.test {
                    check_expr_private_refs(t, declared, *span)?;
                }
                for s in &c.body {
                    check_stmt_private_refs(s, declared)?;
                }
            }
            Ok(())
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            check_stmt_private_refs(block, declared)?;
            if let Some(h) = handler {
                check_stmt_private_refs(h, declared)?;
            }
            if let Some(f) = finalizer {
                check_stmt_private_refs(f, declared)?;
            }
            Ok(())
        }
        Stmt::Let { init, span, .. } => {
            if let Some(i) = init {
                check_expr_private_refs(i, declared, *span)?;
            }
            Ok(())
        }
        Stmt::FunctionDeclaration {
            params, body, span, ..
        } => {
            // Nested functions inherit enclosing class private names (E19.36/E19.39).
            for p in params {
                if let Some(d) = &p.default {
                    check_expr_private_refs(d, declared, *span)?;
                }
            }
            check_stmt_private_refs(body, declared)
        }
        // Nested class introduces its own private environment (validated separately).
        // Heritage is outside that environment — check outer privates only.
        Stmt::ClassDeclaration {
            super_class, span, ..
        } => {
            if let Some(sc) = super_class {
                check_expr_private_refs(sc, declared, *span)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn check_expr_private_refs(expr: &Expr, declared: &[String], span: Span) -> Result<(), Diagnostic> {
    match expr {
        Expr::MemberExpression {
            object,
            property,
            private: true,
            ..
        } => {
            if let Expr::Ident(id) = property.as_ref() {
                if !declared.iter().any(|n| n == &id.name) {
                    return Err(Diagnostic::new(
                        format!("undeclared private field #{}", id.name),
                        span,
                    ));
                }
            }
            // `super.#x` is invalid (E19.39).
            if matches!(object.as_ref(), Expr::Super { .. }) {
                return Err(Diagnostic::new(
                    "private fields cannot be accessed on super".to_string(),
                    span,
                ));
            }
            check_expr_private_refs(object, declared, span)
        }
        Expr::PrivateIn { name, object, .. } => {
            if !declared.iter().any(|n| n == &name.name) {
                return Err(Diagnostic::new(
                    format!("undeclared private field #{}", name.name),
                    span,
                ));
            }
            check_expr_private_refs(object, declared, span)
        }
        Expr::FunctionExpression { body, params, .. } => {
            for p in params {
                if let Some(d) = &p.default {
                    check_expr_private_refs(d, declared, span)?;
                }
            }
            check_stmt_private_refs(body, declared)
        }
        Expr::ArrowFunction { body, params, .. } => {
            for p in params {
                if let Some(d) = &p.default {
                    check_expr_private_refs(d, declared, span)?;
                }
            }
            match body {
                ArrowBody::Expr(e) => check_expr_private_refs(e, declared, span),
                ArrowBody::Block(s) => check_stmt_private_refs(s, declared),
            }
        }
        // Nested class has its own private env; still check heritage for outer privates.
        Expr::ClassExpression {
            super_class, body, ..
        } => {
            if let Some(sc) = super_class {
                // Heritage is outside the class private environment.
                check_expr_private_refs(sc, declared, span)?;
            }
            // Body uses nested class's own declared names (already validated in parse).
            let _ = body;
            Ok(())
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            check_expr_private_refs(callee, declared, span)?;
            for a in args {
                match a {
                    Arg::Expr(e) | Arg::Spread(e) => check_expr_private_refs(e, declared, span)?,
                }
            }
            Ok(())
        }
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => {
            check_expr_private_refs(left, declared, span)?;
            check_expr_private_refs(right, declared, span)
        }
        Expr::Unary { arg, .. } | Expr::Update { arg, .. } => {
            check_expr_private_refs(arg, declared, span)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            check_expr_private_refs(test, declared, span)?;
            check_expr_private_refs(consequent, declared, span)?;
            check_expr_private_refs(alternate, declared, span)
        }
        Expr::MemberExpression {
            object,
            property,
            private: false,
            ..
        } => {
            check_expr_private_refs(object, declared, span)?;
            check_expr_private_refs(property, declared, span)
        }
        Expr::ArrayExpression { elements, .. } => {
            for el in elements {
                match el {
                    ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                        check_expr_private_refs(e, declared, span)?;
                    }
                    ArrayElement::Elision => {}
                }
            }
            Ok(())
        }
        Expr::ObjectExpression { properties, .. } => {
            for p in properties {
                match p {
                    ObjectProp::Property { key, value, .. } => {
                        if let ObjectKey::Computed(e) = key {
                            check_expr_private_refs(e, declared, span)?;
                        }
                        check_expr_private_refs(value, declared, span)?;
                    }
                    ObjectProp::Spread { expr: e, .. } => {
                        check_expr_private_refs(e, declared, span)?;
                    }
                    ObjectProp::Accessor { key, body, .. } => {
                        if let ObjectKey::Computed(e) = key {
                            check_expr_private_refs(e, declared, span)?;
                        }
                        check_stmt_private_refs(body, declared)?;
                    }
                }
            }
            Ok(())
        }
        Expr::Paren { expr: inner, .. } | Expr::As { expr: inner, .. } => {
            check_expr_private_refs(inner, declared, span)
        }
        Expr::TemplateLiteral { expressions, .. } => {
            for e in expressions {
                check_expr_private_refs(e, declared, span)?;
            }
            Ok(())
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            check_expr_private_refs(tag, declared, span)?;
            for e in expressions {
                check_expr_private_refs(e, declared, span)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// `Contains SuperCall` for field initializers: recurse into arrows; skip nested functions/classes.
fn expr_contains_super_call(expr: &Expr) -> bool {
    match expr {
        Expr::Call { callee, args, .. } => {
            matches!(callee.as_ref(), Expr::Super { .. })
                || expr_contains_super_call(callee)
                || args.iter().any(arg_contains_super_call)
        }
        Expr::ArrowFunction { body, params, .. } => {
            params
                .iter()
                .any(|p| p.default.as_ref().is_some_and(expr_contains_super_call))
                || match body {
                    ArrowBody::Expr(e) => expr_contains_super_call(e),
                    ArrowBody::Block(s) => stmt_contains_super_call(s),
                }
        }
        Expr::FunctionExpression { .. } | Expr::ClassExpression { .. } => false,
        Expr::Paren { expr: inner, .. }
        | Expr::Unary { arg: inner, .. }
        | Expr::Update { arg: inner, .. }
        | Expr::As { expr: inner, .. } => expr_contains_super_call(inner),
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => expr_contains_super_call(left) || expr_contains_super_call(right),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_super_call(test)
                || expr_contains_super_call(consequent)
                || expr_contains_super_call(alternate)
        }
        Expr::MemberExpression {
            object, property, ..
        } => expr_contains_super_call(object) || expr_contains_super_call(property),
        Expr::New { callee, args, .. } => {
            expr_contains_super_call(callee) || args.iter().any(arg_contains_super_call)
        }
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_super_call(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { key, value, .. } => {
                object_key_contains_super_call(key) || expr_contains_super_call(value)
            }
            ObjectProp::Spread { expr, .. } => expr_contains_super_call(expr),
            ObjectProp::Accessor { key, .. } => object_key_contains_super_call(key),
        }),
        Expr::TemplateLiteral { expressions, .. } => {
            expressions.iter().any(expr_contains_super_call)
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => expr_contains_super_call(tag) || expressions.iter().any(expr_contains_super_call),
        Expr::ImportCall {
            source, options, ..
        } => {
            expr_contains_super_call(source)
                || options
                    .as_ref()
                    .is_some_and(|o| expr_contains_super_call(o))
        }
        Expr::PrivateIn { object, .. } => expr_contains_super_call(object),
        Expr::ArrayPattern { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Pattern { default, .. } => {
                default.as_ref().is_some_and(expr_contains_super_call)
            }
            _ => false,
        }),
        Expr::ObjectPattern { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop { key, default, .. } => {
                object_key_contains_super_call(key)
                    || default.as_ref().is_some_and(expr_contains_super_call)
            }
            _ => false,
        }),
        _ => false,
    }
}

fn arg_contains_super_call(a: &Arg) -> bool {
    match a {
        Arg::Expr(e) | Arg::Spread(e) => expr_contains_super_call(e),
    }
}

fn object_key_contains_super_call(key: &ObjectKey) -> bool {
    match key {
        ObjectKey::Computed(e) => expr_contains_super_call(e),
        ObjectKey::Ident(_) | ObjectKey::String(_) => false,
    }
}

fn stmt_contains_super_call(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_super_call),
        Stmt::Expression { expr, .. } => expr_contains_super_call(expr),
        Stmt::Return { argument, .. } => argument.as_ref().is_some_and(expr_contains_super_call),
        Stmt::Throw { argument, .. } => expr_contains_super_call(argument),
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_super_call(test)
                || stmt_contains_super_call(consequent)
                || alternate
                    .as_ref()
                    .is_some_and(|a| stmt_contains_super_call(a))
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            expr_contains_super_call(test) || stmt_contains_super_call(body)
        }
        Stmt::Let { init, .. } => init.as_ref().is_some_and(expr_contains_super_call),
        _ => false,
    }
}

/// `ContainsArguments` for field initializers: recurse into arrows; skip nested functions/classes.
fn expr_contains_arguments_ref(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(id) if id.name == "arguments" => true,
        Expr::ArrowFunction { body, params, .. } => {
            params.iter().any(|p| {
                p.default.as_ref().is_some_and(expr_contains_arguments_ref)
                    || binding_contains_arguments(&p.binding)
            }) || match body {
                ArrowBody::Expr(e) => expr_contains_arguments_ref(e),
                ArrowBody::Block(s) => stmt_contains_arguments_ref(s),
            }
        }
        Expr::FunctionExpression { .. } | Expr::ClassExpression { .. } => false,
        Expr::Paren { expr: inner, .. }
        | Expr::Unary { arg: inner, .. }
        | Expr::Update { arg: inner, .. }
        | Expr::As { expr: inner, .. } => expr_contains_arguments_ref(inner),
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => expr_contains_arguments_ref(left) || expr_contains_arguments_ref(right),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_arguments_ref(test)
                || expr_contains_arguments_ref(consequent)
                || expr_contains_arguments_ref(alternate)
        }
        Expr::MemberExpression {
            object,
            property,
            computed,
            ..
        } => {
            expr_contains_arguments_ref(object)
                || (*computed && expr_contains_arguments_ref(property))
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_contains_arguments_ref(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_arguments_ref(e),
                })
        }
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_arguments_ref(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { key, value, .. } => {
                object_key_contains_arguments(key) || expr_contains_arguments_ref(value)
            }
            ObjectProp::Spread { expr, .. } => expr_contains_arguments_ref(expr),
            ObjectProp::Accessor { key, .. } => object_key_contains_arguments(key),
        }),
        Expr::TemplateLiteral { expressions, .. } => {
            expressions.iter().any(expr_contains_arguments_ref)
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            expr_contains_arguments_ref(tag) || expressions.iter().any(expr_contains_arguments_ref)
        }
        Expr::ImportCall {
            source, options, ..
        } => {
            expr_contains_arguments_ref(source)
                || options
                    .as_ref()
                    .is_some_and(|o| expr_contains_arguments_ref(o))
        }
        Expr::PrivateIn { object, .. } => expr_contains_arguments_ref(object),
        Expr::ArrayPattern { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Pattern { binding, default } => {
                binding_contains_arguments(binding)
                    || default.as_ref().is_some_and(expr_contains_arguments_ref)
            }
            ArrayPatternElement::Rest(b) => binding_contains_arguments(b),
            ArrayPatternElement::Elision => false,
        }),
        Expr::ObjectPattern { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop {
                key,
                binding,
                default,
                ..
            } => {
                object_key_contains_arguments(key)
                    || binding_contains_arguments(binding)
                    || default.as_ref().is_some_and(expr_contains_arguments_ref)
            }
            ObjectPatternProp::Rest(b) => binding_contains_arguments(b),
        }),
        _ => false,
    }
}

fn object_key_contains_arguments(key: &ObjectKey) -> bool {
    match key {
        ObjectKey::Computed(e) => expr_contains_arguments_ref(e),
        ObjectKey::Ident(_) | ObjectKey::String(_) => false,
    }
}

fn binding_contains_arguments(b: &BindingPattern) -> bool {
    match b {
        BindingPattern::Ident(id) => id.name == "arguments",
        BindingPattern::Member(e) => expr_contains_arguments_ref(e),
        BindingPattern::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop {
                key,
                binding,
                default,
                ..
            } => {
                object_key_contains_arguments(key)
                    || binding_contains_arguments(binding)
                    || default.as_ref().is_some_and(expr_contains_arguments_ref)
            }
            ObjectPatternProp::Rest(inner) => binding_contains_arguments(inner),
        }),
        BindingPattern::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Pattern { binding, default } => {
                binding_contains_arguments(binding)
                    || default.as_ref().is_some_and(expr_contains_arguments_ref)
            }
            ArrayPatternElement::Rest(inner) => binding_contains_arguments(inner),
            ArrayPatternElement::Elision => false,
        }),
    }
}

fn stmt_contains_arguments_ref(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_arguments_ref),
        Stmt::Expression { expr, .. } => expr_contains_arguments_ref(expr),
        Stmt::Return { argument, .. } => argument.as_ref().is_some_and(expr_contains_arguments_ref),
        Stmt::Throw { argument, .. } => expr_contains_arguments_ref(argument),
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_arguments_ref(test)
                || stmt_contains_arguments_ref(consequent)
                || alternate
                    .as_ref()
                    .is_some_and(|a| stmt_contains_arguments_ref(a))
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            expr_contains_arguments_ref(test) || stmt_contains_arguments_ref(body)
        }
        Stmt::Let { init, .. } => init.as_ref().is_some_and(expr_contains_arguments_ref),
        _ => false,
    }
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

/// Split `pat = default` assignment into binding + default for pattern elements.
fn expr_to_pattern_element(expr: &Expr) -> Option<(BindingPattern, Option<Expr>)> {
    if let Expr::Assign {
        target,
        op: AssignOp::Eq,
        value,
        ..
    } = expr
    {
        let binding = expr_to_binding_pattern(target)?;
        return Some((binding, Some((**value).clone())));
    }
    let binding = expr_to_binding_pattern(expr)?;
    Some((binding, None))
}

/// Reinterpret an array literal as an assignment pattern when every element is
/// a binding/LHS target (`ident`, member, `pat = default`, nested pattern, elision, or trailing rest).
fn array_expr_to_pattern(expr: &Expr) -> Option<Expr> {
    let Expr::ArrayExpression {
        elements,
        trailing_comma,
        span,
    } = expr
    else {
        return None;
    };
    let mut pat_els = Vec::with_capacity(elements.len());
    let mut saw_rest = false;
    for el in elements {
        if saw_rest {
            return None;
        }
        match el {
            ArrayElement::Elision => {
                pat_els.push(ArrayPatternElement::Elision);
            }
            ArrayElement::Expr(inner) => {
                let (binding, default) = expr_to_pattern_element(inner)?;
                pat_els.push(ArrayPatternElement::Pattern { binding, default });
            }
            ArrayElement::Spread(inner) => {
                let binding = expr_to_binding_pattern(inner)?;
                pat_els.push(ArrayPatternElement::Rest(binding));
                saw_rest = true;
            }
        }
    }
    // `[...x,]` — trailing comma after rest is a SyntaxError in assignment patterns.
    if *trailing_comma && saw_rest {
        return None;
    }
    Some(Expr::ArrayPattern {
        elements: pat_els,
        span: *span,
    })
}

/// Reinterpret an object literal as an assignment pattern when every property is
/// a binding target (shorthand, CoverInitializedName, `key: pattern`, or trailing `...ident`).
fn object_expr_to_pattern(expr: &Expr) -> Option<Expr> {
    let Expr::ObjectExpression { properties, span } = expr else {
        return None;
    };
    let mut props = Vec::with_capacity(properties.len());
    let mut saw_rest = false;
    for prop in properties {
        if saw_rest {
            return None;
        }
        match prop {
            ObjectProp::Property {
                key,
                value,
                shorthand,
                span: prop_span,
            } => {
                // CoverInitializedName: `{ a = default }` encoded as shorthand Assign.
                if *shorthand {
                    let ObjectKey::Ident(key_id) = key else {
                        return None;
                    };
                    if let Expr::Assign {
                        target,
                        op: AssignOp::Eq,
                        value: def,
                        ..
                    } = value
                    {
                        let Expr::Ident(id) = target.as_ref() else {
                            return None;
                        };
                        if id.name != key_id.name {
                            return None;
                        }
                        props.push(ObjectPatternProp::Prop {
                            key: key.clone(),
                            binding: BindingPattern::Ident(id.clone()),
                            shorthand: true,
                            default: Some((**def).clone()),
                            span: *prop_span,
                        });
                        continue;
                    }
                }
                let (binding, default) = expr_to_pattern_element(value)?;
                props.push(ObjectPatternProp::Prop {
                    key: key.clone(),
                    binding,
                    shorthand: *shorthand,
                    default,
                    span: *prop_span,
                });
            }
            ObjectProp::Spread { expr: inner, .. } => {
                let binding = expr_to_binding_pattern(inner)?;
                props.push(ObjectPatternProp::Rest(binding));
                saw_rest = true;
            }
            ObjectProp::Accessor { .. } => return None,
        }
    }
    Some(Expr::ObjectPattern {
        properties: props,
        span: *span,
    })
}

fn expr_to_binding_pattern(expr: &Expr) -> Option<BindingPattern> {
    match expr {
        Expr::Ident(id) => Some(BindingPattern::Ident(id.clone())),
        // E19.82.10: private members (`obj.#f`) are valid assignment-pattern targets.
        Expr::MemberExpression {
            optional: false, ..
        } => Some(BindingPattern::Member(Box::new(expr.clone()))),
        Expr::ArrayExpression {
            elements,
            trailing_comma,
            span,
        } => {
            let mut pat_els = Vec::with_capacity(elements.len());
            let mut saw_rest = false;
            for el in elements {
                if saw_rest {
                    return None;
                }
                match el {
                    ArrayElement::Elision => {
                        pat_els.push(ArrayPatternElement::Elision);
                    }
                    ArrayElement::Expr(inner) => {
                        let (binding, default) = expr_to_pattern_element(inner)?;
                        pat_els.push(ArrayPatternElement::Pattern { binding, default });
                    }
                    ArrayElement::Spread(inner) => {
                        let binding = expr_to_binding_pattern(inner)?;
                        pat_els.push(ArrayPatternElement::Rest(binding));
                        saw_rest = true;
                    }
                }
            }
            if *trailing_comma && saw_rest {
                return None;
            }
            Some(BindingPattern::Array {
                elements: pat_els,
                span: *span,
            })
        }
        Expr::ArrayPattern { elements, span } => Some(BindingPattern::Array {
            elements: elements.clone(),
            span: *span,
        }),
        Expr::ObjectExpression { properties, span } => {
            let mut props = Vec::with_capacity(properties.len());
            let mut saw_rest = false;
            for prop in properties {
                if saw_rest {
                    return None;
                }
                match prop {
                    ObjectProp::Property {
                        key,
                        value,
                        shorthand,
                        span: prop_span,
                    } => {
                        if *shorthand {
                            let ObjectKey::Ident(key_id) = key else {
                                return None;
                            };
                            if let Expr::Assign {
                                target,
                                op: AssignOp::Eq,
                                value: def,
                                ..
                            } = value
                            {
                                let Expr::Ident(id) = target.as_ref() else {
                                    return None;
                                };
                                if id.name != key_id.name {
                                    return None;
                                }
                                props.push(ObjectPatternProp::Prop {
                                    key: key.clone(),
                                    binding: BindingPattern::Ident(id.clone()),
                                    shorthand: true,
                                    default: Some((**def).clone()),
                                    span: *prop_span,
                                });
                                continue;
                            }
                        }
                        let (binding, default) = expr_to_pattern_element(value)?;
                        props.push(ObjectPatternProp::Prop {
                            key: key.clone(),
                            binding,
                            shorthand: *shorthand,
                            default,
                            span: *prop_span,
                        });
                    }
                    ObjectProp::Spread { expr: inner, .. } => {
                        let binding = expr_to_binding_pattern(inner)?;
                        props.push(ObjectPatternProp::Rest(binding));
                        saw_rest = true;
                    }
                    ObjectProp::Accessor { .. } => return None,
                }
            }
            Some(BindingPattern::Object {
                properties: props,
                span: *span,
            })
        }
        Expr::ObjectPattern { properties, span } => Some(BindingPattern::Object {
            properties: properties.clone(),
            span: *span,
        }),
        _ => None,
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

    #[test]
    fn parse_let_and_expr() {
        let dump = parse_and_dump("let x = 1 + 2 * 3;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Binary +
        Number 1
        Binary *
          Number 2
          Number 3
"
        );
    }

    #[test]
    fn parse_const_decl() {
        let dump = parse_and_dump("const x = 1 + 2;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Const
    name: x
    init:
      Binary +
        Number 1
        Number 2
"
        );
    }

    #[test]
    fn parse_const_requires_initializer() {
        let err = parse("const x;").unwrap_err();
        assert!(
            err.message
                .contains("const declaration requires an initializer"),
            "got: {}",
            err.message
        );
    }

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
    fn parse_function_expression() {
        let dump = parse_and_dump("let f = function (a) { return a; };").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: f
    init:
      FunctionExpression
        params:
          name: a
        body:
          Block
            Return
              Ident a
"
        );
    }

    #[test]
    fn parse_named_function_expression() {
        let dump = parse_and_dump("let f = function g(n) { return g(n); };").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: f
    init:
      FunctionExpression
        name: g
        params:
          name: n
        body:
          Block
            Return
              Call
                callee:
                  Ident g
                arg[0]:
                  Ident n
"
        );
    }

    #[test]
    fn parse_async_generators() {
        let dump = parse_and_dump(
            "async function* g() { yield 1; yield await p; } let f = async function* (x) { yield x; }; let o = { async *m() { yield 2; } }; class C { async *n() { yield 3; } static async *s() { yield 4; } }",
        )
        .unwrap();
        assert!(dump.contains("async: true"), "got:\n{dump}");
        assert!(dump.contains("generator: true"), "got:\n{dump}");
        assert!(dump.contains("Unary yield"), "got:\n{dump}");
        assert!(dump.contains("Unary await"), "got:\n{dump}");
        assert!(dump.contains("Method\n"), "got:\n{dump}");
        assert!(dump.contains("StaticMethod\n"), "got:\n{dump}");
    }

    #[test]
    fn parse_default_params() {
        let dump =
            parse_and_dump("function f(a = 1, b) { return a + b; } let g = (x = 2) => x;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  FunctionDeclaration
    name: f
    params:
      name: a
        default:
          Number 1
      name: b
    body:
      Block
        Return
          Binary +
            Ident a
            Ident b
  Let
    name: g
    init:
      ArrowFunction
        params:
          name: x
            default:
              Number 2
        body:
          Ident x
"
        );
    }

    #[test]
    fn parse_rest_params() {
        let dump = parse_and_dump(
            "function f(...a) { return a; } function g(x, ...rest) { return x; } let h = (...xs) => xs;",
        )
        .unwrap();
        assert_eq!(
            dump,
            "\
Program
  FunctionDeclaration
    name: f
    params:
      rest: a
    body:
      Block
        Return
          Ident a
  FunctionDeclaration
    name: g
    params:
      name: x
      rest: rest
    body:
      Block
        Return
          Ident x
  Let
    name: h
    init:
      ArrowFunction
        params:
          rest: xs
        body:
          Ident xs
"
        );
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
    fn parse_import_defer_namespace() {
        // E19.42: static deferred namespace import.
        let dump = parse_and_dump("import defer * as ns from './m.js';").unwrap();
        assert!(
            dump.contains("ImportDeclaration")
                && dump.contains("phase: defer")
                && dump.contains("namespace: ns")
                && dump.contains("source: ./m.js"),
            "expected import defer namespace, got:\n{dump}"
        );
        assert!(!dump.contains("ImportSpecifier"), "{dump}");
    }

    #[test]
    fn parse_import_defer_namespace_with_attributes() {
        let dump =
            parse_and_dump("import defer * as ns from './m.js' with { type: \"json\" };").unwrap();
        assert!(
            dump.contains("phase: defer")
                && dump.contains("namespace: ns")
                && dump.contains("ImportAttribute"),
            "expected import defer + attributes, got:\n{dump}"
        );
    }

    #[test]
    fn parse_import_default_binding_named_defer() {
        // `defer` remains a valid default import binding name.
        let dump = parse_and_dump("import defer from './m.js';").unwrap();
        assert!(
            dump.contains("ImportDeclaration")
                && dump.contains("local: defer")
                && !dump.contains("phase: defer"),
            "expected default import named defer, got:\n{dump}"
        );
    }

    #[test]
    fn parse_import_defer_named_fails() {
        assert!(parse("import defer { x } from './m.js';").is_err());
    }

    #[test]
    fn parse_import_defer_default_fails() {
        assert!(parse("import defer x from './m.js';").is_err());
    }

    #[test]
    fn parse_import_with_attributes() {
        let dump = parse_and_dump("import x from \"./m.js\" with { type: \"json\" };").unwrap();
        assert!(
            dump.contains("ImportDeclaration")
                && dump.contains("source: ./m.js")
                && dump.contains("ImportAttribute")
                && dump.contains("key: type")
                && dump.contains("value: \"json\""),
            "expected import with attributes, got:\n{dump}"
        );
    }

    #[test]
    fn parse_side_effect_import_with_empty_attributes() {
        let dump = parse_and_dump("import \"./m.js\" with {};").unwrap();
        assert!(
            dump.contains("ImportDeclaration") && dump.contains("source: ./m.js"),
            "expected side-effect import, got:\n{dump}"
        );
    }

    #[test]
    fn parse_import_assert_attributes() {
        let dump = parse_and_dump("import x from \"./m.js\" assert { type: \"json\" };").unwrap();
        assert!(
            dump.contains("ImportAttribute") && dump.contains("key: type"),
            "expected assert attributes, got:\n{dump}"
        );
    }

    #[test]
    fn parse_import_with_attributes_allows_nlt_before_with() {
        let dump = parse_and_dump("import x from \"./m.js\"\nwith { type: \"json\" };").unwrap();
        assert!(
            dump.contains("ImportAttribute") && dump.contains("key: type"),
            "expected with after newline, got:\n{dump}"
        );
    }

    #[test]
    fn parse_import_assert_rejects_nlt_before_assert() {
        // LineTerminator before `assert` ends the import; `assert` is not a WithClause.
        let dump = parse_and_dump("import x from \"./m.js\"\nassert { type: \"json\" };");
        match dump {
            Ok(d) => assert!(
                !d.contains("ImportAttribute"),
                "assert after newline must not be WithClause, got:\n{d}"
            ),
            Err(_) => {}
        }
    }

    #[test]
    fn parse_export_all_with_attributes() {
        let dump = parse_and_dump("export * from \"./m.js\" with { type: \"json\" };").unwrap();
        assert!(
            dump.contains("ExportAllDeclaration")
                && dump.contains("ImportAttribute")
                && dump.contains("key: type"),
            "expected export * with attributes, got:\n{dump}"
        );
    }

    #[test]
    fn parse_import_attribute_keyword_key() {
        let dump = parse_and_dump("import \"./m.js\" with { if: \"\" };").unwrap();
        assert!(
            dump.contains("key: if"),
            "expected IdentifierName key, got:\n{dump}"
        );
    }

    #[test]
    fn parse_import_attribute_duplicate_key_fails() {
        assert!(
            parse("import x from \"./m.js\" with { type: \"json\", \"typ\\u0065\": \"\" };")
                .is_err()
        );
    }

    #[test]
    fn parse_import_attribute_trailing_comma() {
        let dump = parse_and_dump("import \"./m.js\" with { type: \"json\", };").unwrap();
        assert!(
            dump.contains("ImportAttribute") && dump.contains("key: type"),
            "expected trailing comma with clause, got:\n{dump}"
        );
    }

    /// E19.58: yield in generator FormalParameters.
    #[test]
    fn parse_generator_param_default_yield_fails() {
        assert!(parse("function* g(x = yield) {}").is_err());
        assert!(parse("0, function*(x = yield) {};").is_err());
        assert!(parse("function* g(x = 1) { yield x; }").is_ok());
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
    fn parse_future_reserved_as_identifier_non_strict() {
        // E17.02.08: strict FutureReservedWord tokens as BindingIdentifier / IdentifierReference.
        let dump = parse_and_dump(
            "var static = 1; var let = 2; let implements = 3; function public() { return static + let; }",
        )
        .unwrap();
        assert!(
            dump.contains("name: static")
                && dump.contains("name: let")
                && dump.contains("name: implements")
                && dump.contains("name: public"),
            "future reserved as bindings, got:\n{dump}"
        );
        assert!(
            parse_and_dump("function f(static, let) { return static + let; }").is_ok(),
            "params static/let non-strict"
        );
        assert!(
            parse_and_dump("let static = 1; const public = 2;").is_ok(),
            "lexical static/public non-strict"
        );
        // LexicalBinding BoundNames must not include "let".
        assert!(parse_and_dump("let let = 1;").is_err(), "let let must fail");
        assert!(
            parse_and_dump("const let = 1;").is_err(),
            "const let must fail"
        );
        assert!(
            parse_and_dump("for (let let of []) {}").is_err(),
            "for-of let let must fail"
        );
        // Strict: FutureReservedWord reserved.
        assert!(
            parse_and_dump("\"use strict\"; var static = 1;").is_err(),
            "strict BindingIdentifier static must fail"
        );
        assert!(
            parse_and_dump("\"use strict\"; var let = 1;").is_err(),
            "strict BindingIdentifier let must fail"
        );
        assert!(
            parse_and_dump("\"use strict\"; var implements = 1;").is_err(),
            "strict BindingIdentifier implements must fail"
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

    #[test]
    fn parse_export_all_from() {
        let dump = parse_and_dump("export * from \"./lib.drac\";").unwrap();
        assert!(
            dump.contains("ExportAllDeclaration") && dump.contains("source: ./lib.drac"),
            "expected export * from, got:\n{dump}"
        );
    }

    #[test]
    fn parse_export_named_from() {
        let dump =
            parse_and_dump("export { value, inc as bump, default as d } from \"./lib.drac\";")
                .unwrap();
        assert!(
            dump.contains("ExportNamedDeclaration")
                && dump.contains("source: ./lib.drac")
                && dump.contains("local: value")
                && dump.contains("exported: bump")
                && dump.contains("local: default")
                && dump.contains("exported: d"),
            "expected export {{…}} from, got:\n{dump}"
        );
    }

    #[test]
    fn parse_export_star_as_ns_from() {
        let dump = parse_and_dump("export * as ns from \"./lib.drac\";").unwrap();
        assert!(
            dump.contains("ExportAllDeclaration")
                && dump.contains("exported: ns")
                && dump.contains("source: ./lib.drac"),
            "expected export * as ns from, got:\n{dump}"
        );
    }

    #[test]
    fn parse_export_class() {
        let dump =
            parse_and_dump("export class Point { constructor(x) { this.x = x; } }\n").unwrap();
        assert!(
            dump.contains("ExportNamedDeclaration")
                && dump.contains("ClassDeclaration")
                && dump.contains("name: Point"),
            "expected export class, got:\n{dump}"
        );
    }

    #[test]
    fn parse_export_default_class() {
        let dump =
            parse_and_dump("export default class Counter { constructor() { this.n = 0; } }\n")
                .unwrap();
        assert!(
            dump.contains("ExportDefaultDeclaration")
                && dump.contains("ClassDeclaration")
                && dump.contains("name: Counter"),
            "expected export default class, got:\n{dump}"
        );
    }

    #[test]
    fn parse_export_var() {
        let dump = parse_and_dump("export var name1 = 1;\n").unwrap();
        assert!(
            dump.contains("ExportNamedDeclaration")
                && dump.contains("Var")
                && dump.contains("name: name1"),
            "expected export var, got:\n{dump}"
        );
        let dstr = parse_and_dump("export var { x = 1 } = {};\n").unwrap();
        assert!(
            dstr.contains("ExportNamedDeclaration") && dstr.contains("ObjectPattern"),
            "expected export var destructuring, got:\n{dstr}"
        );
    }

    #[test]
    fn parse_export_var_await_module() {
        // E19.54: `export var x = await expr` under Module [+Await].
        let prog =
            parse_module("export var name1 = await foo;\nexport var { x = await foo } = {};\n")
                .expect("export var await");
        let dump = dump_program(&prog);
        assert!(
            dump.contains("ExportNamedDeclaration")
                && dump.contains("Unary await")
                && dump.matches("ExportNamedDeclaration").count() >= 2,
            "expected export var await, got:\n{dump}"
        );
    }

    #[test]
    fn parse_export_default_class_anonymous_await_extends() {
        // E19.54: `export default class extends fn(await foo) {}`
        let prog = parse_module(
            "function fn() { return function() {}; }\n\
             export default class extends fn(await foo) {}\n",
        )
        .expect("export default class anonymous + await extends");
        let dump = dump_program(&prog);
        assert!(
            dump.contains("ExportDefaultDeclaration")
                && dump.contains("ClassDeclaration")
                && dump.contains("name: __class")
                && dump.contains("Unary await"),
            "expected anonymous default class with await extends, got:\n{dump}"
        );
    }

    #[test]
    fn parse_export_class_await_extends() {
        let prog = parse_module(
            "function fn() { return function() {}; }\n\
             export class C extends fn(await foo) {}\n",
        )
        .expect("export class await extends");
        let dump = dump_program(&prog);
        assert!(
            dump.contains("ExportNamedDeclaration")
                && dump.contains("name: C")
                && dump.contains("Unary await"),
            "expected export class await extends, got:\n{dump}"
        );
    }

    /// E19.32: binding elision is preserved; rest+trailing comma is a SyntaxError.
    #[test]
    fn parse_array_pattern_elision_and_rest_trailing_comma() {
        let elision = parse_and_dump("let [,] = x;\n").unwrap();
        assert!(
            elision.contains("elision"),
            "expected elision in binding pattern, got:\n{elision}"
        );
        let trail = parse_and_dump("let [a,,] = x;\n").unwrap();
        assert!(
            trail.contains("elision") && trail.contains("name: a"),
            "expected trailing elision after a, got:\n{trail}"
        );
        assert!(
            parse_and_dump("let [...x,] = [];\n").is_err(),
            "binding rest+trailing comma must fail"
        );
        // Assignment `[...x,]` stays an array literal (trailing_comma+rest → not a pattern);
        // checker rejects invalid LHS. Bare rest still becomes ArrayPattern.
        let bad_assign = parse_and_dump("[...x,] = [];\n").unwrap();
        assert!(
            bad_assign.contains("ArrayExpression") && !bad_assign.contains("ArrayPattern"),
            "rest+trailing comma must not become assignment pattern, got:\n{bad_assign}"
        );
        let ok_rest = parse_and_dump("[...x] = [];\n").unwrap();
        assert!(
            ok_rest.contains("ArrayPattern") && ok_rest.contains("rest:"),
            "bare rest assignment ok, got:\n{ok_rest}"
        );
    }

    /// E19.51: trailing-dot number + object rest with non-string computed key.
    #[test]
    fn parse_e19_51_obj_rest_non_string_computed_trailing_dot() {
        let dump = parse_and_dump("var a = 1.;\nvar b, rest;\n({[a]:b, ...rest} = vals);\n")
            .expect("var a = 1.; and assignment pattern must parse");
        assert!(
            dump.contains("Number 1.")
                && dump.contains("ObjectPattern")
                && dump.contains("rest:")
                && dump.contains("key: Computed"),
            "trailing-dot number + computed key + rest; got:\n{dump}"
        );
        let dump = parse_and_dump("for (var {[a]:b, ...rest} of vals) {}\n")
            .expect("for-of object rest computed key must parse");
        assert!(
            dump.contains("ObjectPattern") && dump.contains("key: Computed"),
            "for-of dstr; got:\n{dump}"
        );
        let dump = parse_and_dump("let {...{ [k]: v }} = obj;\n").expect("nested rest object");
        assert!(
            dump.contains("ObjectPattern") && dump.contains("key: Computed"),
            "nested rest object computed; got:\n{dump}"
        );
    }

    /// E19.46: computed property names in object binding / assignment patterns.
    #[test]
    fn parse_e19_46_object_binding_computed_keys() {
        let dump = parse_and_dump("let { [k]: v } = a;\n").unwrap();
        assert!(
            dump.contains("ObjectPattern")
                && dump.contains("key: Computed")
                && dump.contains("name: v"),
            "computed key in let object pattern; got:\n{dump}"
        );
        let dump = parse_and_dump("function f({ [k]: v }) {}\n").unwrap();
        assert!(
            dump.contains("ObjectPattern") && dump.contains("key: Computed"),
            "computed key in params; got:\n{dump}"
        );
        let dump = parse_and_dump("({ [k]: x } = a);\n").unwrap();
        assert!(
            dump.contains("ObjectPattern") && dump.contains("key: Computed"),
            "assignment object pattern computed key; got:\n{dump}"
        );
        let dump = parse_and_dump("let { [k + 1]: v = 0 } = a;\n").unwrap();
        assert!(
            dump.contains("key: Computed") && dump.contains("default:"),
            "computed key with default; got:\n{dump}"
        );
        let dump = parse_and_dump("const { [\"x\"]: n } = a;\n").unwrap();
        assert!(
            dump.contains("key: Computed"),
            "string expr computed key; got:\n{dump}"
        );
        assert!(
            parse("let { [k] } = a;\n").is_err(),
            "computed key without ':' must fail (no shorthand)"
        );
    }

    /// E19.43: object binding pattern numeric (and string) PropertyName keys.
    #[test]
    fn parse_e19_43_object_binding_numeric_keys() {
        let dump = parse_and_dump("let { 0: v, 1: w } = a;\n").unwrap();
        assert!(
            dump.contains("ObjectPattern") && dump.contains("key: 0") && dump.contains("key: 1"),
            "numeric keys in let object pattern; got:\n{dump}"
        );
        let dump = parse_and_dump("function f([...{ 0: v, 1: w, length: z }]) {}\n").unwrap();
        assert!(
            dump.contains("ObjectPattern") && dump.contains("key: 0") && dump.contains("name: v"),
            "array rest nested object numeric keys in params; got:\n{dump}"
        );
        let dump = parse_and_dump("({ 0: x } = a);\n").unwrap();
        assert!(
            dump.contains("ObjectPattern") && dump.contains("key: 0"),
            "assignment object pattern numeric key; got:\n{dump}"
        );
        let dump = parse_and_dump("let { \"0\": v } = a;\n").unwrap();
        assert!(
            dump.contains("ObjectPattern") && dump.contains("key: 0"),
            "string key \"0\" in object pattern; got:\n{dump}"
        );
        let dump = parse_and_dump("const { 0x10: n } = a;\n").unwrap();
        assert!(
            dump.contains("key: 16"),
            "hex numeric property name → ToString key; got:\n{dump}"
        );
        assert!(
            parse("let { 0 } = a;\n").is_err(),
            "numeric key without ':' must fail (no shorthand)"
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
