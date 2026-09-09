use std::collections::HashMap;

use draconic_ast::{BindingKind, BindingPattern, Stmt};
use draconic_diagnostics::{Diagnostic, Span};

/// If any catch-parameter bound name appears in LexicallyDeclaredNames of the
/// catch Block, return the conflicting name and its declaration span. Annex
/// B.3.4 allows the same name in VarDeclaredNames (`var`); only lexical
/// `let`/`const`/`class`/`function` at the top level of the catch block are rejected.
pub(crate) fn catch_lexical_conflict(
    param: &BindingPattern,
    handler: &Stmt,
) -> Option<(String, Span)> {
    let body: &[Stmt] = match handler {
        Stmt::Block { body, .. } => body.as_slice(),
        other => std::slice::from_ref(other),
    };
    let mut conflict = None;
    param.for_each_ident(&mut |id| {
        if conflict.is_some() {
            return;
        }
        for stmt in body {
            if let Some(span) = catch_stmt_lexical_name(stmt, &id.name) {
                conflict = Some((id.name.clone(), span));
                return;
            }
        }
    });
    conflict
}

fn catch_stmt_lexical_name(stmt: &Stmt, param: &str) -> Option<Span> {
    let mut s = stmt;
    while let Stmt::Labeled { body, .. } = s {
        s = body;
    }
    match s {
        Stmt::Let {
            kind:
                BindingKind::Let | BindingKind::Const | BindingKind::Using | BindingKind::AwaitUsing,
            binding,
            ..
        } => {
            let mut found = None;
            binding.for_each_ident(&mut |id| {
                if found.is_none() && id.name == param {
                    found = Some(id.span);
                }
            });
            found
        }
        Stmt::ClassDeclaration { name, .. } | Stmt::FunctionDeclaration { name, .. }
            if name.name == param =>
        {
            Some(name.span)
        }
        _ => None,
    }
}

/// Lexical binding kind for statement-list early errors (E19.24 / Annex B.3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LexNameKind {
    /// Plain `function` (not async/generator): sloppy mode may allow duplicates among these only.
    PlainFunction,
    Other,
}

fn peel_labels(stmt: &Stmt) -> &Stmt {
    let mut s = stmt;
    while let Stmt::Labeled { body, .. } = s {
        s = body;
    }
    s
}

/// LexicallyDeclaredNames of a StatementList (not nested blocks).
///
/// When `top_level` (Script / FunctionBody), hoistable `function`/`async`/`generator`
/// declarations are **not** lexical (TopLevelLexicallyDeclaredNames); they are var-like.
pub(crate) fn collect_lexically_declared_names<'a, I>(
    stmts: I,
    top_level: bool,
) -> Vec<(String, Span, LexNameKind)>
where
    I: IntoIterator<Item = &'a Stmt>,
{
    let mut out = Vec::new();
    for stmt in stmts {
        let s = peel_labels(stmt);
        match s {
            Stmt::Let {
                kind:
                    BindingKind::Let | BindingKind::Const | BindingKind::Using | BindingKind::AwaitUsing,
                binding,
                ..
            } => {
                binding.for_each_ident(&mut |id| {
                    out.push((id.name.clone(), id.span, LexNameKind::Other));
                });
            }
            Stmt::ClassDeclaration { name, .. } => {
                out.push((name.name.clone(), name.span, LexNameKind::Other));
            }
            Stmt::FunctionDeclaration {
                name,
                is_async,
                is_generator,
                ..
            } => {
                // Script/FunctionBody: hoistables are TopLevelVarDeclaredNames only.
                if top_level {
                    continue;
                }
                let kind = if *is_async || *is_generator {
                    LexNameKind::Other
                } else {
                    LexNameKind::PlainFunction
                };
                out.push((name.name.clone(), name.span, kind));
            }
            _ => {}
        }
    }
    out
}

/// VarDeclaredNames of a StatementList (walks nested statements; not function/class bodies).
///
/// When `top_level`, direct hoistable function declarations are included
/// (TopLevelVarDeclaredNames).
fn collect_var_declared_names<'a, I>(stmts: I, top_level: bool) -> Vec<(String, Span)>
where
    I: IntoIterator<Item = &'a Stmt>,
{
    let mut out = Vec::new();
    for stmt in stmts {
        if top_level {
            let s = peel_labels(stmt);
            match s {
                Stmt::FunctionDeclaration { name, .. }
                | Stmt::ExternFunctionDeclaration { name, .. } => {
                    out.push((name.name.clone(), name.span));
                }
                _ => {}
            }
        }
        collect_var_declared_names_stmt(stmt, &mut out);
    }
    out
}

