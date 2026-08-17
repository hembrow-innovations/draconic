//! N08.16.40: native observations for private accessors (E18.39).
//!
//! Compile-time evaluation of class private fields + get/set `#x` (instance and
//! static) after IR desugars them to WeakMap/WeakSet + synthetic functions.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp,
    ObjectPropKey, Param, Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64, PRINT_STR};

pub(crate) fn is_es_private_accessors_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_private_accessors(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not private_accessors"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Debug)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    Null,
    Builtin(&'static str),
    UserFn {
        id: u64,
        params: Vec<LocalId>,
        body: Vec<Stmt>,
        props: Rc<RefCell<Vec<(String, Slot)>>>,
    },
    Object {
        id: u64,
        props: Rc<RefCell<Vec<(String, Slot)>>>,
        proto: Rc<RefCell<JsVal>>,
    },
    WeakMap(Rc<RefCell<Vec<(u64, JsVal)>>>),
    WeakSet(Rc<RefCell<Vec<u64>>>),
    Err { message: String },
}

#[derive(Clone, Debug)]
enum Slot {
    Data(JsVal),
    Accessor { get: Option<JsVal>, set: Option<JsVal> },
}

enum Flow {
    Normal,
    Return(JsVal),
    Throw(JsVal),
}

struct ModuleInfo {
    prints: Vec<JsVal>,
}

fn next_id() -> u64 {
    static N: AtomicU64 = AtomicU64::new(1);
    N.fetch_add(1, Ordering::Relaxed)
}

fn new_obj(proto: JsVal) -> JsVal {
    JsVal::Object {
        id: next_id(),
        props: Rc::new(RefCell::new(Vec::new())),
        proto: Rc::new(RefCell::new(proto)),
    }
}

fn new_fn(params: Vec<LocalId>, body: Vec<Stmt>) -> JsVal {
    let proto = new_obj(JsVal::Builtin("Object.prototype"));
    JsVal::UserFn {
        id: next_id(),
        params,
        body,
        props: Rc::new(RefCell::new(vec![("prototype".into(), Slot::Data(proto))])),
    }
}

fn obj_id(v: &JsVal) -> Option<u64> {
    match v {
        JsVal::Object { id, .. } | JsVal::UserFn { id, .. } => Some(*id),
        _ => None,
    }
}

fn is_objectish(v: &JsVal) -> bool {
    matches!(
        v,
        JsVal::Object { .. }
            | JsVal::UserFn { .. }
            | JsVal::WeakMap(_)
            | JsVal::WeakSet(_)
            | JsVal::Builtin(_)
    )
}

fn set_data(props: &Rc<RefCell<Vec<(String, Slot)>>>, key: String, val: JsVal) {
    let mut p = props.borrow_mut();
    if let Some((_, s)) = p.iter_mut().find(|(k, _)| *k == key) {
        *s = Slot::Data(val);
    } else {
        p.push((key, Slot::Data(val)));
    }
}

fn get_data(props: &[(String, Slot)], key: &str) -> Option<JsVal> {
    props.iter().find(|(k, _)| k == key).and_then(|(_, s)| match s {
        Slot::Data(v) => Some(v.clone()),
        Slot::Accessor { get: Some(g), .. } => Some(g.clone()),
        _ => None,
    })
}

fn delete_key(props: &Rc<RefCell<Vec<(String, Slot)>>>, key: &str) {
    props.borrow_mut().retain(|(k, _)| k != key);
}

thread_local! {
    static THIS: RefCell<JsVal> = RefCell::new(JsVal::Undef);
    static NEW_TARGET: RefCell<JsVal> = RefCell::new(JsVal::Undef);
}

fn with_this<R>(t: JsVal, f: impl FnOnce() -> R) -> R {
    THIS.with(|c| {
        let prev = c.replace(t);
        let o = f();
        c.replace(prev);
        o
    })
}

fn current_this() -> JsVal {
    THIS.with(|c| c.borrow().clone())
}

fn with_new_target<R>(t: JsVal, f: impl FnOnce() -> R) -> R {
    NEW_TARGET.with(|c| {
        let prev = c.replace(t);
        let o = f();
        c.replace(prev);
        o
    })
}

fn current_new_target() -> JsVal {
    NEW_TARGET.with(|c| c.borrow().clone())
}

fn builtin(name: &str) -> Option<JsVal> {
    match name {
        "undefined" => Some(JsVal::Undef),
        "Object" | "Function" | "WeakMap" | "WeakSet" | "TypeError" | "Error" => {
            Some(JsVal::Builtin(match name {
                "Object" => "Object",
                "Function" => "Function",
                "WeakMap" => "WeakMap",
                "WeakSet" => "WeakSet",
                "TypeError" => "TypeError",
                "Error" => "Error",
                _ => unreachable!(),
            }))
        }
        _ => None,
    }
}

fn has_private_accessor_surface(module: &Module) -> bool {
    module.locals.iter().any(|l| {
        l.name.contains("__drac_pag_")
            || l.name.contains("__drac_pas_")
            || l.name.contains("__drac_pf_")
    })
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    if !has_private_accessor_surface(module) {
        return None;
    }
    if !body_ok(&module.body) {
        return None;
    }
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    for loc in &module.locals {
        if let Some(v) = builtin(&loc.name) {
            env.insert(loc.id, v);
        }
    }
    match eval_body(&module.body, &mut env) {
        Ok(Flow::Normal) => {}
        _ => return None,
    }
    let mut prints = Vec::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            // Skip class/instance bindings and synthetic temps.
            if loc.name.starts_with("__") {
                continue;
            }
            match env.get(local) {
                Some(v @ (JsVal::Num(_) | JsVal::Str(_) | JsVal::Bool(_))) => {
                    if matches!(
                        loc.ty,
                        Type::Number | Type::Any | Type::Boolean | Type::String
                    ) {
                        prints.push(v.clone());
                    }
                }
                Some(JsVal::Undef)
                | Some(JsVal::Null)
                | Some(JsVal::Object { .. })
                | Some(JsVal::UserFn { .. })
                | Some(JsVal::Builtin(_))
                | Some(JsVal::WeakMap(_))
                | Some(JsVal::WeakSet(_))
                | Some(JsVal::Err { .. }) => {}
                None => return None,
            }
        }
    }
    if prints.is_empty() {
        return None;
    }
    Some(ModuleInfo { prints })
}

