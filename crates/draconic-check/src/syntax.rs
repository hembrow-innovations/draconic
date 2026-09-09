use draconic_ast::{
    Arg, ArrayElement, ArrowBody, BindingPattern, Expr, ObjectProp, Param, Stmt, TypeAnn,
};
use draconic_diagnostics::Span;

/// `void` type annotation (keyword parsed as Named "void"); valid only as extern return (F06.02).
pub(crate) fn is_void_type_ann(ann: &TypeAnn) -> bool {
    matches!(ann, TypeAnn::Named { name, .. } if name == "void")
}

/// Whether a labelled item is (or wraps) an iteration statement — needed for
/// `continue label` validity (ECMA-262 LabelledStatement).
pub(crate) fn is_iteration_labelled_item(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::While { .. }
        | Stmt::DoWhile { .. }
        | Stmt::For { .. }
        | Stmt::ForIn { .. }
        | Stmt::ForOf { .. } => true,
        Stmt::Labeled { body, .. } => is_iteration_labelled_item(body),
        _ => false,
    }
}

/// ECMA-262 IsSimpleParameterList: only BindingIdentifiers, no rest/defaults.
pub(crate) fn is_simple_parameter_list(params: &[Param]) -> bool {
    params
        .iter()
        .all(|p| !p.rest && p.default.is_none() && matches!(p.binding, BindingPattern::Ident(_)))
}

/// SuperCall in parameter defaults (E19.39 method early error).
pub(crate) fn params_contain_super_call(params: &[Param]) -> bool {
    params
        .iter()
        .any(|p| p.default.as_ref().is_some_and(expr_contains_super_call))
}

/// SuperCall or SuperProperty in formals (plain / async / generator functions).
pub(crate) fn params_contain_super(params: &[Param]) -> bool {
    params
        .iter()
        .any(|p| p.default.as_ref().is_some_and(expr_contains_super))
}

/// True when `expr` is (or chains from) an OptionalExpression (`?.`).
pub(crate) fn expr_has_optional_chain(expr: &Expr) -> bool {
    match expr {
        Expr::MemberExpression {
            object, optional, ..
        } => *optional || expr_has_optional_chain(object),
        Expr::Call {
            callee, optional, ..
        } => *optional || expr_has_optional_chain(callee),
        Expr::Paren { expr: inner, .. } => expr_has_optional_chain(inner),
        _ => false,
    }
}

pub(crate) fn expr_contains_super_call(expr: &Expr) -> bool {
    match expr {
        Expr::Call { callee, args, .. } => {
            matches!(callee.as_ref(), Expr::Super { .. })
                || expr_contains_super_call(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_super_call(e),
                })
        }
        Expr::ArrowFunction { body, params, .. } => {
            params_contain_super_call(params)
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
            expr_contains_super_call(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_super_call(e),
                })
        }
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_super_call(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, .. } => expr_contains_super_call(value),
            ObjectProp::Spread { expr, .. } => expr_contains_super_call(expr),
            ObjectProp::Accessor { .. } => false,
        }),
        _ => false,
    }
}

/// SuperCall or SuperProperty (not nested in inner functions/classes).
pub(crate) fn expr_contains_super(expr: &Expr) -> bool {
    match expr {
        Expr::Super { .. } => true,
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_contains_super(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_super(e),
                })
        }
        Expr::ArrowFunction { body, params, .. } => {
            params_contain_super(params)
                || match body {
                    ArrowBody::Expr(e) => expr_contains_super(e),
                    ArrowBody::Block(s) => stmt_contains_super(s),
                }
        }
        // Nested function/class bodies are their own ContainsSuper roots.
        Expr::FunctionExpression { .. } | Expr::ClassExpression { .. } => false,
        Expr::Paren { expr: inner, .. }
        | Expr::Unary { arg: inner, .. }
        | Expr::Update { arg: inner, .. }
        | Expr::As { expr: inner, .. } => expr_contains_super(inner),
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => expr_contains_super(left) || expr_contains_super(right),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_super(test)
                || expr_contains_super(consequent)
                || expr_contains_super(alternate)
        }
        Expr::MemberExpression {
            object, property, ..
        } => expr_contains_super(object) || expr_contains_super(property),
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_super(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, .. } => expr_contains_super(value),
            ObjectProp::Spread { expr, .. } => expr_contains_super(expr),
            ObjectProp::Accessor { .. } => false,
        }),
        _ => false,
    }
}

pub(crate) fn stmt_contains_super(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_super),
        Stmt::Expression { expr, .. } => expr_contains_super(expr),
        Stmt::Return { argument, .. } => argument.as_ref().is_some_and(expr_contains_super),
        Stmt::Throw { argument, .. } => expr_contains_super(argument),
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_super(test)
                || stmt_contains_super(consequent)
                || alternate.as_ref().is_some_and(|a| stmt_contains_super(a))
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            expr_contains_super(test) || stmt_contains_super(body)
        }
        Stmt::Let { init, .. } => init.as_ref().is_some_and(expr_contains_super),
        // Nested function/class declarations are separate ContainsSuper roots.
        Stmt::FunctionDeclaration { .. } | Stmt::ClassDeclaration { .. } => false,
        _ => false,
    }
}