pub(crate) fn collect_var_declared_names_stmt(stmt: &Stmt, out: &mut Vec<(String, Span)>) {
    match stmt {
        Stmt::Labeled { body, .. } => collect_var_declared_names_stmt(body, out),
        Stmt::Let {
            kind: BindingKind::Var,
            binding,
            ..
        } => {
            binding.for_each_ident(&mut |id| {
                out.push((id.name.clone(), id.span));
            });
        }
        Stmt::Block { body, .. } => {
            for child in body {
                collect_var_declared_names_stmt(child, out);
            }
        }
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            collect_var_declared_names_stmt(consequent, out);
            if let Some(alt) = alternate {
                collect_var_declared_names_stmt(alt, out);
            }
        }
        Stmt::While { body, .. } | Stmt::DoWhile { body, .. } | Stmt::With { body, .. } => {
            collect_var_declared_names_stmt(body, out);
        }
        Stmt::For { init, body, .. } => {
            if let Some(init) = init {
                collect_var_declared_names_stmt(init, out);
            }
            collect_var_declared_names_stmt(body, out);
        }
        Stmt::ForIn { left, body, .. } | Stmt::ForOf { left, body, .. } => {
            collect_var_declared_names_stmt(left, out);
            collect_var_declared_names_stmt(body, out);
        }
        Stmt::Switch { cases, .. } => {
            for case in cases {
                for child in &case.body {
                    collect_var_declared_names_stmt(child, out);
                }
            }
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            collect_var_declared_names_stmt(block, out);
            if let Some(handler) = handler {
                collect_var_declared_names_stmt(handler, out);
            }
            if let Some(finalizer) = finalizer {
                collect_var_declared_names_stmt(finalizer, out);
            }
        }
        Stmt::FunctionDeclaration { .. } | Stmt::ClassDeclaration { .. } => {}
        _ => {}
    }
}

