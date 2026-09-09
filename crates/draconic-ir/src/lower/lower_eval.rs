use std::collections::HashSet;

use draconic_ast::{ClassElement, Expr as AstExpr, Stmt as AstStmt};
use draconic_check::{CheckedProgram, Type};

use crate::lower::LowerCtx;
use crate::{Arg, Expr, Stmt};

use crate::lower::lower_expr::lower_expr;

pub(crate) fn ast_expr_is_eval_ident(expr: &AstExpr) -> bool {
    match expr {
        AstExpr::Ident(id) => id.name == "eval",
        AstExpr::Paren { expr: inner, .. } => ast_expr_is_eval_ident(inner),
        _ => false,
    }
}

/// String value of a string/template-no-sub literal (parens peeled), if any.
pub(crate) fn ast_string_literal_value(expr: &AstExpr) -> Option<String> {
    match expr {
        AstExpr::Paren { expr: inner, .. } => ast_string_literal_value(inner),
        AstExpr::String(s) => Some(s.value.to_string_lossy()),
        AstExpr::TemplateLiteral {
            quasis,
            expressions,
            ..
        } if expressions.is_empty() && quasis.len() == 1 => {
            Some(quasis[0].cooked.to_string_lossy())
        }
        _ => None,
    }
}

/// `ContainsArguments` over eval source text (skip strings/comments/templates roughly).
pub(crate) fn source_contains_arguments_ident(src: &str) -> bool {
    let b = src.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        let c = b[i];
        // line comment
        if c == b'/' && i + 1 < b.len() && b[i + 1] == b'/' {
            i += 2;
            while i < b.len() && b[i] != b'\n' && b[i] != b'\r' {
                i += 1;
            }
            continue;
        }
        // block comment
        if c == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i = i.saturating_add(2);
            continue;
        }
        // string ' or "
        if c == b'\'' || c == b'"' {
            let q = c;
            i += 1;
            while i < b.len() {
                if b[i] == b'\\' {
                    i = i.saturating_add(2);
                    continue;
                }
                if b[i] == q {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        // template literal (skip; nested ${} not fully parsed — false negatives ok for tests)
        if c == b'`' {
            i += 1;
            while i < b.len() {
                if b[i] == b'\\' {
                    i = i.saturating_add(2);
                    continue;
                }
                if b[i] == b'`' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        // identifier start
        if c == b'_' || c == b'$' || c.is_ascii_alphabetic() || (c >= 0x80) {
            let start = i;
            i += 1;
            while i < b.len() {
                let d = b[i];
                if d == b'_' || d == b'$' || d.is_ascii_alphanumeric() || d >= 0x80 {
                    i += 1;
                } else {
                    break;
                }
            }
            if &src[start..i] == "arguments" {
                return true;
            }
            continue;
        }
        i += 1;
    }
    false
}

/// True when the running private environment is non-empty (fields/methods/accessors).
pub(crate) fn ctx_has_private_env(ctx: &LowerCtx) -> bool {
    !ctx.private_fields.is_empty()
        || !ctx.private_methods.is_empty()
        || !ctx.private_accessors.is_empty()
}

/// Collect private identifier names currently in scope for eval fragment wrapping.
pub(crate) fn ctx_private_names(ctx: &LowerCtx) -> Vec<String> {
    let mut names: HashSet<String> = HashSet::new();
    names.extend(ctx.private_fields.keys().cloned());
    names.extend(ctx.private_methods.keys().cloned());
    names.extend(ctx.private_accessors.keys().cloned());
    let mut v: Vec<String> = names.into_iter().collect();
    v.sort();
    v
}

/// Parse `src` as an expression under a synthetic class that declares `private_names`,
/// so AllPrivateNamesValid accepts `#m` refs (E19.82.08).
pub(crate) fn parse_eval_expr_with_privates(
    src: &str,
    private_names: &[String],
) -> Option<AstExpr> {
    let mut decls = String::new();
    for n in private_names {
        decls.push('#');
        decls.push_str(n);
        decls.push(';');
    }
    // Parenthesize so assignment / comma / etc. parse as a single Expression.
    let wrapped = format!("class __DracEvalPriv {{{decls}__run(){{return({src});}}}}");
    let program = draconic_parser::parse(&wrapped).ok()?;
    extract_synthetic_eval_return_expr(&program)
}

pub(crate) fn extract_synthetic_eval_return_expr(
    program: &draconic_ast::Program,
) -> Option<AstExpr> {
    let stmt = program.body.first()?;
    let AstStmt::ClassDeclaration { body, .. } = stmt else {
        return None;
    };
    for el in body {
        if let ClassElement::Method {
            key,
            body: method_body,
            is_static: false,
            is_private: false,
            ..
        } = el
        {
            let is_run = match key {
                draconic_ast::ObjectKey::Ident(id) => id.name == "__run",
                draconic_ast::ObjectKey::String(s) => s.value.to_string_lossy() == "__run",
                _ => false,
            };
            if !is_run {
                continue;
            }
            let AstStmt::Block { body, .. } = method_body.as_ref() else {
                continue;
            };
            if let Some(AstStmt::Return {
                argument: Some(expr),
                ..
            }) = body.first()
            {
                return Some(expr.clone());
            }
        }
    }
    None
}

/// Direct `eval("…#m…")` with a private environment: lower the string as an expression
/// so WeakMap/brand desugaring applies (native `#` would not see our desugared fields).
pub(crate) fn try_lower_direct_eval_private(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    src: &str,
    super_class: Option<&AstExpr>,
) -> Option<Expr> {
    if !src.contains('#') || !ctx_has_private_env(ctx) {
        return None;
    }
    let names = ctx_private_names(ctx);
    let expr = parse_eval_expr_with_privates(src, &names)?;
    Some(lower_expr(checked, ctx, &expr, super_class))
}

/// `(() => { throw new SyntaxError("…arguments…"); })()` for field-init eval (E19.82.06).
pub(crate) fn field_init_eval_arguments_error() -> Expr {
    let msg = Expr::String {
        value:
            "'arguments' is not allowed in class field initializer or static initialization block"
                .into(),
        ty: Type::String,
    };
    let err = Expr::New {
        callee: Box::new(Expr::IdentName {
            name: "SyntaxError".into(),
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(msg)],
        ty: Type::Any,
    };
    let throw_fn = Expr::Function {
        name: None,
        params: Vec::new(),
        body: vec![Stmt::Throw { value: err }],
        is_async: false,
        is_generator: false,
        is_arrow: true,
        is_method: false,
        ty: Type::Function,
    };
    Expr::Call {
        callee: Box::new(throw_fn),
        args: Vec::new(),
        optional: false,
        ty: Type::Any,
    }
}