pub(crate) fn stmt_contains_super_call(stmt: &Stmt) -> bool {
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

/// `true` when `stmts` begins with a `"use strict"` directive prologue.
pub(crate) fn stmt_list_has_use_strict(stmts: &[Stmt]) -> bool {
    for stmt in stmts {
        match stmt {
            Stmt::Expression {
                expr: Expr::String(s),
                ..
            } => {
                if s.value.to_string_lossy() == "use strict" {
                    return true;
                }
            }
            _ => break,
        }
    }
    false
}

pub(crate) fn body_has_use_strict(body: &Stmt) -> bool {
    match body {
        Stmt::Block { body, .. } => stmt_list_has_use_strict(body),
        _ => false,
    }
}

/// `true` for the literal `true` expression (used by T07.02 loop reachability).
fn is_literal_true(expr: &Expr) -> bool {
    matches!(expr, Expr::Boolean { value: true, .. })
}

/// Whether `stmt` — the body of a loop — contains an unlabeled `break` that would
/// exit that loop, i.e. a `break` not shadowed by an inner loop or switch.
fn loop_body_has_escaping_break(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Block { body, .. } => body.iter().any(loop_body_has_escaping_break),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            loop_body_has_escaping_break(consequent)
                || alternate
                    .as_deref()
                    .is_some_and(loop_body_has_escaping_break)
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            loop_body_has_escaping_break(block)
                || handler.as_deref().is_some_and(loop_body_has_escaping_break)
                || finalizer
                    .as_deref()
                    .is_some_and(loop_body_has_escaping_break)
        }
        Stmt::Labeled { body, .. } => loop_body_has_escaping_break(body),
        // A `break` inside these targets the inner construct, not the outer loop.
        Stmt::While { .. }
        | Stmt::DoWhile { .. }
        | Stmt::For { .. }
        | Stmt::ForIn { .. }
        | Stmt::ForOf { .. }
        | Stmt::Switch { .. } => false,
        Stmt::Break { label, .. } => label.is_none(),
        _ => false,
    }
}

/// Whether control flow can never reach the end of `stmt` (always returns, throws,
/// or loops forever). Conservative toward "terminates" to avoid false positives on
/// valid code; used by the T07.02 missing-return check.
pub(crate) fn stmt_cannot_fall_through(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { .. } | Stmt::Throw { .. } => true,
        Stmt::Block { body, .. } => body.iter().any(stmt_cannot_fall_through),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => match alternate {
            Some(alt) => stmt_cannot_fall_through(consequent) && stmt_cannot_fall_through(alt),
            None => false,
        },
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            is_literal_true(test) && !loop_body_has_escaping_break(body)
        }
        Stmt::For { test, body, .. } => {
            test.as_ref().is_none_or(is_literal_true) && !loop_body_has_escaping_break(body)
        }
        Stmt::Switch { cases, .. } => {
            // A `default` must exist (a non-matching discriminant otherwise exits the
            // switch) and the concatenated case bodies, in source order, must reach a
            // terminating statement (case bodies fall through to the next case).
            if !cases.iter().any(|c| c.test.is_none()) {
                return false;
            }
            cases
                .iter()
                .any(|c| c.body.iter().any(stmt_cannot_fall_through))
        }
        _ => false,
    }
}

