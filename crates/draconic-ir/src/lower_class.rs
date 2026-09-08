use draconic_ast::{
    Arg as AstArg, ArrayElement as AstArrayElement, ClassElement, Expr as AstExpr, Ident,
    ObjectProp as AstObjectProp, Stmt as AstStmt, UnaryOp,
};
use draconic_check::{CheckedProgram, Type};
use draconic_diagnostics::Span;

use crate::lower::LowerCtx;
use crate::{BindingKind, Expr, Stmt};

use crate::lower_class_local::lower_class_local;

/// Desugar `class Name extends? Super { constructor… methods… fields… }` to function + assigns.
///
/// E19.57: outer binding is mutable (`let C = …`); methods close over an inner `const` name so
/// reassignment inside the class is a runtime TypeError while `C = null` outside still works.
pub(crate) fn lower_class(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    name: &Ident,
    super_class: Option<&AstExpr>,
    elements: &[ClassElement],
) -> Vec<Stmt> {
    let outer = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.span == name.span)
        .map(|s| s.id)
        .expect("class binding must be declared");
    let inner = ctx.alloc_synthetic_local(format!("__cls_{}", name.name), Type::Function);
    ctx.class_name_remap.push((outer, inner));
    // Pass BindingIdentifier so constructor `.name === "C"` (not `__cls_C` from const).
    // Anonymous `export default class {…}` uses synthetic `__class` → SetFunctionName "default"
    // (E19.82.04 / ClassDefinitionEvaluation className for Default export).
    let name_hint = if name.name == "__class" {
        "default"
    } else {
        name.name.as_str()
    };
    let mut body = lower_class_local(checked, ctx, inner, super_class, elements, Some(name_hint));
    ctx.class_name_remap.pop();
    body.push(Stmt::Return {
        value: Some(Expr::Local {
            id: inner,
            ty: Type::Function,
        }),
    });
    let (needs_yield, needs_await) = class_eval_yield_await(super_class, elements);
    let iife = wrap_class_builder_iife(body, needs_yield, needs_await);
    vec![Stmt::Declare {
        local: outer,
        init: Some(iife),
        kind: BindingKind::Let,
    }]
}

/// Class expression → IIFE that builds the constructor and returns it (E18.33).
///
/// `name_hint` is the NamedEvaluation binding id for anonymous classes
/// (`var cls = class {}` → constructor `.name === "cls"`) (E19.31).
pub(crate) fn lower_class_expression(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    name: Option<&Ident>,
    super_class: Option<&AstExpr>,
    elements: &[ClassElement],
    span: Span,
    name_hint: Option<&str>,
) -> Expr {
    let class_span = name.map(|n| n.span).unwrap_or(span);
    let local = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.span == class_span)
        .map(|s| s.id)
        .expect("class expression binding must be declared");
    // Named classes keep their BindingIdentifier. Anonymous classes always get
    // SetFunctionName: binding hint when present, else "" (ECMA-262 default).
    let named_eval = if name.is_none() {
        Some(name_hint.unwrap_or(""))
    } else {
        None
    };
    let mut body = lower_class_local(checked, ctx, local, super_class, elements, named_eval);
    body.push(Stmt::Return {
        value: Some(Expr::Local {
            id: local,
            ty: Type::Function,
        }),
    });
    let (needs_yield, needs_await) = class_eval_yield_await(super_class, elements);
    wrap_class_builder_iife(body, needs_yield, needs_await)
}

