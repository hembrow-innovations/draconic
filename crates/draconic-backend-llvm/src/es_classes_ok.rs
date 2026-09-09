use std::collections::HashMap;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_ir::{Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Stmt};

use super::{ClassInfo, FnInfo, MethodRet};

pub(super) fn method_body_ok(body: &[Stmt], by_id: &HashMap<LocalId, &Local>) -> bool {
    body.iter().all(|s| method_stmt_ok(s, by_id))
}

fn method_stmt_ok(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Return { value: None } => true,
        Stmt::Return { value: Some(e) } => {
            number_expr_ok_method(e, by_id) || string_expr_ok_method(e, by_id)
        }
        Stmt::Block { body } => body.iter().all(|s| method_stmt_ok(s, by_id)),
        Stmt::Expr {
            expr:
                Expr::Call {
                    callee: c,
                    args,
                    optional,
                    ..
                },
        } if matches!(c.as_ref(), Expr::Super { .. }) && !*optional => {
            args.iter().all(|a| match a {
                Arg::Expr(e) => number_expr_ok_method(e, by_id),
                Arg::Spread(_) => false,
            })
        }
        Stmt::Expr {
            expr:
                Expr::Assign {
                    target:
                        AssignTarget::Member {
                            object, property, ..
                        },
                    op: AssignOp::Eq,
                    value,
                    ..
                },
        } => {
            matches!(object.as_ref(), Expr::This { .. })
                && matches!(property.as_ref(), Expr::String { .. })
                && (number_expr_ok_method(value, by_id) || is_undefined_expr(value))
        }
        _ => false,
    }
}

pub(super) fn method_ret_kind(body: &[Stmt]) -> MethodRet {
    for s in body {
        if let Stmt::Return { value: Some(e) } = s {
            if string_expr_ok_method(e, &HashMap::new()) {
                return MethodRet::String;
            }
        }
    }
    MethodRet::Number
}

fn string_expr_ok_method(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            number_expr_ok_method(arg, by_id)
                || matches!(arg.as_ref(), Expr::Member { .. })
                || matches!(arg.as_ref(), Expr::Local { .. })
        }
        Expr::String { .. } => true,
        _ => false,
    }
}

pub(super) fn is_undefined_expr(expr: &Expr) -> bool {
    match expr {
        Expr::IdentName { name, .. } if name == "undefined" => true,
        Expr::Unary {
            op: UnaryOp::Void, ..
        } => true,
        _ => false,
    }
}

fn number_expr_ok_method(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { id, ty } => {
            matches!(ty, Type::Number | Type::Any)
                || by_id
                    .get(id)
                    .is_some_and(|l| matches!(l.ty, Type::Number | Type::Any))
        }
        Expr::This { .. } => false,
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            !*optional
                && matches!(object.as_ref(), Expr::This { .. })
                && matches!(property.as_ref(), Expr::String { .. })
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            !*optional
                && (super_method_callee_ok(callee) || this_method_callee_ok(callee))
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => number_expr_ok_method(e, by_id),
                    Arg::Spread(_) => false,
                })
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            matches!(
                op,
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
            ) && number_expr_ok_method(left, by_id)
                && number_expr_ok_method(right, by_id)
        }
        Expr::Assign {
            target: AssignTarget::Member {
                object, property, ..
            },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            matches!(object.as_ref(), Expr::This { .. })
                && matches!(property.as_ref(), Expr::String { .. })
                && number_expr_ok_method(value, by_id)
        }
        _ => false,
    }
}

/// `super.m` / `super["m"]` as call callee (IR keeps bare Super in methods).
fn super_method_callee_ok(callee: &Expr) -> bool {
    match callee {
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            !*optional
                && matches!(object.as_ref(), Expr::Super { .. })
                && matches!(property.as_ref(), Expr::String { .. })
        }
        _ => false,
    }
}

/// `this.m(...)` call callee in methods (e.g. Child.total → this.base()).
fn this_method_callee_ok(callee: &Expr) -> bool {
    match callee {
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            !*optional
                && matches!(object.as_ref(), Expr::This { .. })
                && matches!(property.as_ref(), Expr::String { .. })
        }
        _ => false,
    }
}

pub(super) fn is_object_slot(
    init: &Expr,
    class_of: &HashMap<LocalId, usize>,
    by_id: &HashMap<LocalId, &Local>,
) -> bool {
    match init {
        Expr::New { .. } => true,
        Expr::Local { id, ty } => {
            class_of.contains_key(id)
                || matches!(ty, Type::Object | Type::Function)
                || by_id.get(id).is_some_and(|l| {
                    class_of.contains_key(id)
                        || matches!(l.ty, Type::Object | Type::Function | Type::Shape(_))
                })
        }
        // Member reads of instance props are numbers in class_basic fixtures
        // (typed `any`); do not treat as object slots.
        _ => false,
    }
}

