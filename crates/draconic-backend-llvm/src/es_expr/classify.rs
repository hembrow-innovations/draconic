use std::collections::HashMap;

use crate::emitter::SlotTy;
use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, Stmt, UpdateTarget,
};

use super::*;

pub(super) fn slot_for_declare(
    local: LocalId,
    init: &Option<Expr>,
    by_id: &HashMap<LocalId, &Local>,
) -> Option<SlotTy> {
    let loc = by_id.get(&local)?;
    match loc.ty {
        Type::Number => {
            if let Some(init) = init {
                if !expr_is_number_subset(init, by_id) {
                    return None;
                }
            }
            Some(SlotTy::Number)
        }
        Type::BigInt => {
            if let Some(init) = init {
                if !expr_is_bigint_subset(init, by_id) {
                    return None;
                }
            }
            Some(SlotTy::BigInt)
        }
        Type::Boolean => {
            if let Some(init) = init {
                if !expr_is_boolean_subset(init, by_id) {
                    return None;
                }
            }
            Some(SlotTy::Boolean)
        }
        Type::String => {
            if let Some(init) = init {
                if !expr_is_string_subset(init, by_id) {
                    return None;
                }
            }
            Some(SlotTy::String)
        }
        Type::Null => {
            if let Some(init) = init {
                if !expr_is_undefined_subset(init, by_id) {
                    return None;
                }
            }
            Some(SlotTy::Undefined)
        }
        // Untyped: string index / for-in-of (string), `.length` (number), Math, or Number (N08.02.08 / N08.07.01 / N08.08.05–06).
        Type::Any => {
            if let Some(init) = init {
                if expr_is_string_length(init, by_id) {
                    return Some(SlotTy::Number);
                }
                if expr_is_math_number(init, by_id) {
                    return Some(SlotTy::Number);
                }
                if expr_is_number_ctor_const(init, by_id) {
                    return Some(SlotTy::Number);
                }
                if expr_is_number_ctor_bool(init, by_id) {
                    return Some(SlotTy::Boolean);
                }
                if expr_is_string_subset(init, by_id) {
                    return Some(SlotTy::String);
                }
                return None;
            }
            Some(SlotTy::String)
        }
        _ => None,
    }
}

pub(super) fn classify(module: &Module) -> Option<ModuleInfo> {
    classify_body(module).filter(|info| !info.user_locals.is_empty())
}

pub(super) fn classify_body(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut user_locals = Vec::new();
    let mut alloc_locals = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for stmt in &module.body {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let slot = slot_for_declare(*local, init, &by_id)?;
                if seen.insert(*local) {
                    user_locals.push((*local, slot));
                    alloc_locals.push((*local, slot));
                }
            }
            Stmt::Expr { .. }
            | Stmt::Block { .. }
            | Stmt::If { .. }
            | Stmt::While { .. }
            | Stmt::DoWhile { .. }
            | Stmt::For { .. }
            | Stmt::ForIn { .. }
            | Stmt::ForOf { .. }
            | Stmt::Switch { .. }
            | Stmt::Labeled { .. } => {
                if !stmt_is_subset(stmt, &by_id) {
                    return None;
                }
                collect_for_init_allocs(stmt, &by_id, &mut alloc_locals, &mut seen)?;
            }
            _ => return None,
        }
    }
    Some(ModuleInfo {
        user_locals,
        alloc_locals,
    })
}