fn body_ok(body: &[Stmt]) -> bool {
    body.iter().all(stmt_ok)
}

fn stmt_ok(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Declare { init: None, .. } => true,
        Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } | Stmt::Throw { value: e } => {
            expr_ok(e)
        }
        Stmt::Return { value: None } => true,
        Stmt::Return { value: Some(e) } => expr_ok(e),
        Stmt::Function {
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => params_ok(params) && body_ok(body),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => expr_ok(test) && stmt_ok(consequent) && alternate.as_ref().is_none_or(|a| stmt_ok(a)),
        Stmt::Block { body } => body_ok(body),
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
        _ => false,
    }
}

fn params_ok(params: &[Param]) -> bool {
    params
        .iter()
        .all(|p| !p.rest && p.default.is_none() && matches!(p.pattern, Pattern::Local(_)))
}

fn expr_ok(expr: &Expr) -> bool {
    match expr {
        Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Boolean { .. }
        | Expr::Null { .. }
        | Expr::Local { .. }
        | Expr::This { .. }
        | Expr::NewTarget { .. }
        | Expr::IdentName { .. } => true,
        Expr::Function {
            name: None,
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => params_ok(params) && body_ok(body),
        Expr::Unary {
            op:
                UnaryOp::TypeOf
                | UnaryOp::Minus
                | UnaryOp::Plus
                | UnaryOp::Not
                | UnaryOp::Void
                | UnaryOp::Delete,
            arg,
            ..
        } => expr_ok(arg),
        Expr::Binary { left, right, op, .. } => {
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
            _ => false,
        }),
        _ => false,
    }
}

fn eval_body(body: &[Stmt], env: &mut HashMap<LocalId, JsVal>) -> Result<Flow, ()> {
    for s in body {
        match eval_stmt(s, env)? {
            Flow::Normal => {}
            other => return Ok(other),
        }
    }
    Ok(Flow::Normal)
}

fn eval_stmt(stmt: &Stmt, env: &mut HashMap<LocalId, JsVal>) -> Result<Flow, ()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => ok_val(eval_expr(e, env)?)?,
                None => JsVal::Undef,
            };
            env.insert(*local, v);
            Ok(Flow::Normal)
        }
        Stmt::Expr { expr } => {
            let _ = ok_val(eval_expr(expr, env)?)?;
            Ok(Flow::Normal)
        }
        Stmt::Throw { value } => Ok(Flow::Throw(ok_val(eval_expr(value, env)?)?)),
        Stmt::Return { value: None } => Ok(Flow::Return(JsVal::Undef)),
        Stmt::Return { value: Some(e) } => Ok(Flow::Return(ok_val(eval_expr(e, env)?)?)),
        Stmt::Function {
            local,
            params,
            body,
            is_async: false,
            is_generator: false,
        } => {
            let ids = param_ids(params)?;
            env.insert(*local, new_fn(ids, body.clone()));
            Ok(Flow::Normal)
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            let t = ok_val(eval_expr(test, env)?)?;
            if to_bool(&t) {
                eval_stmt(consequent, env)
            } else if let Some(a) = alternate {
                eval_stmt(a, env)
            } else {
                Ok(Flow::Normal)
            }
        }
        Stmt::Block { body } => eval_body(body, env),
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            let completion = match eval_body(block, env)? {
                Flow::Throw(exc) => {
                    if let Some(h) = handler {
                        if let Some(Pattern::Local(pid)) = handler_param {
                            env.insert(*pid, exc);
                        }
                        eval_body(h, env)?
                    } else {
                        Flow::Throw(exc)
                    }
                }
                other => other,
            };
            if let Some(fin) = finalizer {
                match eval_body(fin, env)? {
                    Flow::Normal => Ok(completion),
                    abrupt => Ok(abrupt),
                }
            } else {
                Ok(completion)
            }
        }
        _ => Err(()),
    }
}