/// Whether ClassDefinitionEvaluation evaluates `yield` / `await` (E19.78).
/// Computed keys, extends, static field inits, and static blocks run at class eval time.
pub(crate) fn class_eval_yield_await(
    super_class: Option<&AstExpr>,
    elements: &[ClassElement],
) -> (bool, bool) {
    let mut needs_yield = super_class.is_some_and(ast_has_yield);
    let mut needs_await = super_class.is_some_and(ast_has_await);
    for el in elements {
        match el {
            ClassElement::Method { key, .. } | ClassElement::Accessor { key, .. } => {
                needs_yield |= object_key_has_yield(key);
                needs_await |= object_key_has_await(key);
            }
            ClassElement::Field {
                key,
                value,
                is_static,
                ..
            } => {
                needs_yield |= object_key_has_yield(key);
                needs_await |= object_key_has_await(key);
                if *is_static {
                    if let Some(v) = value {
                        needs_yield |= ast_has_yield(v);
                        needs_await |= ast_has_await(v);
                    }
                }
            }
            ClassElement::StaticBlock { body, .. } => {
                needs_await |= stmt_has_await(body);
            }
            ClassElement::Constructor { .. } => {}
        }
        if needs_yield && needs_await {
            break;
        }
    }
    (needs_yield, needs_await)
}

/// Build class IIFE, preserving outer `yield`/`await` via `yield*` / `await` (E19.78).
pub(crate) fn wrap_class_builder_iife(
    body: Vec<Stmt>,
    needs_yield: bool,
    needs_await: bool,
) -> Expr {
    // Class code is strict (ECMA-262 §15.7). Inject a directive at the top of the
    // builder so every function defined within (ctor, methods, accessors, static
    // blocks, private helpers) is strict by containment — this is what keeps
    // non-simple-param class functions strict, since they cannot carry their own
    // directive (E19.87). Simple params: a directive is always legal here.
    let mut body = body;
    body.insert(
        0,
        Stmt::Expr {
            expr: Expr::String {
                value: "use strict".into(),
                ty: Type::String,
            },
        },
    );
    let call = Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: Vec::new(),
            body,
            is_async: needs_await && !needs_yield,
            is_generator: needs_yield,
            is_arrow: false,
            is_method: false,
            ty: Type::Function,
        }),
        args: Vec::new(),
        optional: false,
        ty: Type::Function,
    };
    if needs_yield {
        Expr::Unary {
            op: UnaryOp::YieldStar,
            arg: Box::new(call),
            ty: Type::Function,
        }
    } else if needs_await {
        Expr::Unary {
            op: UnaryOp::Await,
            arg: Box::new(call),
            ty: Type::Function,
        }
    } else {
        call
    }
}

pub(crate) fn object_key_has_yield(key: &draconic_ast::ObjectKey) -> bool {
    matches!(key, draconic_ast::ObjectKey::Computed(e) if ast_has_yield(e))
}

pub(crate) fn object_key_has_await(key: &draconic_ast::ObjectKey) -> bool {
    matches!(key, draconic_ast::ObjectKey::Computed(e) if ast_has_await(e))
}

/// True if `expr` evaluates a `yield`/`yield*` in the current function (not nested fn/class).
pub(crate) fn ast_has_yield(expr: &AstExpr) -> bool {
    match expr {
        AstExpr::Unary {
            op: UnaryOp::Yield | UnaryOp::YieldStar,
            ..
        } => true,
        AstExpr::FunctionExpression { .. } | AstExpr::ClassExpression { .. } => false,
        AstExpr::Unary { arg, .. }
        | AstExpr::Paren { expr: arg, .. }
        | AstExpr::As { expr: arg, .. }
        | AstExpr::Update { arg, .. } => ast_has_yield(arg),
        AstExpr::Binary { left, right, .. }
        | AstExpr::Assign {
            target: left,
            value: right,
            ..
        } => ast_has_yield(left) || ast_has_yield(right),
        AstExpr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => ast_has_yield(test) || ast_has_yield(consequent) || ast_has_yield(alternate),
        AstExpr::Call { callee, args, .. } | AstExpr::New { callee, args, .. } => {
            ast_has_yield(callee)
                || args.iter().any(|a| match a {
                    AstArg::Expr(e) | AstArg::Spread(e) => ast_has_yield(e),
                })
        }
        AstExpr::MemberExpression {
            object, property, ..
        } => ast_has_yield(object) || ast_has_yield(property),
        AstExpr::PrivateIn { object, .. } => ast_has_yield(object),
        AstExpr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            AstArrayElement::Expr(e) | AstArrayElement::Spread(e) => ast_has_yield(e),
            AstArrayElement::Elision => false,
        }),
        AstExpr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            AstObjectProp::Property { key, value, .. } => {
                object_key_has_yield(key) || ast_has_yield(value)
            }
            AstObjectProp::Accessor { key, body, .. } => {
                object_key_has_yield(key) || stmt_has_yield(body)
            }
            AstObjectProp::Spread { expr, .. } => ast_has_yield(expr),
        }),
        AstExpr::ArrowFunction { body, params, .. } => {
            // Arrow may contain yield only in defaults (illegal in generator params separately).
            params
                .iter()
                .any(|p| p.default.as_ref().is_some_and(ast_has_yield))
                || match body {
                    draconic_ast::ArrowBody::Expr(e) => ast_has_yield(e),
                    draconic_ast::ArrowBody::Block(s) => stmt_has_yield(s),
                }
        }
        AstExpr::TemplateLiteral { expressions, .. } => expressions.iter().any(ast_has_yield),
        AstExpr::TaggedTemplate {
            tag, expressions, ..
        } => ast_has_yield(tag) || expressions.iter().any(ast_has_yield),
        AstExpr::ImportCall {
            source, options, ..
        } => ast_has_yield(source) || options.as_ref().is_some_and(|o| ast_has_yield(o)),
        AstExpr::ArrayPattern { .. } | AstExpr::ObjectPattern { .. } => false,
        _ => false,
    }
}

