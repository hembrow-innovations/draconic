use std::collections::HashMap;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp,
    ObjectPropKey, Param, Pattern, Stmt,
};

use super::*;

pub(super) fn module_has_accessor_surface(module: &Module) -> bool {
    fn expr_acc(e: &Expr) -> bool {
        match e {
            Expr::Object { properties, .. } => properties.iter().any(|p| match p {
                ObjectProp::Accessor { .. } => true,
                ObjectProp::Property { value, .. } => expr_acc(value),
                ObjectProp::Spread(s) => expr_acc(s),
            }),
            Expr::Function { body, .. } => body.iter().any(stmt_acc),
            Expr::Unary { arg, .. } => expr_acc(arg),
            Expr::Binary { left, right, .. } => expr_acc(left) || expr_acc(right),
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => expr_acc(test) || expr_acc(consequent) || expr_acc(alternate),
            Expr::Member {
                object, property, ..
            } => expr_acc(object) || expr_acc(property),
            Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
                expr_acc(callee)
                    || args.iter().any(|a| match a {
                        Arg::Expr(e) => expr_acc(e),
                        _ => false,
                    })
            }
            Expr::Assign { value, .. } => expr_acc(value),
            _ => false,
        }
    }
    fn stmt_acc(s: &Stmt) -> bool {
        match s {
            Stmt::Declare { init: Some(e), .. }
            | Stmt::Expr { expr: e }
            | Stmt::Return { value: Some(e) }
            | Stmt::Throw { value: e } => expr_acc(e),
            Stmt::Block { body } => body.iter().any(stmt_acc),
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                expr_acc(test)
                    || stmt_acc(consequent)
                    || alternate.as_ref().is_some_and(|a| stmt_acc(a))
            }
            _ => false,
        }
    }
    module.body.iter().any(stmt_acc)
}

pub(super) fn classify(module: &Module) -> Option<ModuleInfo> {
    if crate::es_classes::walk_es_classes_applies(module) {
        return None;
    }
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    if !module_has_builtin_surface(module, &by_id) && !module_has_accessor_surface(module) {
        return None;
    }
    if !body_ok(&module.body) {
        return None;
    }

    let interp = Interp::new();
    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    for loc in &module.locals {
        if let Some(b) = builtin_for_name(&loc.name) {
            env.insert(
                loc.id,
                match b {
                    BuiltinId::Undefined => JsVal::Undef,
                    BuiltinId::Nan => JsVal::Num(f64::NAN),
                    BuiltinId::Infinity => JsVal::Num(f64::INFINITY),
                    other => JsVal::Builtin(other),
                },
            );
        }
    }

    match interp.eval_body(&module.body, &mut env) {
        Ok(Flow::Normal) => {}
        _ => return None,
    }

    let mut user_locals = Vec::new();
    let mut values = HashMap::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            match env.get(local) {
                Some(v @ (JsVal::Num(_) | JsVal::Str(_) | JsVal::Bool(_) | JsVal::Null)) => {
                    if matches!(
                        loc.ty,
                        Type::Number | Type::Any | Type::Boolean | Type::String | Type::Null
                    ) && is_observe_local_name(&loc.name)
                    {
                        user_locals.push(*local);
                        values.insert(*local, v.clone());
                    }
                }
                Some(
                    JsVal::Undef
                    | JsVal::Builtin(_)
                    | JsVal::ErrorInst { .. }
                    | JsVal::DateInst { .. }
                    | JsVal::RegExpInst { .. }
                    | JsVal::MapInst { .. }
                    | JsVal::SetInst { .. }
                    | JsVal::WeakMapInst { .. }
                    | JsVal::WeakSetInst { .. }
                    | JsVal::ArrayBufferInst { .. }
                    | JsVal::TypedArrayInst { .. }
                    | JsVal::DataViewInst { .. }
                    | JsVal::Array(_)
                    | JsVal::UserFn { .. }
                    | JsVal::Object { .. },
                ) => {}
                None => return None,
            }
        }
    }

    if user_locals.is_empty() {
        return None;
    }

    Some(ModuleInfo {
        user_locals,
        values,
    })
}

pub(super) fn is_observe_local_name(name: &str) -> bool {
    // Skip IR synthetics (`__drac_*`, `__cls_*`, `__class`, …) and non-user bindings.
    !name.starts_with("__")
}