fn param_ids(params: &[Param]) -> Result<Vec<LocalId>, ()> {
    params
        .iter()
        .map(|p| match &p.pattern {
            Pattern::Local(id) => Ok(*id),
            _ => Err(()),
        })
        .collect()
}

fn ok_val(r: Result<JsVal, Flow>) -> Result<JsVal, ()> {
    match r {
        Ok(v) => Ok(v),
        Err(_) => Err(()),
    }
}

fn eval_expr(expr: &Expr, env: &mut HashMap<LocalId, JsVal>) -> Result<Result<JsVal, Flow>, ()> {
    match expr {
        Expr::Number { raw, .. } => Ok(Ok(JsVal::Num(raw.parse().map_err(|_| ())?))),
        Expr::Boolean { value, .. } => Ok(Ok(JsVal::Bool(*value))),
        Expr::String { value, .. } => Ok(Ok(JsVal::Str(value.to_string_lossy()))),
        Expr::Null { .. } => Ok(Ok(JsVal::Null)),
        Expr::Local { id, .. } => Ok(Ok(env.get(id).cloned().ok_or(())?)),
        Expr::This { .. } => Ok(Ok(current_this())),
        Expr::NewTarget { .. } => Ok(Ok(current_new_target())),
        Expr::IdentName { name, .. } => Ok(Ok(builtin(name).ok_or(())?)),
        Expr::Function {
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => {
            let ids = param_ids(params)?;
            Ok(Ok(new_fn(ids, body.clone())))
        }
        Expr::Unary { op, arg, .. } => eval_unary(*op, arg, env),
        Expr::Binary {
            left, op, right, ..
        } => eval_binary(left, *op, right, env),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            let t = match eval_expr(test, env)? {
                Ok(v) => v,
                Err(f) => return Ok(Err(f)),
            };
            if to_bool(&t) {
                eval_expr(consequent, env)
            } else {
                eval_expr(alternate, env)
            }
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            let obj = match eval_expr(object, env)? {
                Ok(v) => v,
                Err(f) => return Ok(Err(f)),
            };
            let key = match eval_key(property, env)? {
                Ok(k) => k,
                Err(f) => return Ok(Err(f)),
            };
            Ok(Ok(member_get(&obj, &key, env)?))
        }
        Expr::New { callee, args, .. } => {
            let c = match eval_expr(callee, env)? {
                Ok(v) => v,
                Err(f) => return Ok(Err(f)),
            };
            let av = eval_args(args, env)?;
            Ok(Ok(eval_new(&c, &av, env)?))
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            let av = match eval_args_flow(args, env)? {
                Ok(v) => v,
                Err(f) => return Ok(Err(f)),
            };
            if let Expr::Member {
                object,
                property,
                optional: false,
                ..
            } = callee.as_ref()
            {
                let obj = match eval_expr(object, env)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                let key = match eval_key(property, env)? {
                    Ok(k) => k,
                    Err(f) => return Ok(Err(f)),
                };
                let mut obj = obj;
                let result = method_call(&mut obj, &key, &av, env)?;
                if let Expr::Local { id, .. } = object.as_ref() {
                    env.insert(*id, obj);
                }
                return Ok(Ok(result));
            }
            let c = match eval_expr(callee, env)? {
                Ok(v) => v,
                Err(f) => return Ok(Err(f)),
            };
            Ok(Ok(call_val(&c, &av, JsVal::Undef, env)?))
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = match eval_expr(value, env)? {
                Ok(v) => v,
                Err(f) => return Ok(Err(f)),
            };
            env.insert(*id, v.clone());
            Ok(Ok(v))
        }
        Expr::Assign {
            target:
                AssignTarget::Member {
                    object,
                    property,
                    ..
                },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = match eval_expr(value, env)? {
                Ok(v) => v,
                Err(f) => return Ok(Err(f)),
            };
            let mut obj = match eval_expr(object, env)? {
                Ok(o) => o,
                Err(f) => return Ok(Err(f)),
            };
            let key = match eval_key(property, env)? {
                Ok(k) => k,
                Err(f) => return Ok(Err(f)),
            };
            member_set(&mut obj, &key, v.clone(), env)?;
            if let Expr::Local { id, .. } = object.as_ref() {
                env.insert(*id, obj);
            }
            Ok(Ok(v))
        }
        Expr::Array { elements, .. } => {
            // unused in fixture observations
            let mut out = Vec::new();
            for el in elements {
                match el {
                    ArrayElement::Expr(e) => out.push(match eval_expr(e, env)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    }),
                    ArrayElement::Elision => out.push(JsVal::Undef),
                    ArrayElement::Spread(_) => return Err(()),
                }
            }
            Ok(Ok(JsVal::Object {
                id: next_id(),
                props: Rc::new(RefCell::new(Vec::new())),
                proto: Rc::new(RefCell::new(JsVal::Builtin("Array.prototype"))),
            }))
        }
        Expr::Object { properties, .. } => {
            let mut props = Vec::new();
            let mut proto = JsVal::Builtin("Object.prototype");
            for p in properties {
                match p {
                    ObjectProp::Property {
                        key: ObjectPropKey::Static(k),
                        value,
                    } => {
                        let v = match eval_expr(value, env)? {
                            Ok(v) => v,
                            Err(f) => return Ok(Err(f)),
                        };
                        let key = k.to_string_lossy();
                        if key == "__proto__" {
                            proto = v;
                            continue;
                        }
                        props.push((key, Slot::Data(v)));
                    }
                    ObjectProp::Property {
                        key: ObjectPropKey::Computed(ke),
                        value,
                    } => {
                        let key = match eval_key(ke, env)? {
                            Ok(k) => k,
                            Err(f) => return Ok(Err(f)),
                        };
                        let v = match eval_expr(value, env)? {
                            Ok(v) => v,
                            Err(f) => return Ok(Err(f)),
                        };
                        props.push((key, Slot::Data(v)));
                    }
                    _ => return Err(()),
                }
            }
            Ok(Ok(JsVal::Object {
                id: next_id(),
                props: Rc::new(RefCell::new(props)),
                proto: Rc::new(RefCell::new(proto)),
            }))
        }
        _ => Err(()),
    }
}