pub(crate) fn stmt_has_yield(stmt: &AstStmt) -> bool {
    match stmt {
        AstStmt::Block { body, .. } => body.iter().any(stmt_has_yield),
        AstStmt::Expression { expr, .. } => ast_has_yield(expr),
        AstStmt::Return {
            argument: Some(e), ..
        }
        | AstStmt::Throw { argument: e, .. } => ast_has_yield(e),
        AstStmt::Let { init: Some(e), .. } => ast_has_yield(e),
        AstStmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            ast_has_yield(test)
                || stmt_has_yield(consequent)
                || alternate.as_ref().is_some_and(|a| stmt_has_yield(a))
        }
        AstStmt::While { test, body, .. } => ast_has_yield(test) || stmt_has_yield(body),
        AstStmt::DoWhile { body, test, .. } => stmt_has_yield(body) || ast_has_yield(test),
        AstStmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            init.as_ref().is_some_and(|s| stmt_has_yield(s))
                || test.as_ref().is_some_and(ast_has_yield)
                || update.as_ref().is_some_and(ast_has_yield)
                || stmt_has_yield(body)
        }
        AstStmt::ForIn {
            left, right, body, ..
        }
        | AstStmt::ForOf {
            left, right, body, ..
        } => stmt_has_yield(left) || ast_has_yield(right) || stmt_has_yield(body),
        AstStmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            stmt_has_yield(block)
                || handler.as_ref().is_some_and(|h| stmt_has_yield(h))
                || finalizer.as_ref().is_some_and(|f| stmt_has_yield(f))
        }
        AstStmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            ast_has_yield(discriminant)
                || cases.iter().any(|c| {
                    c.test.as_ref().is_some_and(ast_has_yield) || c.body.iter().any(stmt_has_yield)
                })
        }
        AstStmt::Labeled { body, .. } => stmt_has_yield(body),
        AstStmt::With { object, body, .. } => ast_has_yield(object) || stmt_has_yield(body),
        _ => false,
    }
}