/// Collect `for (let|const x = …)` / `for (let|const k in/of …)` locals into alloc slots (not prints).
pub(super) fn collect_for_init_allocs(
    stmt: &Stmt,
    by_id: &HashMap<LocalId, &Local>,
    alloc_locals: &mut Vec<(LocalId, SlotTy)>,
    seen: &mut std::collections::HashSet<LocalId>,
) -> Option<()> {
    match stmt {
        Stmt::For { init, body, .. } => {
            if let Some(i) = init.as_ref() {
                if let Stmt::Declare { local, init, .. } = i.as_ref() {
                    let slot = slot_for_declare(*local, init, by_id)?;
                    if seen.insert(*local) {
                        alloc_locals.push((*local, slot));
                    }
                } else {
                    collect_for_init_allocs(i, by_id, alloc_locals, seen)?;
                }
            }
            collect_for_init_allocs(body, by_id, alloc_locals, seen)
        }
        Stmt::ForIn { left, body, .. } | Stmt::ForOf { left, body, .. } => {
            if let Stmt::Declare { local, init, .. } = left.as_ref() {
                let slot = slot_for_declare(*local, init, by_id)?;
                if seen.insert(*local) {
                    alloc_locals.push((*local, slot));
                }
            } else {
                collect_for_init_allocs(left, by_id, alloc_locals, seen)?;
            }
            collect_for_init_allocs(body, by_id, alloc_locals, seen)
        }
        Stmt::Block { body } => {
            for s in body {
                collect_for_init_allocs(s, by_id, alloc_locals, seen)?;
            }
            Some(())
        }
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            collect_for_init_allocs(consequent, by_id, alloc_locals, seen)?;
            if let Some(a) = alternate {
                collect_for_init_allocs(a, by_id, alloc_locals, seen)?;
            }
            Some(())
        }
        Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
            collect_for_init_allocs(body, by_id, alloc_locals, seen)
        }
        Stmt::Switch { cases, .. } => {
            for c in cases {
                for s in &c.body {
                    collect_for_init_allocs(s, by_id, alloc_locals, seen)?;
                }
            }
            Some(())
        }
        Stmt::Labeled { body, .. } => collect_for_init_allocs(body, by_id, alloc_locals, seen),
        _ => Some(()),
    }
}

/// Nested statement subset for control-flow bodies and blocks.
/// `for` / `for-in` / `for-of` may introduce nested `let` bindings.
/// Labeled statements + labeled/unlabeled `break`/`continue` (N08.02.07).
/// `for-in`/`for-of` iterate strings only this Loop (N08.02.08).
/// `switch` discriminant and case tests are number subset only.
pub(super) fn stmt_is_subset(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Expr { expr } => match expr.ty() {
            Type::Number => expr_is_number_subset(expr, by_id),
            Type::BigInt => expr_is_bigint_subset(expr, by_id),
            Type::Boolean => expr_is_boolean_subset(expr, by_id),
            Type::String => expr_is_string_subset(expr, by_id),
            Type::Null => expr_is_undefined_subset(expr, by_id),
            // Assignment-form for-in/of left: bare local ref.
            Type::Any => matches!(
                expr,
                Expr::Local { id, .. } if by_id.get(id).is_some_and(|l| l.ty == Type::Any)
            ),
            _ => false,
        },
        Stmt::Block { body } => body.iter().all(|s| stmt_is_subset(s, by_id)),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            (expr_is_boolean_subset(test, by_id) || expr_is_number_subset(test, by_id))
                && stmt_is_subset(consequent, by_id)
                && alternate
                    .as_ref()
                    .map(|a| stmt_is_subset(a, by_id))
                    .unwrap_or(true)
        }
        Stmt::While { test, body } | Stmt::DoWhile { test, body } => {
            (expr_is_boolean_subset(test, by_id) || expr_is_number_subset(test, by_id))
                && stmt_is_subset(body, by_id)
        }
        Stmt::For {
            init,
            test,
            update,
            body,
        } => {
            let init_ok = init
                .as_ref()
                .map(|i| match i.as_ref() {
                    Stmt::Declare { local, init, .. } => {
                        slot_for_declare(*local, init, by_id).is_some()
                    }
                    other => stmt_is_subset(other, by_id),
                })
                .unwrap_or(true);
            let test_ok = test
                .as_ref()
                .map(|t| expr_is_boolean_subset(t, by_id) || expr_is_number_subset(t, by_id))
                .unwrap_or(true);
            let update_ok = update
                .as_ref()
                .map(|u| match u.ty() {
                    Type::Number => expr_is_number_subset(u, by_id),
                    Type::Boolean => expr_is_boolean_subset(u, by_id),
                    Type::String => expr_is_string_subset(u, by_id),
                    Type::Null => expr_is_undefined_subset(u, by_id),
                    _ => false,
                })
                .unwrap_or(true);
            init_ok && test_ok && update_ok && stmt_is_subset(body, by_id)
        }
        Stmt::ForIn { left, right, body } => {
            for_in_of_left_ok(left, by_id)
                && expr_is_string_subset(right, by_id)
                && stmt_is_subset(body, by_id)
        }
        Stmt::ForOf {
            left,
            right,
            body,
            is_await,
        } => {
            !*is_await
                && for_in_of_left_ok(left, by_id)
                && expr_is_string_subset(right, by_id)
                && stmt_is_subset(body, by_id)
        }
        Stmt::Switch {
            discriminant,
            cases,
        } => {
            if !expr_is_number_subset(discriminant, by_id) {
                return false;
            }
            cases.iter().all(|c| {
                let test_ok = c
                    .test
                    .as_ref()
                    .map(|t| expr_is_number_subset(t, by_id))
                    .unwrap_or(true);
                test_ok && c.body.iter().all(|s| stmt_is_subset(s, by_id))
            })
        }
        Stmt::Labeled { body, .. } => stmt_is_subset(body, by_id),
        Stmt::Break { .. } | Stmt::Continue { .. } => true,
        _ => false,
    }
}