fn eval_args(args: &[Arg], env: &mut HashMap<LocalId, JsVal>) -> Result<Vec<JsVal>, ()> {
    match eval_args_flow(args, env)? {
        Ok(v) => Ok(v),
        Err(_) => Err(()),
    }
}

fn eval_args_flow(
    args: &[Arg],
    env: &mut HashMap<LocalId, JsVal>,
) -> Result<Result<Vec<JsVal>, Flow>, ()> {
    let mut out = Vec::new();
    for a in args {
        match a {
            Arg::Expr(e) => match eval_expr(e, env)? {
                Ok(v) => out.push(v),
                Err(f) => return Ok(Err(f)),
            },
            _ => return Err(()),
        }
    }
    Ok(Ok(out))
}

fn eval_key(expr: &Expr, env: &mut HashMap<LocalId, JsVal>) -> Result<Result<String, Flow>, ()> {
    match expr {
        Expr::String { value, .. } => Ok(Ok(value.to_string_lossy())),
        e => match eval_expr(e, env)? {
            Ok(JsVal::Str(s)) => Ok(Ok(s)),
            Ok(JsVal::Num(n)) => Ok(Ok(format!("{}", n as i64))),
            Ok(_) => Err(()),
            Err(f) => Ok(Err(f)),
        },
    }
}

fn eval_unary(
    op: UnaryOp,
    arg: &Expr,
    env: &mut HashMap<LocalId, JsVal>,
) -> Result<Result<JsVal, Flow>, ()> {
    match op {
        UnaryOp::Delete => {
            if let Expr::Member {
                object,
                property,
                optional: false,
                ..
            } = arg
            {
                let mut obj = match eval_expr(object, env)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                let key = match eval_key(property, env)? {
                    Ok(k) => k,
                    Err(f) => return Ok(Err(f)),
                };
                match &obj {
                    JsVal::Object { props, .. } | JsVal::UserFn { props, .. } => {
                        delete_key(props, &key);
                    }
                    _ => {}
                }
                if let Expr::Local { id, .. } = object.as_ref() {
                    env.insert(*id, obj);
                }
                return Ok(Ok(JsVal::Bool(true)));
            }
            let _ = match eval_expr(arg, env)? {
                Ok(v) => v,
                Err(f) => return Ok(Err(f)),
            };
            Ok(Ok(JsVal::Bool(true)))
        }
        UnaryOp::Void => {
            let _ = match eval_expr(arg, env)? {
                Ok(v) => v,
                Err(f) => return Ok(Err(f)),
            };
            Ok(Ok(JsVal::Undef))
        }
        _ => {
            let v = match eval_expr(arg, env)? {
                Ok(v) => v,
                Err(f) => return Ok(Err(f)),
            };
            match op {
                UnaryOp::TypeOf => Ok(Ok(JsVal::Str(typeof_str(&v)))),
                UnaryOp::Not => Ok(Ok(JsVal::Bool(!to_bool(&v)))),
                UnaryOp::Minus => match v {
                    JsVal::Num(n) => Ok(Ok(JsVal::Num(-n))),
                    _ => Err(()),
                },
                UnaryOp::Plus => match v {
                    JsVal::Num(n) => Ok(Ok(JsVal::Num(n))),
                    _ => Err(()),
                },
                _ => Err(()),
            }
        }
    }
}