pub(super) fn builtin_for_name(name: &str) -> Option<BuiltinId> {
    match name {
        "undefined" => Some(BuiltinId::Undefined),
        "globalThis" => Some(BuiltinId::GlobalThis),
        "Object" => Some(BuiltinId::Object),
        "Function" => Some(BuiltinId::Function),
        "Array" => Some(BuiltinId::Array),
        "String" => Some(BuiltinId::String),
        "Boolean" => Some(BuiltinId::Boolean),
        "Error" => Some(BuiltinId::Error),
        "TypeError" => Some(BuiltinId::TypeError),
        "RangeError" => Some(BuiltinId::RangeError),
        "ReferenceError" => Some(BuiltinId::ReferenceError),
        "SyntaxError" => Some(BuiltinId::SyntaxError),
        "URIError" => Some(BuiltinId::UriError),
        "EvalError" => Some(BuiltinId::EvalError),
        "AggregateError" => Some(BuiltinId::AggregateError),
        "parseInt" => Some(BuiltinId::ParseInt),
        "parseFloat" => Some(BuiltinId::ParseFloat),
        "isNaN" => Some(BuiltinId::IsNaN),
        "isFinite" => Some(BuiltinId::IsFinite),
        "NaN" => Some(BuiltinId::Nan),
        "Infinity" => Some(BuiltinId::Infinity),
        "encodeURI" => Some(BuiltinId::EncodeUri),
        "decodeURI" => Some(BuiltinId::DecodeUri),
        "encodeURIComponent" => Some(BuiltinId::EncodeUriComponent),
        "decodeURIComponent" => Some(BuiltinId::DecodeUriComponent),
        "escape" => Some(BuiltinId::Escape),
        "unescape" => Some(BuiltinId::Unescape),
        "JSON" => Some(BuiltinId::Json),
        "Date" => Some(BuiltinId::Date),
        "RegExp" => Some(BuiltinId::RegExp),
        "Map" => Some(BuiltinId::Map),
        "Set" => Some(BuiltinId::Set),
        "WeakMap" => Some(BuiltinId::WeakMap),
        "WeakSet" => Some(BuiltinId::WeakSet),
        "ArrayBuffer" => Some(BuiltinId::ArrayBuffer),
        "DataView" => Some(BuiltinId::DataView),
        "Uint8Array" => Some(BuiltinId::Uint8Array),
        "Int32Array" => Some(BuiltinId::Int32Array),
        "Float64Array" => Some(BuiltinId::Float64Array),
        "parseUrl" => Some(BuiltinId::ParseUrl),
        "parseQuery" => Some(BuiltinId::ParseQuery),
        "serializeQuery" => Some(BuiltinId::SerializeQuery),
        "parseFlags" => Some(BuiltinId::ParseFlags),
        "flagHelp" => Some(BuiltinId::FlagHelp),
        _ => None,
    }
}

pub(super) fn module_has_builtin_surface(
    module: &Module,
    by_id: &HashMap<LocalId, &Local>,
) -> bool {
    module
        .body
        .iter()
        .any(|s| stmt_has_builtin_surface(s, by_id))
}

pub(super) fn stmt_has_builtin_surface(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } | Stmt::Throw { value: e } => {
            expr_has_builtin_surface(e, by_id)
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            block.iter().any(|s| stmt_has_builtin_surface(s, by_id))
                || handler
                    .as_ref()
                    .is_some_and(|h| h.iter().any(|s| stmt_has_builtin_surface(s, by_id)))
                || finalizer
                    .as_ref()
                    .is_some_and(|f| f.iter().any(|s| stmt_has_builtin_surface(s, by_id)))
        }
        Stmt::Block { body } => body.iter().any(|s| stmt_has_builtin_surface(s, by_id)),
        _ => false,
    }
}

pub(super) fn expr_has_builtin_surface(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::RegExp { .. } => true,
        Expr::Local { id, .. } => by_id
            .get(id)
            .is_some_and(|l| builtin_for_name(&l.name).is_some()),
        Expr::IdentName { name, .. } => builtin_for_name(name).is_some(),
        Expr::Function { body, .. } => body.iter().any(|s| stmt_has_builtin_surface(s, by_id)),
        Expr::Unary { arg, .. } => expr_has_builtin_surface(arg, by_id),
        Expr::Binary { left, right, .. } => {
            expr_has_builtin_surface(left, by_id) || expr_has_builtin_surface(right, by_id)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_has_builtin_surface(test, by_id)
                || expr_has_builtin_surface(consequent, by_id)
                || expr_has_builtin_surface(alternate, by_id)
        }
        Expr::Member {
            object, property, ..
        } => expr_has_builtin_surface(object, by_id) || expr_has_builtin_surface(property, by_id),
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_has_builtin_surface(callee, by_id)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_builtin_surface(e, by_id),
                    _ => false,
                })
        }
        Expr::Assign { value, .. } => expr_has_builtin_surface(value, by_id),
        Expr::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_has_builtin_surface(e, by_id),
            ArrayElement::Elision => false,
        }),
        Expr::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { key, value } => {
                (match key {
                    ObjectPropKey::Static(_) => false,
                    ObjectPropKey::Computed(e) => expr_has_builtin_surface(e, by_id),
                }) || expr_has_builtin_surface(value, by_id)
            }
            ObjectProp::Accessor { key, value, .. } => {
                (match key {
                    ObjectPropKey::Static(_) => false,
                    ObjectPropKey::Computed(e) => expr_has_builtin_surface(e, by_id),
                }) || expr_has_builtin_surface(value, by_id)
            }
            ObjectProp::Spread(e) => expr_has_builtin_surface(e, by_id),
        }),
        Expr::Null { .. } => false,
        _ => false,
    }
}