pub(super) fn for_in_of_left_ok(left: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match left {
        Stmt::Declare { local, init, .. } => slot_for_declare(*local, init, by_id).is_some(),
        Stmt::Expr {
            expr: Expr::Local { id, ty },
        } => {
            (*ty == Type::Any || *ty == Type::String)
                && by_id
                    .get(id)
                    .is_some_and(|l| l.ty == *ty || l.ty == Type::Any)
        }
        _ => false,
    }
}

/// Operand of `typeof` / `void` / `delete` in the supported subset.
pub(super) fn expr_is_unary_keyword_arg(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Number { ty, .. } => *ty == Type::Number,
        Expr::BigInt { ty, .. } => *ty == Type::BigInt,
        Expr::String { ty, .. } => *ty == Type::String,
        Expr::Boolean { ty, .. } => *ty == Type::Boolean,
        Expr::Null { ty } => *ty == Type::Null,
        Expr::Local { id, ty } => {
            if is_math_global_local(*id, *ty, by_id) {
                return true;
            }
            if is_number_ctor_local(*id, *ty, by_id) {
                return true;
            }
            if is_nan_or_infinity_local(*id, *ty, by_id) {
                return true;
            }
            matches!(
                ty,
                Type::Number | Type::BigInt | Type::String | Type::Boolean | Type::Null | Type::Any
            ) && by_id.get(id).is_some_and(|l| l.ty == *ty)
        }
        e if expr_is_number_subset(e, by_id) => true,
        e if expr_is_bigint_subset(e, by_id) => true,
        e if expr_is_boolean_subset(e, by_id) => true,
        e if expr_is_string_subset(e, by_id) => true,
        e if expr_is_undefined_subset(e, by_id) => true,
        _ => false,
    }
}

/// Global `Math` binding (host builtin local; N08.08.05).
pub(super) fn is_math_global_local(
    id: LocalId,
    ty: Type,
    by_id: &HashMap<LocalId, &Local>,
) -> bool {
    ty == Type::Object
        && by_id
            .get(&id)
            .is_some_and(|l| l.name == "Math" && l.ty == Type::Object)
}

pub(super) fn is_math_global_expr(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Local { id, ty } => is_math_global_local(*id, *ty, by_id),
        _ => false,
    }
}