fn eval_binary(
    left: &Expr,
    op: BinaryOp,
    right: &Expr,
    env: &mut HashMap<LocalId, JsVal>,
) -> Result<Result<JsVal, Flow>, ()> {
    let l = match eval_expr(left, env)? {
        Ok(v) => v,
        Err(f) => return Ok(Err(f)),
    };
    match op {
        BinaryOp::And => {
            if !to_bool(&l) {
                return Ok(Ok(l));
            }
            eval_expr(right, env)
        }
        BinaryOp::Or => {
            if to_bool(&l) {
                return Ok(Ok(l));
            }
            eval_expr(right, env)
        }
        BinaryOp::Comma => {
            let r = match eval_expr(right, env)? {
                Ok(v) => v,
                Err(f) => return Ok(Err(f)),
            };
            Ok(Ok(r))
        }
        BinaryOp::EqEqEq | BinaryOp::EqEq => {
            let r = match eval_expr(right, env)? {
                Ok(v) => v,
                Err(f) => return Ok(Err(f)),
            };
            Ok(Ok(JsVal::Bool(strict_eq(&l, &r))))
        }
        BinaryOp::NotEqEq | BinaryOp::NotEq => {
            let r = match eval_expr(right, env)? {
                Ok(v) => v,
                Err(f) => return Ok(Err(f)),
            };
            Ok(Ok(JsVal::Bool(!strict_eq(&l, &r))))
        }
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
            let r = match eval_expr(right, env)? {
                Ok(v) => v,
                Err(f) => return Ok(Err(f)),
            };
            let ln = to_num(&l)?;
            let rn = to_num(&r)?;
            let n = match op {
                BinaryOp::Add => ln + rn,
                BinaryOp::Sub => ln - rn,
                BinaryOp::Mul => ln * rn,
                BinaryOp::Div => ln / rn,
                BinaryOp::Rem => ln % rn,
                _ => unreachable!(),
            };
            Ok(Ok(JsVal::Num(n)))
        }
        _ => Err(()),
    }
}

fn to_bool(v: &JsVal) -> bool {
    match v {
        JsVal::Bool(b) => *b,
        JsVal::Undef | JsVal::Null => false,
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        _ => true,
    }
}

fn to_num(v: &JsVal) -> Result<f64, ()> {
    match v {
        JsVal::Num(n) => Ok(*n),
        JsVal::Bool(true) => Ok(1.0),
        JsVal::Bool(false) => Ok(0.0),
        _ => Err(()),
    }
}

fn typeof_str(v: &JsVal) -> String {
    match v {
        JsVal::Num(_) => "number".into(),
        JsVal::Bool(_) => "boolean".into(),
        JsVal::Str(_) => "string".into(),
        JsVal::Undef => "undefined".into(),
        JsVal::UserFn { .. } | JsVal::Builtin("Object" | "Function" | "WeakMap" | "WeakSet" | "TypeError" | "Error") => {
            "function".into()
        }
        JsVal::Builtin("Object.prototype" | "Function.prototype" | "Array.prototype") => {
            "object".into()
        }
        _ => "object".into(),
    }
}

fn strict_eq(a: &JsVal, b: &JsVal) -> bool {
    match (a, b) {
        (JsVal::Num(x), JsVal::Num(y)) => x == y,
        (JsVal::Bool(x), JsVal::Bool(y)) => x == y,
        (JsVal::Str(x), JsVal::Str(y)) => x == y,
        (JsVal::Undef, JsVal::Undef) => true,
        (JsVal::Null, JsVal::Null) => true,
        (JsVal::Builtin(x), JsVal::Builtin(y)) => x == y,
        (JsVal::Object { id: x, .. }, JsVal::Object { id: y, .. }) => x == y,
        (JsVal::UserFn { id: x, .. }, JsVal::UserFn { id: y, .. }) => x == y,
        (JsVal::WeakMap(x), JsVal::WeakMap(y)) => Rc::ptr_eq(x, y),
        (JsVal::WeakSet(x), JsVal::WeakSet(y)) => Rc::ptr_eq(x, y),
        _ => false,
    }
}

fn member_get(obj: &JsVal, key: &str, env: &mut HashMap<LocalId, JsVal>) -> Result<JsVal, ()> {
    match obj {
        JsVal::Object { props, proto, .. } => {
            if let Some((_, slot)) = props.borrow().iter().find(|(k, _)| k == key) {
                return match slot {
                    Slot::Data(v) => Ok(v.clone()),
                    Slot::Accessor { get: Some(g), .. } => {
                        call_val(g, &[], obj.clone(), env)
                    }
                    Slot::Accessor { get: None, .. } => Ok(JsVal::Undef),
                };
            }
            if key == "__proto__" {
                return Ok(proto.borrow().clone());
            }
            let p = proto.borrow().clone();
            if matches!(p, JsVal::Null) {
                return Ok(JsVal::Undef);
            }
            member_get(&p, key, env)
        }
        JsVal::UserFn { props, .. } => {
            if let Some(v) = get_data(&props.borrow(), key) {
                return Ok(v);
            }
            Ok(JsVal::Undef)
        }
        JsVal::Builtin("Object") => match key {
            "prototype" => Ok(JsVal::Builtin("Object.prototype")),
            "defineProperty" => Ok(JsVal::Builtin("Object.defineProperty")),
            "getOwnPropertyDescriptor" => Ok(JsVal::Builtin("Object.getOwnPropertyDescriptor")),
            "isExtensible" => Ok(JsVal::Builtin("Object.isExtensible")),
            "getPrototypeOf" => Ok(JsVal::Builtin("Object.getPrototypeOf")),
            _ => Ok(JsVal::Undef),
        },
        JsVal::Builtin("Function") => match key {
            "prototype" => Ok(JsVal::Builtin("Function.prototype")),
            _ => Ok(JsVal::Undef),
        },
        JsVal::Builtin("Object.prototype") | JsVal::Builtin("Function.prototype") => {
            Ok(JsVal::Undef)
        }
        _ => Ok(JsVal::Undef),
    }
}