pub(super) fn body_ok(body: &[Stmt]) -> bool {
    body.iter().all(stmt_ok)
}

pub(super) fn stmt_ok(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Declare { init, .. } => match init {
            None => true,
            Some(e) => expr_ok(e),
        },
        Stmt::Expr { expr } => expr_ok(expr),
        Stmt::Throw { value } => expr_ok(value),
        Stmt::Return { value: None } => true,
        Stmt::Return { value: Some(e) } => expr_ok(e),
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            match (handler.is_some(), handler_param) {
                (true, None) | (true, Some(Pattern::Local(_))) | (false, None) => {}
                _ => return false,
            }
            body_ok(block)
                && handler.as_ref().is_none_or(|h| body_ok(h))
                && finalizer.as_ref().is_none_or(|f| body_ok(f))
        }
        Stmt::Block { body } => body_ok(body),
        // Class ctor `new.target` check (E18.22 accessors fixture).
        Stmt::If {
            test,
            consequent,
            alternate,
        } => expr_ok(test) && stmt_ok(consequent) && alternate.as_ref().is_none_or(|a| stmt_ok(a)),
        _ => false,
    }
}

pub(super) fn simple_fn_params_ok(params: &[Param]) -> bool {
    params
        .iter()
        .all(|p| !p.rest && p.default.is_none() && matches!(p.pattern, Pattern::Local(_)))
}

pub(super) fn user_fn_from_expr(interp: &Interp, expr: &Expr) -> Option<JsVal> {
    match expr {
        Expr::Function {
            name: None,
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } if simple_fn_params_ok(params) && body_ok(body) => {
            let ids = params
                .iter()
                .map(|p| match &p.pattern {
                    Pattern::Local(id) => *id,
                    _ => unreachable!(),
                })
                .collect();
            Some(new_user_fn(interp, ids, body.clone()))
        }
        _ => None,
    }
}

pub(super) fn expr_ok(expr: &Expr) -> bool {
    match expr {
        Expr::Number { .. } | Expr::String { .. } | Expr::Boolean { .. } | Expr::Null { .. } => {
            true
        }
        Expr::RegExp { .. } => true,
        Expr::Local { .. }
        | Expr::This { .. }
        | Expr::IdentName { .. }
        | Expr::NewTarget { .. } => true,
        Expr::Function {
            name: None,
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => simple_fn_params_ok(params) && body_ok(body),
        Expr::Unary {
            op:
                UnaryOp::TypeOf
                | UnaryOp::Minus
                | UnaryOp::Plus
                | UnaryOp::Void
                | UnaryOp::Delete
                | UnaryOp::Not,
            arg,
            ..
        } => expr_ok(arg),
        Expr::Binary {
            left, right, op, ..
        } => {
            matches!(
                op,
                BinaryOp::EqEqEq
                    | BinaryOp::NotEqEq
                    | BinaryOp::EqEq
                    | BinaryOp::NotEq
                    | BinaryOp::And
                    | BinaryOp::Or
                    | BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem
                    | BinaryOp::Nullish
                    | BinaryOp::Comma
            ) && expr_ok(left)
                && expr_ok(right)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => expr_ok(test) && expr_ok(consequent) && expr_ok(alternate),
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => expr_ok(object) && expr_ok(property),
        Expr::New { callee, args, .. }
        | Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            expr_ok(callee)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_ok(e),
                    _ => false,
                })
        }
        Expr::Assign {
            target: AssignTarget::Local(_),
            op: AssignOp::Eq,
            value,
            ..
        } => expr_ok(value),
        Expr::Assign {
            target: AssignTarget::Member {
                object, property, ..
            },
            op: AssignOp::Eq,
            value,
            ..
        } => expr_ok(object) && expr_ok(property) && expr_ok(value),
        Expr::Array { elements, .. } => elements.iter().all(|el| match el {
            ArrayElement::Expr(e) => expr_ok(e),
            ArrayElement::Elision => true,
            ArrayElement::Spread(_) => false,
        }),
        Expr::Object { properties, .. } => properties.iter().all(|p| match p {
            ObjectProp::Property {
                key: ObjectPropKey::Static(_),
                value,
            } => expr_ok(value),
            ObjectProp::Property {
                key: ObjectPropKey::Computed(k),
                value,
            } => expr_ok(k) && expr_ok(value),
            ObjectProp::Accessor {
                key: ObjectPropKey::Static(_),
                value,
                ..
            } => expr_ok(value),
            ObjectProp::Accessor {
                key: ObjectPropKey::Computed(k),
                value,
                ..
            } => expr_ok(k) && expr_ok(value),
            ObjectProp::Spread(_) => false,
        }),
        _ => false,
    }
}