/// `Math.prop` / `Math["prop"]` → property name when object is the Math global.
pub(super) fn math_member_name(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> Option<String> {
    match expr {
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } if is_math_global_expr(object, by_id) => match property.as_ref() {
            Expr::String { value, .. } => Some(value.to_string_lossy()),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn is_math_const_name(name: &str) -> bool {
    matches!(name, "E" | "PI" | "LN2" | "LOG2E")
}

pub(super) fn is_math_method_name(name: &str) -> bool {
    matches!(
        name,
        "abs" | "floor" | "ceil" | "round" | "min" | "max" | "pow" | "sqrt" | "sign"
    )
}

/// Number-producing `Math` member or call (E08.05 / N08.08.05).
pub(super) fn expr_is_math_number(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    if let Some(name) = math_member_name(expr, by_id) {
        return is_math_const_name(&name);
    }
    match expr {
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            let Some(name) = math_member_name(callee, by_id) else {
                return false;
            };
            if !is_math_method_name(&name) {
                return false;
            }
            let n = args.len();
            let arity_ok = match name.as_str() {
                "min" | "max" => n >= 1,
                "pow" => n == 2,
                _ => n == 1,
            };
            arity_ok
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_is_number_subset(e, by_id),
                    _ => false,
                })
        }
        _ => false,
    }
}

/// Global `Number` constructor binding (host builtin local; N08.08.06).
pub(super) fn is_number_ctor_local(
    id: LocalId,
    ty: Type,
    by_id: &HashMap<LocalId, &Local>,
) -> bool {
    ty == Type::Function
        && by_id
            .get(&id)
            .is_some_and(|l| l.name == "Number" && l.ty == Type::Function)
}

pub(super) fn is_number_ctor_expr(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Local { id, ty } => is_number_ctor_local(*id, *ty, by_id),
        _ => false,
    }
}

/// Global `NaN` / `Infinity` number bindings (host builtins; N08.08.06).
pub(super) fn is_nan_or_infinity_local(
    id: LocalId,
    ty: Type,
    by_id: &HashMap<LocalId, &Local>,
) -> bool {
    ty == Type::Number
        && by_id
            .get(&id)
            .is_some_and(|l| l.ty == Type::Number && (l.name == "NaN" || l.name == "Infinity"))
}

pub(super) fn nan_or_infinity_name(
    id: LocalId,
    by_id: &HashMap<LocalId, &Local>,
) -> Option<&'static str> {
    let l = by_id.get(&id)?;
    if l.ty != Type::Number {
        return None;
    }
    match l.name.as_str() {
        "NaN" => Some("NaN"),
        "Infinity" => Some("Infinity"),
        _ => None,
    }
}

/// `Number.prop` / `Number["prop"]` → property name when object is the Number constructor.
pub(super) fn number_ctor_member_name(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
) -> Option<String> {
    match expr {
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } if is_number_ctor_expr(object, by_id) => match property.as_ref() {
            Expr::String { value, .. } => Some(value.to_string_lossy()),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn is_number_ctor_const_name(name: &str) -> bool {
    matches!(
        name,
        "NaN"
            | "POSITIVE_INFINITY"
            | "NEGATIVE_INFINITY"
            | "MAX_VALUE"
            | "MIN_VALUE"
            | "EPSILON"
            | "MAX_SAFE_INTEGER"
            | "MIN_SAFE_INTEGER"
    )
}

pub(super) fn is_number_ctor_method_name(name: &str) -> bool {
    matches!(name, "isNaN" | "isFinite" | "isInteger" | "isSafeInteger")
}

/// Number-producing `Number.*` constant member (E08.06 / N08.08.06).
pub(super) fn expr_is_number_ctor_const(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    if let Some(name) = number_ctor_member_name(expr, by_id) {
        return is_number_ctor_const_name(&name);
    }
    false
}

/// Boolean-producing `Number.isNaN` / `isFinite` / `isInteger` / `isSafeInteger` call.
pub(super) fn expr_is_number_ctor_bool(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            let Some(name) = number_ctor_member_name(callee, by_id) else {
                return false;
            };
            if !is_number_ctor_method_name(&name) {
                return false;
            }
            args.len() == 1
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_is_number_subset(e, by_id),
                    _ => false,
                })
        }
        _ => false,
    }
}