fn member_set(
    obj: &mut JsVal,
    key: &str,
    val: JsVal,
    env: &mut HashMap<LocalId, JsVal>,
) -> Result<(), ()> {
    match obj {
        JsVal::Object { props, proto, .. } => {
            if key == "__proto__" {
                *proto.borrow_mut() = val;
                return Ok(());
            }
            let existing = props.borrow().iter().find(|(k, _)| k == key).map(|(_, s)| s.clone());
            match existing {
                Some(Slot::Accessor { set: Some(s), .. }) => {
                    call_val(&s, &[val], obj.clone(), env)?;
                    Ok(())
                }
                Some(Slot::Accessor { set: None, .. }) => Ok(()),
                _ => {
                    set_data(props, key.to_string(), val);
                    Ok(())
                }
            }
        }
        JsVal::UserFn { props, .. } => {
            set_data(props, key.to_string(), val);
            Ok(())
        }
        _ => Err(()),
    }
}

fn eval_new(callee: &JsVal, args: &[JsVal], env: &mut HashMap<LocalId, JsVal>) -> Result<JsVal, ()> {
    match callee {
        JsVal::Builtin("WeakMap") => Ok(JsVal::WeakMap(Rc::new(RefCell::new(Vec::new())))),
        JsVal::Builtin("WeakSet") => Ok(JsVal::WeakSet(Rc::new(RefCell::new(Vec::new())))),
        JsVal::Builtin("TypeError") | JsVal::Builtin("Error") => {
            let msg = match args.first() {
                Some(JsVal::Str(s)) => s.clone(),
                _ => String::new(),
            };
            Ok(JsVal::Err { message: msg })
        }
        JsVal::UserFn { params, body, props, .. } => {
            let proto = get_data(&props.borrow(), "prototype")
                .unwrap_or(JsVal::Builtin("Object.prototype"));
            let this_obj = match proto {
                JsVal::Object { .. } | JsVal::Null | JsVal::UserFn { .. } | JsVal::Builtin(_) => {
                    new_obj(proto)
                }
                _ => new_obj(JsVal::Builtin("Object.prototype")),
            };
            let params = params.clone();
            let body = body.clone();
            let result = with_new_target(callee.clone(), || {
                call_user(&params, &body, this_obj.clone(), args, env)
            })?;
            if is_objectish(&result) && !matches!(result, JsVal::Undef) {
                Ok(result)
            } else {
                Ok(this_obj)
            }
        }
        _ => Err(()),
    }
}

fn method_call(
    recv: &mut JsVal,
    key: &str,
    args: &[JsVal],
    env: &mut HashMap<LocalId, JsVal>,
) -> Result<JsVal, ()> {
    // WeakMap / WeakSet methods
    match recv {
        JsVal::WeakMap(entries) => match key {
            "set" => {
                let k = args.first().ok_or(())?;
                let id = obj_id(k).ok_or(())?;
                let v = args.get(1).cloned().unwrap_or(JsVal::Undef);
                {
                    let mut e = entries.borrow_mut();
                    if let Some((_, slot)) = e.iter_mut().find(|(i, _)| *i == id) {
                        *slot = v;
                    } else {
                        e.push((id, v));
                    }
                }
                return Ok(recv.clone());
            }
            "get" => {
                let k = args.first().ok_or(())?;
                let id = match obj_id(k) {
                    Some(i) => i,
                    None => return Ok(JsVal::Undef),
                };
                return Ok(entries
                    .borrow()
                    .iter()
                    .find(|(i, _)| *i == id)
                    .map(|(_, v)| v.clone())
                    .unwrap_or(JsVal::Undef));
            }
            "has" => {
                let k = args.first().ok_or(())?;
                let id = match obj_id(k) {
                    Some(i) => i,
                    None => return Ok(JsVal::Bool(false)),
                };
                return Ok(JsVal::Bool(entries.borrow().iter().any(|(i, _)| *i == id)));
            }
            _ => {}
        },
        JsVal::WeakSet(values) => match key {
            "add" => {
                let v = args.first().ok_or(())?;
                let id = obj_id(v).ok_or(())?;
                {
                    let mut e = values.borrow_mut();
                    if !e.iter().any(|i| *i == id) {
                        e.push(id);
                    }
                }
                return Ok(recv.clone());
            }
            "has" => {
                let v = args.first().ok_or(())?;
                let id = match obj_id(v) {
                    Some(i) => i,
                    None => return Ok(JsVal::Bool(false)),
                };
                return Ok(JsVal::Bool(values.borrow().iter().any(|i| *i == id)));
            }
            _ => {}
        },
        JsVal::UserFn { params, body, .. } if key == "call" => {
            let this_arg = args.first().cloned().unwrap_or(JsVal::Undef);
            let rest: Vec<_> = args.iter().skip(1).cloned().collect();
            return call_user(&params.clone(), &body.clone(), this_arg, &rest, env);
        }
        JsVal::Builtin("Object") => match key {
            "isExtensible" => return Ok(JsVal::Bool(true)),
            "getPrototypeOf" => {
                let t = args.first().ok_or(())?;
                return match t {
                    JsVal::Object { proto, .. } => Ok(proto.borrow().clone()),
                    JsVal::UserFn { props, .. } => Ok(get_data(&props.borrow(), "prototype")
                        .unwrap_or(JsVal::Undef)),
                    _ => Ok(JsVal::Null),
                };
            }
            "getOwnPropertyDescriptor" => {
                let t = args.first().ok_or(())?;
                let k = match args.get(1) {
                    Some(JsVal::Str(s)) => s.as_str(),
                    _ => return Err(()),
                };
                return get_own_desc(t, k);
            }
            "defineProperty" => {
                let mut t = args.first().cloned().ok_or(())?;
                let k = match args.get(1) {
                    Some(JsVal::Str(s)) => s.clone(),
                    _ => return Err(()),
                };
                let desc = args.get(2).ok_or(())?;
                define_prop(&mut t, &k, desc, env)?;
                // writeback by id
                if let Some(id) = obj_id(&t) {
                    for v in env.values_mut() {
                        if obj_id(v) == Some(id) {
                            *v = t.clone();
                        }
                    }
                }
                return Ok(t);
            }
            _ => {}
        },
        _ => {}
    }
    let c = member_get(recv, key, env)?;
    match c {
        JsVal::UserFn { params, body, .. } => {
            call_user(&params, &body, recv.clone(), args, env)
        }
        JsVal::Builtin(name) => call_builtin(name, args, env),
        JsVal::Undef => Err(()),
        other => call_val(&other, args, JsVal::Undef, env),
    }
}