pub(crate) fn stmt_span(stmt: &Stmt) -> Span {
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

/// Peel covering parentheses (E19.60 cover IdentifierReference).
pub(crate) fn peel_parens(expr: &Expr) -> &Expr {
    let mut inner = expr;
    while let Expr::Paren { expr, .. } = inner {
        inner = expr.as_ref();
    }
    inner
}

/// The formal parameters of a function expression / arrow value (peeling parens),
/// if `expr` denotes one. Used to record call signatures for function bindings (T07.01).
pub(crate) fn fn_params_of_expr(expr: &Expr) -> Option<&[Param]> {
    match peel_parens(expr) {
        Expr::FunctionExpression { params, .. } | Expr::ArrowFunction { params, .. } => {
            Some(params)
        }
        _ => None,
    }
}

/// Peel parens; if the core is Ident `eval`/`arguments`, return (name, span). E19.49.
pub(crate) fn strict_forbidden_assign_target(expr: &Expr) -> Option<(String, Span)> {
    match peel_parens(expr) {
        Expr::Ident(id) if id.name == "eval" || id.name == "arguments" => {
            Some((id.name.clone(), id.span))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::check;
    use draconic_parser::parse;

    // E19.58: OptionalExpression is not a valid AssignmentTarget / update target.
    #[test]
    fn check_optional_chain_assignment_fails() {
        let program = parse("let o = {}; o?.p = 1;").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("invalid assignment target"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_optional_chain_update_fails() {
        let program = parse("let o = {}; o?.p++;").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("invalid update target"),
            "unexpected: {}",
            err.message
        );
    }

    // E19.58 / E19.82.05: Super in arrows only when lexically nested in Super context.
    #[test]
    fn check_arrow_super_call_fails() {
        // E19.67: super outside method is parse-time SyntaxError.
        assert!(
            parse("() => super();").is_err(),
            "top-level arrow SuperCall must fail at parse"
        );
    }

    #[test]
    fn check_async_arrow_super_call_fails() {
        assert!(
            parse("async () => super();").is_err(),
            "top-level async arrow SuperCall must fail at parse"
        );
    }

    #[test]
    fn check_arrow_super_property_outside_method_fails() {
        assert!(
            parse("() => super.x;").is_err(),
            "top-level arrow SuperProperty must fail at parse"
        );
    }

    #[test]
    fn check_async_arrow_super_property_outside_method_fails() {
        assert!(
            parse("async () => super.x;").is_err(),
            "top-level async arrow SuperProperty must fail at parse"
        );
    }

    #[test]
    fn check_arrow_super_property_in_method_ok() {
        let program =
            parse("class B {} class C extends B { m() { return () => super.x; } }").unwrap();
        check(program).expect("arrow SuperProperty in method must typecheck");
    }

    #[test]
    fn check_arrow_super_call_in_derived_ctor_ok() {
        // E19.82.05: SuperCall in arrow nested in derived constructor is valid.
        let program =
            parse("class B {} class C extends B { constructor() { let f = () => super(); f(); } }")
                .unwrap();
        check(program).expect("arrow SuperCall in derived ctor must typecheck");
    }

    #[test]
    fn check_arrow_super_property_in_field_ok() {
        // E19.82.05: SuperProperty in field initializer arrows is valid.
        let program = parse("class C { f = () => { super.x = 1; }; }").unwrap();
        check(program).expect("arrow SuperProperty in field init must typecheck");
    }

    // --- T07.02: missing return in annotated non-void function ---

    #[test]
    fn check_missing_return_errors() {
        let program = parse("function f(): number { let x = 1; }").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_missing_return_empty_body_errors() {
        let program = parse("function f(): string {}").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_missing_return_if_without_else_errors() {
        let program = parse("function f(x: boolean): number { if (x) { return 1; } }").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_missing_return_shape_errors() {
        let program = parse("function f(): { x: number } {}").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_function_expression_missing_return_errors() {
        let program = parse("let f = function (): number { let x = 1; };").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_arrow_block_missing_return_errors() {
        let program = parse("let f = (): number => { let x = 1; };").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_return_ends_function_ok() {
        let program = parse("function f(): number { return 1; }").unwrap();
        check(program).expect("trailing return should typecheck");
    }

    #[test]
    fn check_return_in_condition_then_tail_ok() {
        let program =
            parse("function f(x: boolean): number { if (x) { return 1; } return 2; }").unwrap();
        check(program).expect("return in if plus trailing return should typecheck");
    }

    #[test]
    fn check_both_if_branches_return_ok() {
        let program =
            parse("function f(x: boolean): number { if (x) { return 1; } else { return 2; } }")
                .unwrap();
        check(program).expect("both if branches returning should typecheck");
    }

    #[test]
    fn check_infinite_loop_ok() {
        let program = parse("function f(): number { while (true) { let x = 1; } }").unwrap();
        check(program).expect("infinite loop should satisfy return type");
    }

    #[test]
    fn check_infinite_loop_with_inner_break_shadowed_ok() {
        let program =
            parse("function f(): number { while (true) { for (;;) { break; } } }").unwrap();
        check(program).expect("inner break still inside nested loop should satisfy return type");
    }

    #[test]
    fn check_infinite_loop_with_escaping_break_errors() {
        let program = parse("function f(): number { while (true) { break; } }").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_throw_only_ok() {
        let program = parse(r#"function f(): number { throw new Error("x"); }"#).unwrap();
        check(program).expect("throw-only body should satisfy return type");
    }

    #[test]
    fn check_any_return_fall_through_ok() {
        let program = parse("function f(): any { let x = 1; }").unwrap();
        check(program).expect("`any` return type should allow fall-off-end");
    }

    #[test]
    fn check_unannotated_function_fall_through_ok() {
        let program = parse("function f() { let x = 1; }").unwrap();
        check(program).expect("unannotated function should allow fall-off-end");
    }

    #[test]
    fn check_switch_all_cases_return_ok() {
        let program = parse(
            r#"
            function f(x: number): number {
              switch (x) {
                case 1: return 1;
                default: return 0;
              }
            }
            "#,
        )
        .unwrap();
        check(program).expect("switch with all cases returning should typecheck");
    }

    #[test]
    fn check_switch_missing_default_errors() {
        let program = parse(
            r#"
            function f(x: number): number {
              switch (x) {
                case 1: return 1;
                case 2: return 2;
              }
            }
            "#,
        )
        .unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "unexpected: {}",
            err.message
        );
    }
}