/// Same-type BigInt subset (E08.02–E08.04 / N08.08.02–N08.08.04): literals, unary `-`/`~`,
/// `+` `-` `*` `/` `%`, bitwise `&` `|` `^` `<<` `>>` (no `>>>`), `**` / `**=`, locals.
/// Values must fit signed i64 at emit; `**` exponents non-negative in fixtures.
pub(super) fn expr_is_bigint_subset(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::BigInt { ty, .. } => *ty == Type::BigInt,
        Expr::Local { id, ty } => {
            *ty == Type::BigInt && by_id.get(id).is_some_and(|l| l.ty == Type::BigInt)
        }
        Expr::Unary { op, arg, ty } => {
            *ty == Type::BigInt
                && matches!(op, UnaryOp::Minus | UnaryOp::BitNot)
                && expr_is_bigint_subset(arg, by_id)
        }
        Expr::Binary {
            left,
            op,
            right,
            ty,
        } => {
            *ty == Type::BigInt
                && matches!(
                    op,
                    BinaryOp::Add
                        | BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Div
                        | BinaryOp::Rem
                        | BinaryOp::BitAnd
                        | BinaryOp::BitOr
                        | BinaryOp::BitXor
                        | BinaryOp::Shl
                        | BinaryOp::Shr
                        | BinaryOp::Pow
                        | BinaryOp::Comma
                )
                && expr_is_bigint_subset(left, by_id)
                && expr_is_bigint_subset(right, by_id)
        }
        Expr::Assign {
            target,
            op,
            value,
            ty,
        } => {
            *ty == Type::BigInt
                && matches!(op, AssignOp::Eq | AssignOp::PowEq)
                && matches!(
                    target,
                    AssignTarget::Local(id) if by_id.get(id).is_some_and(|l| l.ty == Type::BigInt)
                )
                && expr_is_bigint_subset(value, by_id)
        }
        _ => false,
    }
}

pub(super) fn expr_is_string_subset(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::String { ty, .. } => *ty == Type::String,
        // N08.07.02: untagged template → string (cooked quasis + ToString interpolations).
        Expr::Template {
            expressions, ty, ..
        } => *ty == Type::String && expressions.iter().all(|e| expr_is_concat_operand(e, by_id)),
        Expr::Local { id, ty } => {
            (*ty == Type::String
                && by_id
                    .get(id)
                    .is_some_and(|l| l.ty == Type::String || l.ty == Type::Any))
                || (*ty == Type::Any && by_id.get(id).is_some_and(|l| l.ty == Type::Any))
        }
        Expr::Unary { op, arg, ty } => {
            *ty == Type::String
                && matches!(op, UnaryOp::TypeOf)
                && expr_is_unary_keyword_arg(arg, by_id)
        }
        Expr::Binary {
            left,
            op,
            right,
            ty,
        } => {
            *ty == Type::String
                && match op {
                    BinaryOp::Comma => {
                        expr_is_unary_keyword_arg(left, by_id)
                            && expr_is_string_subset(right, by_id)
                    }
                    // String concat (N08.02.08 / N08.07.01); number operand → ToString.
                    BinaryOp::Add => {
                        expr_is_concat_operand(left, by_id)
                            && expr_is_concat_operand(right, by_id)
                            && (expr_is_string_operand(left, by_id)
                                || expr_is_string_operand(right, by_id))
                    }
                    _ => false,
                }
        }
        Expr::Member {
            object,
            property,
            computed,
            optional,
            ty,
        } => {
            !*optional
                && (*ty == Type::String || *ty == Type::Any)
                && expr_is_string_subset(object, by_id)
                && *computed
                && expr_is_number_subset(property, by_id)
        }
        Expr::Assign {
            target,
            op,
            value,
            ty,
        } => {
            *ty == Type::String
                && matches!(op, AssignOp::Eq)
                && matches!(
                    target,
                    AssignTarget::Local(id)
                        if by_id.get(id).is_some_and(|l| {
                            l.ty == Type::String || l.ty == Type::Any
                        })
                )
                && expr_is_string_subset(value, by_id)
        }
        _ => false,
    }
}