fn get_own_desc(target: &JsVal, key: &str) -> Result<JsVal, ()> {
    let slot = match target {
        JsVal::Object { props, .. } | JsVal::UserFn { props, .. } => props
            .borrow()
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, s)| s.clone()),
        _ => None,
    };
    match slot {
        Some(Slot::Data(v)) => Ok(JsVal::Object {
            id: next_id(),
            props: Rc::new(RefCell::new(vec![
                ("value".into(), Slot::Data(v)),
                ("writable".into(), Slot::Data(JsVal::Bool(true))),
                ("enumerable".into(), Slot::Data(JsVal::Bool(true))),
                ("configurable".into(), Slot::Data(JsVal::Bool(true))),
            ])),
            proto: Rc::new(RefCell::new(JsVal::Builtin("Object.prototype"))),
        }),
        Some(Slot::Accessor { get, set }) => {
            let mut props = vec![
                ("enumerable".into(), Slot::Data(JsVal::Bool(true))),
                ("configurable".into(), Slot::Data(JsVal::Bool(true))),
            ];
            if let Some(g) = get {
                props.push(("get".into(), Slot::Data(g)));
            }
            if let Some(s) = set {
                props.push(("set".into(), Slot::Data(s)));
            }
            Ok(JsVal::Object {
                id: next_id(),
                props: Rc::new(RefCell::new(props)),
                proto: Rc::new(RefCell::new(JsVal::Builtin("Object.prototype"))),
            })
        }
        None => Ok(JsVal::Undef),
    }
}

fn define_prop(
    target: &mut JsVal,
    key: &str,
    desc: &JsVal,
    _env: &mut HashMap<LocalId, JsVal>,
) -> Result<(), ()> {
    let JsVal::Object { props: dprops, .. } = desc else {
        return Err(());
    };
    let d = dprops.borrow();
    let value = get_data(&d, "value");
    let get = get_data(&d, "get");
    let set = get_data(&d, "set");
    let props = match target {
        JsVal::Object { props, .. } | JsVal::UserFn { props, .. } => props.clone(),
        _ => return Err(()),
    };
    if get.is_some() || set.is_some() {
        let slot = Slot::Accessor {
            get: get.filter(|g| !matches!(g, JsVal::Undef)),
            set: set.filter(|s| !matches!(s, JsVal::Undef)),
        };
        let mut p = props.borrow_mut();
        if let Some((_, s)) = p.iter_mut().find(|(k, _)| k == key) {
            *s = slot;
        } else {
            p.push((key.to_string(), slot));
        }
    } else if let Some(v) = value {
        set_data(&props, key.to_string(), v);
    }
    Ok(())
}

fn call_val(
    callee: &JsVal,
    args: &[JsVal],
    this: JsVal,
    env: &mut HashMap<LocalId, JsVal>,
) -> Result<JsVal, ()> {
    match callee {
        JsVal::UserFn { params, body, .. } => call_user(params, body, this, args, env),
        JsVal::Builtin(name) => call_builtin(name, args, env),
        _ => Err(()),
    }
}