/// Statement-list early errors (E19.24).
///
/// - LexicallyDeclaredNames must not contain duplicates (Annex B: sloppy plain
///   `function` duplicates only are allowed — block/switch only).
/// - LexicallyDeclaredNames ∩ VarDeclaredNames must be empty.
///
/// `top_level`: Script or FunctionBody (TopLevel*DeclaredNames); otherwise Block/CaseBlock.
pub(crate) fn check_statement_list_early_errors<'a, I>(
    stmts: I,
    strict: bool,
    top_level: bool,
) -> Result<(), Diagnostic>
where
    I: IntoIterator<Item = &'a Stmt> + Clone,
{
    let lexical = collect_lexically_declared_names(stmts.clone(), top_level);
    let mut seen: HashMap<String, LexNameKind> = HashMap::new();
    for (name, span, kind) in &lexical {
        if let Some(prev) = seen.get(name) {
            let allow_sloppy_fn = !strict
                && !top_level
                && *prev == LexNameKind::PlainFunction
                && *kind == LexNameKind::PlainFunction;
            if !allow_sloppy_fn {
                return Err(Diagnostic::new(
                    format!("duplicate declaration of `{name}`"),
                    *span,
                ));
            }
        } else {
            seen.insert(name.clone(), *kind);
        }
    }
    let vars = collect_var_declared_names(stmts, top_level);
    let mut var_names = HashMap::new();
    for (name, span) in vars {
        var_names.entry(name).or_insert(span);
    }
    for (name, span, _) in &lexical {
        if var_names.contains_key(name) {
            return Err(Diagnostic::new(
                format!("duplicate declaration of `{name}`"),
                *span,
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::bind_globals::find_ident_use;
    use crate::{bind, check};
    use draconic_ast::BindingKind;
    use draconic_parser::parse;

    #[test]
    fn bind_duplicate_let_errors() {
        let program = parse("let x = 1; let x = 2;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("x"),
            "unexpected message: {}",
            err.message
        );
    }

    // E19.24: early SyntaxError for strict arrow eval/arguments + block/switch redeclarations.
    #[test]
    fn bind_strict_arrow_eval_param_errors() {
        let program = parse("\"use strict\"; let af = eval => 1;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("eval") && err.message.contains("strict"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_strict_arrow_arguments_param_errors() {
        let program = parse("\"use strict\"; let af = (arguments) => 1;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("arguments") && err.message.contains("strict"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_sloppy_arrow_eval_param_ok() {
        let program = parse("let af = eval => eval;").unwrap();
        bind(program).expect("sloppy arrow may bind eval");
    }

    // E19.49: strict eval/arguments bindings + assign targets.
    #[test]
    fn bind_e19_49_strict_var_eval_errors() {
        let program = parse("\"use strict\"; var eval;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("eval") && err.message.contains("strict"),
            "unexpected: {}",
            err.message
        );
        let program = parse("\"use strict\"; var arguments;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("arguments") && err.message.contains("strict"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn bind_e19_49_strict_catch_eval_errors() {
        let program = parse("\"use strict\"; try {} catch (eval) {}").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("eval") && err.message.contains("strict"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn bind_e19_49_strict_assign_eval_errors() {
        let program = parse("\"use strict\"; eval = 1;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("eval") && err.message.contains("strict"),
            "unexpected: {}",
            err.message
        );
        let program = parse("\"use strict\"; arguments += 1;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("arguments") && err.message.contains("strict"),
            "unexpected: {}",
            err.message
        );
        let program = parse("\"use strict\"; ++arguments;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("arguments") && err.message.contains("strict"),
            "unexpected: {}",
            err.message
        );
        let program = parse("\"use strict\"; (eval) = 1;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("eval") && err.message.contains("strict"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn bind_e19_49_sloppy_eval_assign_ok() {
        bind(parse("eval = 1;").unwrap()).expect("sloppy eval assign");
        bind(parse("var eval;").unwrap()).expect("sloppy var eval");
    }

    // E19.39: early SyntaxError residuals.
    #[test]
    fn bind_use_strict_non_simple_params_errors() {
        let program = parse("function f(a = 0) { \"use strict\"; }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("use strict") || err.message.contains("non-simple"),
            "unexpected: {}",
            err.message
        );
        let program = parse("({ m(a = 0) { \"use strict\"; } });").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("use strict") || err.message.contains("non-simple"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn bind_strict_delete_identifier_errors() {
        let program = parse("\"use strict\"; delete x;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("delete"),
            "unexpected: {}",
            err.message
        );
        let program = parse("\"use strict\"; delete ((x));").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("delete"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn bind_method_param_redecl_errors() {
        let program = parse("({ method(param) { let param; } });").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") || err.message.contains("param"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn bind_object_method_super_call_errors() {
        let program = parse("({ m() { super(); } });").unwrap();
        let err = bind(program).unwrap_err();
        assert!(err.message.contains("super"), "unexpected: {}", err.message);
    }

    // E17.02.04: duplicate formals allowed only for non-strict simple plain `function`.
    #[test]
    fn bind_sloppy_duplicate_params_ok() {
        let program = parse("function f(a, a) { return a; }").unwrap();
        bind(program).expect("sloppy simple duplicate formals");
    }

    #[test]
    fn bind_sloppy_duplicate_params_function_expr_ok() {
        let program = parse("let f = function (a, b, a) { return a; };").unwrap();
        bind(program).expect("sloppy FE simple duplicate formals");
    }

    #[test]
    fn bind_strict_duplicate_params_errors() {
        let program = parse("function f(a, a) { \"use strict\"; return a; }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("a"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_duplicate_params_with_default_errors() {
        let program = parse("function f(a, a = 1) { return a; }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("a"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_duplicate_params_arrow_errors() {
        let program = parse("let f = (a, a) => a;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("a"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_duplicate_params_method_errors() {
        let program = parse("let o = { m(a, a) { return a; } };").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("a"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_duplicate_params_async_errors() {
        let program = parse("async function f(a, a) { return a; }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("a"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_duplicate_params_generator_errors() {
        let program = parse("function* f(a, a) { yield a; }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("a"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_block_function_let_redeclaration_errors() {
        let program = parse("{ function f() {} let f }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("f"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_block_var_let_redeclaration_errors() {
        let program = parse("{ var f; let f }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("f"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_switch_var_let_redeclaration_errors() {
        let program = parse("switch (0) { case 1: var f; default: let f }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("f"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_inner_block_var_outer_let_redeclaration_errors() {
        let program = parse("{ let f; { var f; } }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("f"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_sloppy_block_duplicate_function_ok() {
        let program = parse("{ function f() {} function f() {} }").unwrap();
        bind(program).expect("Annex B allows sloppy duplicate plain functions");
    }

    #[test]
    fn bind_strict_block_duplicate_function_errors() {
        let program = parse("\"use strict\"; { function f() {} function f() {} }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("f"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_catch_var_same_name_allowed_annex_b() {
        let program = parse(
            r#"function f() {
                try { throw 1; } catch (e) { var e = 2; return e; }
            }"#,
        )
        .unwrap();
        bind(program).expect("Annex B.3.4 allows var same name as catch param");
    }

    #[test]
    fn bind_catch_let_same_name_errors() {
        let program = parse("try { throw 1; } catch (e) { let e = 2; }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("e"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_catch_function_same_name_errors() {
        let program = parse("try { throw 1; } catch (e) { function e() {} }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("e"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_resolves_call_callee_and_args() {
        let program = parse("let f = 1; let a = 2; f(a);").unwrap();
        let bound = bind(program).unwrap();
        let f_span = find_ident_use(&bound.program, "f");
        let a_span = find_ident_use(&bound.program, "a");
        assert_eq!(bound.symbol(bound.resolve(f_span).unwrap()).name, "f");
        assert_eq!(bound.symbol(bound.resolve(a_span).unwrap()).name, "a");
    }

    #[test]
    fn bind_resolves_arguments_in_function() {
        let program = parse("function f(a) { return arguments.length + arguments[0]; }").unwrap();
        let bound = bind(program).unwrap();
        let args_span = find_ident_use(&bound.program, "arguments");
        let sym = bound.symbol(bound.resolve(args_span).unwrap());
        assert_eq!(sym.name, "arguments");
        assert_eq!(sym.kind, BindingKind::Var);
    }

    #[test]
    fn bind_arguments_free_in_arrow_at_top_level() {
        // Top-level arrow has no `arguments` binding; name stays free (runtime
        // ReferenceError on GetValue / typeof → "undefined").
        let program = parse("let f = () => arguments.length;").unwrap();
        bind(program).expect("free arguments in arrow binds");
        check(parse("let f = () => arguments.length;").unwrap()).expect("check free arguments");
    }
}