/// `s.length` → number (IR types Member as `any`).
pub(super) fn expr_is_string_length(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Member {
            object,
            property,
            computed,
            optional,
            ..
        } => {
            !*optional
                && !*computed
                && expr_is_string_subset(object, by_id)
                && matches!(
                    property.as_ref(),
                    Expr::String { value, .. } if value.to_string_lossy() == "length"
                )
        }
        _ => false,
    }
}

pub(super) fn expr_is_string_operand(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Local { id, ty } if *ty == Type::Any => {
            by_id.get(id).is_some_and(|l| l.ty == Type::Any)
        }
        e => expr_is_string_subset(e, by_id),
    }
}

pub(super) fn expr_is_concat_operand(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    expr_is_string_operand(expr, by_id) || expr_is_number_subset(expr, by_id)
}

pub(super) fn expr_is_undefined_subset(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Local { id, ty } => {
            *ty == Type::Null && by_id.get(id).is_some_and(|l| l.ty == Type::Null)
        }
        Expr::Unary { op, arg, ty } => {
            *ty == Type::Null
                && matches!(op, UnaryOp::Void)
                && expr_is_unary_keyword_arg(arg, by_id)
        }
        Expr::Binary {
            left,
            op,
            right,
            ty,
        } => {
            *ty == Type::Null
                && matches!(op, BinaryOp::Comma)
                && expr_is_unary_keyword_arg(left, by_id)
                && expr_is_undefined_subset(right, by_id)
        }
        Expr::Assign {
            target,
            op,
            value,
            ty,
        } => {
            *ty == Type::Null
                && matches!(op, AssignOp::Eq)
                && matches!(target, AssignTarget::Local(id) if by_id.get(id).is_some_and(|l| l.ty == Type::Null))
                && expr_is_undefined_subset(value, by_id)
        }
        _ => false,
    }
}

pub(super) fn expr_is_number_subset(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Number { ty, .. } => *ty == Type::Number,
        Expr::Local { id, ty } => {
            *ty == Type::Number && by_id.get(id).is_some_and(|l| l.ty == Type::Number)
        }
        // N08.07.01: `s.length` (Member typed `any`).
        e if expr_is_string_length(e, by_id) => true,
        // N08.08.05: `Math.*` constants/methods (Member/Call typed `any`).
        e if expr_is_math_number(e, by_id) => true,
        // N08.08.06: `Number.*` constants (Member typed `any`).
        e if expr_is_number_ctor_const(e, by_id) => true,
        Expr::Unary { op, arg, ty } => {
            *ty == Type::Number
                && matches!(op, UnaryOp::Minus | UnaryOp::Plus | UnaryOp::BitNot)
                && expr_is_number_subset(arg, by_id)
        }
        Expr::Binary {
            left,
            op,
            right,
            ty,
        } => {
            *ty == Type::Number
                && matches!(
                    op,
                    BinaryOp::Add
                        | BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Div
                        | BinaryOp::Rem
                        | BinaryOp::And
                        | BinaryOp::Or
                        | BinaryOp::BitAnd
                        | BinaryOp::BitOr
                        | BinaryOp::BitXor
                        | BinaryOp::Shl
                        | BinaryOp::Shr
                        | BinaryOp::UShr
                        | BinaryOp::Pow
                        | BinaryOp::Comma
                )
                && expr_is_number_subset(left, by_id)
                && expr_is_number_subset(right, by_id)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ty,
        } => {
            *ty == Type::Number
                && (expr_is_boolean_subset(test, by_id) || expr_is_number_subset(test, by_id))
                && expr_is_number_subset(consequent, by_id)
                && expr_is_number_subset(alternate, by_id)
        }
        Expr::Assign {
            target,
            op,
            value,
            ty,
        } => {
            *ty == Type::Number
                && is_number_assign_op(*op)
                && matches!(target, AssignTarget::Local(id) if by_id.get(id).is_some_and(|l| l.ty == Type::Number))
                && expr_is_number_subset(value, by_id)
        }
        Expr::Update { target, ty, .. } => {
            *ty == Type::Number
                && matches!(
                    target,
                    UpdateTarget::Local(id) if by_id.get(id).is_some_and(|l| l.ty == Type::Number)
                )
        }
        _ => false,
    }
}