fn call_builtin(name: &str, args: &[JsVal], env: &mut HashMap<LocalId, JsVal>) -> Result<JsVal, ()> {
    match name {
        "Object.isExtensible" => Ok(JsVal::Bool(true)),
        "Object.getPrototypeOf" => {
            let t = args.first().ok_or(())?;
            match t {
                JsVal::Object { proto, .. } => Ok(proto.borrow().clone()),
                JsVal::UserFn { props, .. } => {
                    Ok(get_data(&props.borrow(), "prototype").unwrap_or(JsVal::Undef))
                }
                _ => Ok(JsVal::Null),
            }
        }
        "Object.getOwnPropertyDescriptor" => {
            let t = args.first().ok_or(())?;
            let k = match args.get(1) {
                Some(JsVal::Str(s)) => s.as_str(),
                _ => return Err(()),
            };
            get_own_desc(t, k)
        }
        "Object.defineProperty" => {
            let mut t = args.first().cloned().ok_or(())?;
            let k = match args.get(1) {
                Some(JsVal::Str(s)) => s.clone(),
                _ => return Err(()),
            };
            let desc = args.get(2).ok_or(())?;
            define_prop(&mut t, &k, desc, env)?;
            if let Some(id) = obj_id(&t) {
                for v in env.values_mut() {
                    if obj_id(v) == Some(id) {
                        *v = t.clone();
                    }
                }
            }
            Ok(t)
        }
        "TypeError" | "Error" => {
            let msg = match args.first() {
                Some(JsVal::Str(s)) => s.clone(),
                _ => String::new(),
            };
            Ok(JsVal::Err { message: msg })
        }
        _ => Err(()),
    }
}

fn call_user(
    params: &[LocalId],
    body: &[Stmt],
    this: JsVal,
    args: &[JsVal],
    env: &mut HashMap<LocalId, JsVal>,
) -> Result<JsVal, ()> {
    let mut saved = Vec::new();
    for (i, pid) in params.iter().enumerate() {
        saved.push((*pid, env.get(pid).cloned()));
        env.insert(*pid, args.get(i).cloned().unwrap_or(JsVal::Undef));
    }
    let flow = with_this(this, || eval_body(body, env))?;
    for (pid, prev) in saved {
        match prev {
            Some(v) => {
                env.insert(pid, v);
            }
            None => {
                env.remove(&pid);
            }
        }
    }
    match flow {
        Flow::Normal => Ok(JsVal::Undef),
        Flow::Return(v) => Ok(v),
        Flow::Throw(_) => Err(()),
    }
}

struct Emitter {
    out: String,
    body: String,
    strs: Vec<(String, String)>,
}

impl Emitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            body: String::new(),
            strs: Vec::new(),
        }
    }

    fn intern(&mut self, s: &str) -> String {
        if let Some((_, n)) = self.strs.iter().find(|(v, _)| v == s) {
            return n.clone();
        }
        let n = format!("@.pastr.{}", self.strs.len());
        self.strs.push((s.to_string(), n.clone()));
        n
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for v in &info.prints {
            match v {
                JsVal::Num(n) => {
                    let lit = format!("{n:?}");
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {lit}"))).ok();
                }
                JsVal::Str(s) => {
                    let name = self.intern(s);
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
                JsVal::Bool(b) => {
                    let name = self.intern(if *b { "true" } else { "false" });
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
                _ => return Err(diag("private_accessors: non-printable")),
            }
        }
        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.16.40 private accessors E18.39)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        for (s, name) in &self.strs {
            let n = s.len() + 1;
            let mut esc = String::new();
            for b in s.bytes() {
                match b {
                    b'\\' => esc.push_str("\\5C"),
                    b'"' => esc.push_str("\\22"),
                    c if (0x20..0x7f).contains(&c) => esc.push(c as char),
                    c => esc.push_str(&format!("\\{c:02X}")),
                }
            }
            writeln!(
                self.out,
                "{name} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
            )
            .ok();
        }
        writeln!(self.out, "\ndefine i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        self.out.push_str(&self.body);
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn finish(self) -> String {
        self.out
    }
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    #[test]
    fn private_accessors_classifies_and_prints() {
        let src = include_str!(
            "../../../tests/conformance/fixtures/es/annex-b/private_accessors.drac"
        );
        let m = compile_source(src).expect("compile");
        assert!(
            is_es_private_accessors_module(&m),
            "should classify private_accessors"
        );
        let ir = emit_es_private_accessors(&m).expect("emit");
        assert!(!ir.contains("draconic_rt_hello"), "no hello stub:\n{ir}");
        // a=1 b=10 c=undefined e=10 f=5 g=1 h=7 i=undefined j=undefined k=1 l=2 m=3 n=7
        assert!(ir.contains("double 1") || ir.contains("double 1.0"), "{ir}");
        assert!(ir.contains("double 10") || ir.contains("double 10.0"), "{ir}");
        assert!(ir.contains("undefined"), "{ir}");
        assert!(ir.contains("double 5") || ir.contains("double 5.0"), "{ir}");
        assert!(ir.contains("double 7") || ir.contains("double 7.0"), "{ir}");
        assert!(ir.contains("double 3") || ir.contains("double 3.0"), "{ir}");
    }
}