pub(super) fn object_expr_ok(
    expr: &Expr,
    class_of: &HashMap<LocalId, usize>,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    classes: &[ClassInfo],
) -> bool {
    match expr {
        Expr::This { .. } => true,
        Expr::New { callee, args, .. } => {
            let Expr::Local { id, .. } = callee.as_ref() else {
                return false;
            };
            if !class_of.contains_key(id) {
                return false;
            }
            args.iter().all(|a| match a {
                Arg::Expr(e) => number_expr_ok(e, class_of, by_id, functions, classes),
                Arg::Spread(_) => false,
            })
        }
        Expr::Local { id, .. } => {
            class_of.contains_key(id)
                || by_id.get(id).is_some_and(|l| {
                    matches!(
                        l.ty,
                        Type::Object | Type::Function | Type::Any | Type::Shape(_)
                    )
                })
        }
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            !*optional
                && object_expr_ok(object, class_of, by_id, functions, classes)
                && matches!(property.as_ref(), Expr::String { .. })
        }
        _ => false,
    }
}

pub(super) fn number_expr_ok(
    expr: &Expr,
    class_of: &HashMap<LocalId, usize>,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    classes: &[ClassInfo],
) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { id, ty } => {
            matches!(ty, Type::Number | Type::Any)
                || by_id
                    .get(id)
                    .is_some_and(|l| matches!(l.ty, Type::Number | Type::Any))
        }
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            !*optional
                && object_expr_ok(object, class_of, by_id, functions, classes)
                && matches!(property.as_ref(), Expr::String { .. })
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            !*optional
                && method_callee_ok(callee, class_of, by_id, functions, classes)
                && method_call_returns_number(callee, classes, functions)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => number_expr_ok(e, class_of, by_id, functions, classes),
                    Arg::Spread(_) => false,
                })
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            matches!(
                op,
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
            ) && number_expr_ok(left, class_of, by_id, functions, classes)
                && number_expr_ok(right, class_of, by_id, functions, classes)
        }
        Expr::New { .. } => false,
        _ => false,
    }
}

pub(super) fn typeof_string_expr_ok(
    expr: &Expr,
    class_of: &HashMap<LocalId, usize>,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    classes: &[ClassInfo],
) -> bool {
    match expr {
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            // `typeof obj.missing` / `typeof` of values.
            matches!(
                arg.as_ref(),
                Expr::Member {
                    optional: false,
                    ..
                }
            ) || object_expr_ok(arg, class_of, by_id, functions, classes)
                || number_expr_ok(arg, class_of, by_id, functions, classes)
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            !*optional
                && method_callee_ok(callee, class_of, by_id, functions, classes)
                && method_call_returns_string(callee, classes, functions)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => number_expr_ok(e, class_of, by_id, functions, classes),
                    Arg::Spread(_) => false,
                })
        }
        Expr::String { .. } => true,
        _ => false,
    }
}

fn method_call_returns_number(callee: &Expr, classes: &[ClassInfo], functions: &[FnInfo]) -> bool {
    match method_call_ret(callee, classes, functions) {
        Some(MethodRet::String) => false,
        _ => true,
    }
}

fn method_call_returns_string(callee: &Expr, classes: &[ClassInfo], functions: &[FnInfo]) -> bool {
    method_call_ret(callee, classes, functions) == Some(MethodRet::String)
}

fn method_call_ret(
    callee: &Expr,
    classes: &[ClassInfo],
    functions: &[FnInfo],
) -> Option<MethodRet> {
    let Expr::Member { property, .. } = callee else {
        return None;
    };
    let Expr::String { value, .. } = property.as_ref() else {
        return None;
    };
    let name = value.to_string_lossy();
    let mut found: Option<MethodRet> = None;
    for c in classes {
        for (mname, idx) in c.methods.iter().chain(c.static_methods.iter()) {
            if *mname == name {
                let r = functions[*idx].ret;
                if let Some(prev) = found {
                    if prev != r {
                        return None;
                    }
                }
                found = Some(r);
            }
        }
    }
    found
}

fn method_callee_ok(
    callee: &Expr,
    class_of: &HashMap<LocalId, usize>,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    classes: &[ClassInfo],
) -> bool {
    match callee {
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            !*optional
                && object_expr_ok(object, class_of, by_id, functions, classes)
                && matches!(property.as_ref(), Expr::String { .. })
        }
        _ => false,
    }
}

pub(super) fn side_effect_ok(
    expr: &Expr,
    class_of: &HashMap<LocalId, usize>,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    classes: &[ClassInfo],
) -> bool {
    match expr {
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            !*optional
                && method_callee_ok(callee, class_of, by_id, functions, classes)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => number_expr_ok(e, class_of, by_id, functions, classes),
                    Arg::Spread(_) => false,
                })
        }
        _ => false,
    }
}