/// Simple `=` plus numeric compound ops (not logical `&&=`/`||=`/`??=` — N08.01.04.09).
pub(super) fn is_number_assign_op(op: AssignOp) -> bool {
    matches!(
        op,
        AssignOp::Eq
            | AssignOp::AddEq
            | AssignOp::SubEq
            | AssignOp::MulEq
            | AssignOp::DivEq
            | AssignOp::RemEq
            | AssignOp::PowEq
            | AssignOp::ShlEq
            | AssignOp::ShrEq
            | AssignOp::UShrEq
            | AssignOp::BitAndEq
            | AssignOp::BitOrEq
            | AssignOp::BitXorEq
    )
}

pub(super) fn expr_is_boolean_subset(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    // N08.08.06: `Number.isNaN(…)` etc. are typed `any` but produce boolean.
    if expr_is_number_ctor_bool(expr, by_id) {
        return true;
    }
    match expr {
        Expr::Boolean { ty, .. } => *ty == Type::Boolean,
        Expr::Local { id, ty } => {
            *ty == Type::Boolean && by_id.get(id).is_some_and(|l| l.ty == Type::Boolean)
        }
        Expr::Unary { op, arg, ty } => {
            if *ty != Type::Boolean {
                return false;
            }
            match op {
                UnaryOp::Not => expr_is_boolean_subset(arg, by_id),
                // `delete` of a non-reference (literal/expr) is always `true` in non-strict.
                UnaryOp::Delete => expr_is_unary_keyword_arg(arg, by_id),
                _ => false,
            }
        }
        Expr::Binary {
            left,
            op,
            right,
            ty,
        } => {
            if *ty != Type::Boolean {
                return false;
            }
            match op {
                BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                    (expr_is_number_subset(left, by_id) && expr_is_number_subset(right, by_id))
                        || (expr_is_bigint_subset(left, by_id)
                            && expr_is_bigint_subset(right, by_id))
                }
                BinaryOp::EqEq | BinaryOp::NotEq | BinaryOp::EqEqEq | BinaryOp::NotEqEq => {
                    (expr_is_number_subset(left, by_id) && expr_is_number_subset(right, by_id))
                        || (expr_is_bigint_subset(left, by_id)
                            && expr_is_bigint_subset(right, by_id))
                        || (expr_is_boolean_subset(left, by_id)
                            && expr_is_boolean_subset(right, by_id))
                        || (expr_is_string_subset(left, by_id)
                            && expr_is_string_subset(right, by_id))
                }
                BinaryOp::And | BinaryOp::Or => {
                    expr_is_boolean_subset(left, by_id) && expr_is_boolean_subset(right, by_id)
                }
                BinaryOp::Comma => {
                    expr_is_unary_keyword_arg(left, by_id) && expr_is_boolean_subset(right, by_id)
                }
                _ => false,
            }
        }
        Expr::Assign {
            target,
            op,
            value,
            ty,
        } => {
            *ty == Type::Boolean
                && matches!(op, AssignOp::Eq)
                && matches!(target, AssignTarget::Local(id) if by_id.get(id).is_some_and(|l| l.ty == Type::Boolean))
                && expr_is_boolean_subset(value, by_id)
        }
        _ => false,
    }
}