/// True if `expr` evaluates `await` in the current async/module context (not nested async fn).
pub(crate) fn ast_has_await(expr: &AstExpr) -> bool {
    match expr {
        AstExpr::Unary {
            op: UnaryOp::Await, ..
        } => true,
        AstExpr::FunctionExpression { .. } | AstExpr::ClassExpression { .. } => false,
        AstExpr::ArrowFunction { is_async: true, .. } => false,
        AstExpr::Unary { arg, .. }
        | AstExpr::Paren { expr: arg, .. }
        | AstExpr::As { expr: arg, .. }
        | AstExpr::Update { arg, .. } => ast_has_await(arg),
        AstExpr::Binary { left, right, .. }
        | AstExpr::Assign {
            target: left,
            value: right,
            ..
        } => ast_has_await(left) || ast_has_await(right),
        AstExpr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => ast_has_await(test) || ast_has_await(consequent) || ast_has_await(alternate),
        AstExpr::Call { callee, args, .. } | AstExpr::New { callee, args, .. } => {
            ast_has_await(callee)
                || args.iter().any(|a| match a {
                    AstArg::Expr(e) | AstArg::Spread(e) => ast_has_await(e),
                })
        }
        AstExpr::MemberExpression {
            object, property, ..
        } => ast_has_await(object) || ast_has_await(property),
        AstExpr::PrivateIn { object, .. } => ast_has_await(object),
        AstExpr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            AstArrayElement::Expr(e) | AstArrayElement::Spread(e) => ast_has_await(e),
            AstArrayElement::Elision => false,
        }),
        AstExpr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            AstObjectProp::Property { key, value, .. } => {
                object_key_has_await(key) || ast_has_await(value)
            }
            AstObjectProp::Accessor { key, body, .. } => {
                object_key_has_await(key) || stmt_has_await(body)
            }
            AstObjectProp::Spread { expr, .. } => ast_has_await(expr),
        }),
        AstExpr::ArrowFunction { body, params, .. } => {
            params
                .iter()
                .any(|p| p.default.as_ref().is_some_and(ast_has_await))
                || match body {
                    draconic_ast::ArrowBody::Expr(e) => ast_has_await(e),
                    draconic_ast::ArrowBody::Block(s) => stmt_has_await(s),
                }
        }
        AstExpr::TemplateLiteral { expressions, .. } => expressions.iter().any(ast_has_await),
        AstExpr::TaggedTemplate {
            tag, expressions, ..
        } => ast_has_await(tag) || expressions.iter().any(ast_has_await),
        AstExpr::ImportCall {
            source, options, ..
        } => ast_has_await(source) || options.as_ref().is_some_and(|o| ast_has_await(o)),
        _ => false,
    }
}

pub(crate) fn stmt_has_await(stmt: &AstStmt) -> bool {
    match stmt {
        AstStmt::Block { body, .. } => body.iter().any(stmt_has_await),
        AstStmt::Expression { expr, .. } => ast_has_await(expr),
        AstStmt::Return {
            argument: Some(e), ..
        }
        | AstStmt::Throw { argument: e, .. } => ast_has_await(e),
        AstStmt::Let { init: Some(e), .. } => ast_has_await(e),
        AstStmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            ast_has_await(test)
                || stmt_has_await(consequent)
                || alternate.as_ref().is_some_and(|a| stmt_has_await(a))
        }
        AstStmt::While { test, body, .. } => ast_has_await(test) || stmt_has_await(body),
        AstStmt::DoWhile { body, test, .. } => stmt_has_await(body) || ast_has_await(test),
        AstStmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            init.as_ref().is_some_and(|s| stmt_has_await(s))
                || test.as_ref().is_some_and(ast_has_await)
                || update.as_ref().is_some_and(ast_has_await)
                || stmt_has_await(body)
        }
        AstStmt::ForIn {
            left, right, body, ..
        }
        | AstStmt::ForOf {
            left, right, body, ..
        } => stmt_has_await(left) || ast_has_await(right) || stmt_has_await(body),
        AstStmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            stmt_has_await(block)
                || handler.as_ref().is_some_and(|h| stmt_has_await(h))
                || finalizer.as_ref().is_some_and(|f| stmt_has_await(f))
        }
        AstStmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            ast_has_await(discriminant)
                || cases.iter().any(|c| {
                    c.test.as_ref().is_some_and(ast_has_await) || c.body.iter().any(stmt_has_await)
                })
        }
        AstStmt::Labeled { body, .. } => stmt_has_await(body),
        AstStmt::With { object, body, .. } => ast_has_await(object) || stmt_has_await(body),
        _ => false,
    }
}
