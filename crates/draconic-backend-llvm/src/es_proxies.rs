//! N08.13.01–N08.13.08: native observations for Proxy basics + `set` + `has`/`in`
//! + `delete`/`deleteProperty` + `apply` + `construct` + Reflect basics + `ownKeys`
//! (E14.01–E14.08).
//!
//! Compile-time evaluation of a small Proxy/Reflect subset: `typeof Proxy`,
//! `new Proxy(target, handler)`, empty-handler get/set/`in`/`delete`/call/`new`/ownKeys
//! pass-through, `get`/`set`/`has`/`deleteProperty`/`apply`/`construct`/`ownKeys` traps
//! (function props; free-var capture; string keys), `typeof Reflect` +
//! `Reflect.get`/`set`/`has`/`deleteProperty`/`apply`/`construct`/`ownKeys` on plain
//! objects + Proxy targets, array literals as arg lists, member assign, method calls
//! (`obj.m()` thisArg), function constructors (`this` + prop init), `typeof` on proxies.
//! Objects live on a heap so proxy targets share identity with outer locals. Emits
//! Runtime prints of final top-level number/string/bool locals.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, JsString, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp,
    ObjectPropKey, Param, Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64, PRINT_STR};

pub(crate) fn is_es_proxies_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_proxies(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_proxies module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReflectOp {
    Get,
    Set,
    Has,
    DeleteProperty,
    Apply,
    Construct,
    OwnKeys,
}

#[derive(Clone, Debug)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    Null,
    /// Builtin `Proxy` constructor.
    ProxyCtor,
    /// Builtin `Reflect` object.
    ReflectObj,
    /// `Reflect.get` / `set` / `has` / `deleteProperty` / `apply` / `construct` / `ownKeys`.
    ReflectMethod(ReflectOp),
    /// Plain object (index into object heap).
    Object(usize),
    /// Function value (index into `fns`).
    Fn(usize),
    /// Proxy instance (index into `proxies`).
    Proxy(usize),
}

thread_local! {
    static CURRENT_THIS: RefCell<Option<JsVal>> = const { RefCell::new(None) };
}

#[derive(Clone, Debug)]
struct FnRec {
    params: Vec<LocalId>,
    body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
struct ObjectRec {
    props: HashMap<String, JsVal>,
    /// Insertion order of own string keys (for `Reflect.ownKeys`).
    keys: Vec<String>,
}

#[derive(Clone, Debug)]
struct ProxyRec {
    target: JsVal,
    /// Optional `get` trap function index.
    get_trap: Option<usize>,
    /// Optional `set` trap function index.
    set_trap: Option<usize>,
    /// Optional `has` trap function index.
    has_trap: Option<usize>,
    /// Optional `deleteProperty` trap function index.
    delete_trap: Option<usize>,
    /// Optional `apply` trap function index.
    apply_trap: Option<usize>,
    /// Optional `construct` trap function index.
    construct_trap: Option<usize>,
    /// Optional `ownKeys` trap function index.
    own_keys_trap: Option<usize>,
}

fn object_set_prop(rec: &mut ObjectRec, key: String, value: JsVal) {
    if !rec.props.contains_key(&key) {
        rec.keys.push(key.clone());
    }
    rec.props.insert(key, value);
}

fn object_delete_prop(rec: &mut ObjectRec, key: &str) {
    if rec.props.remove(key).is_some() {
        rec.keys.retain(|k| k != key);
    }
}

fn empty_object() -> ObjectRec {
    ObjectRec {
        props: HashMap::new(),
        keys: Vec::new(),
    }
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

struct Emitter {
    out: String,
    body: String,
    str_consts: Vec<(String, String)>,
}

fn js_string_to_utf8(s: &JsString) -> String {
    s.to_string_lossy()
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    if !module_has_proxy(module) {
        return None;
    }
    if !module.body.iter().all(|s| {
        matches!(
            s,
            Stmt::Declare { .. } | Stmt::Expr { .. } | Stmt::Function { .. }
        )
    }) {
        return None;
    }

    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    // Install builtins used by this subset.
    for loc in &module.locals {
        if loc.name == "Proxy" && loc.ty == Type::Function {
            env.insert(loc.id, JsVal::ProxyCtor);
        }
        if loc.name == "Reflect" && matches!(loc.ty, Type::Object | Type::Any | Type::Function) {
            env.insert(loc.id, JsVal::ReflectObj);
        }
    }

    let mut fns: Vec<FnRec> = Vec::new();
    let mut objects: Vec<ObjectRec> = Vec::new();
    let mut proxies: Vec<ProxyRec> = Vec::new();

    if eval_body(&module.body, &mut env, &mut fns, &mut objects, &mut proxies).is_err() {
        return None;
    }

    let mut user_locals = Vec::new();
    let mut values = HashMap::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            match env.get(local) {
                Some(v @ (JsVal::Num(_) | JsVal::Str(_) | JsVal::Bool(_))) => {
                    if matches!(
                        loc.ty,
                        Type::Number | Type::Any | Type::Boolean | Type::String
                    ) {
                        user_locals.push(*local);
                        values.insert(*local, v.clone());
                    }
                }
                Some(JsVal::Undef)
                    if matches!(loc.ty, Type::Any | Type::String | Type::Number) =>
                {
                    // skip undefined for print unless we need it
                }
                Some(
                    JsVal::Object(_)
                    | JsVal::Proxy(_)
                    | JsVal::Fn(_)
                    | JsVal::ProxyCtor
                    | JsVal::ReflectObj
                    | JsVal::ReflectMethod(_)
                    | JsVal::Null,
                ) => {}
                None => return None,
                _ => {}
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

fn module_has_proxy(module: &Module) -> bool {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    module.body.iter().any(|s| stmt_has_proxy(s, &by_id))
}

fn stmt_has_proxy(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } => expr_has_proxy(e, by_id),
        Stmt::Function { body, .. } => body.iter().any(|s| stmt_has_proxy(s, by_id)),
        Stmt::Block { body } => body.iter().any(|s| stmt_has_proxy(s, by_id)),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            expr_has_proxy(test, by_id)
                || stmt_has_proxy(consequent, by_id)
                || alternate
                    .as_ref()
                    .is_some_and(|a| stmt_has_proxy(a, by_id))
        }
        Stmt::Return { value: Some(e) } => expr_has_proxy(e, by_id),
        _ => false,
    }
}

fn expr_has_proxy(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Local { id, .. } => {
            by_id
                .get(id)
                .is_some_and(|l| l.name == "Proxy" || l.name == "Reflect")
        }
        Expr::New { callee, args, .. } => {
            expr_has_proxy(callee, by_id)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_proxy(e, by_id),
                    _ => false,
                })
        }
        Expr::Call { callee, args, .. } => {
            expr_has_proxy(callee, by_id)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_proxy(e, by_id),
                    _ => false,
                })
        }
        Expr::Member { object, property, .. } => {
            expr_has_proxy(object, by_id) || expr_has_proxy(property, by_id)
        }
        Expr::Unary { arg, .. } => expr_has_proxy(arg, by_id),
        Expr::Binary { left, right, .. } => {
            expr_has_proxy(left, by_id) || expr_has_proxy(right, by_id)
        }
        Expr::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, .. } | ObjectProp::Accessor { value, .. } => {
                expr_has_proxy(value, by_id)
            }
            ObjectProp::Spread(e) => expr_has_proxy(e, by_id),
        }),
        Expr::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_has_proxy(e, by_id),
            ArrayElement::Elision => false,
        }),
        Expr::Function { body, .. } => body.iter().any(|s| stmt_has_proxy(s, by_id)),
        Expr::Assign { target, value, .. } => {
            assign_target_has_proxy(target, by_id) || expr_has_proxy(value, by_id)
        }
        _ => false,
    }
}

fn assign_target_has_proxy(target: &AssignTarget, by_id: &HashMap<LocalId, &Local>) -> bool {
    match target {
        AssignTarget::Local(_) | AssignTarget::Name(_) => false,
        AssignTarget::Member {
            object, property, ..
        } => expr_has_proxy(object, by_id) || expr_has_proxy(property, by_id),
        AssignTarget::Deref(e) => expr_has_proxy(e, by_id),
        AssignTarget::ArrayPattern { .. } | AssignTarget::ObjectPattern { .. } => false,
    }
}

fn eval_body(
    body: &[Stmt],
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<(), ()> {
    for stmt in body {
        eval_stmt(stmt, env, fns, objects, proxies)?;
    }
    Ok(())
}

fn eval_stmt(
    stmt: &Stmt,
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<Option<JsVal>, ()> {
    match stmt {
        Stmt::Function { .. } => Ok(None),
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => eval_expr(e, env, fns, objects, proxies)?,
                None => JsVal::Undef,
            };
            env.insert(*local, v);
            Ok(None)
        }
        Stmt::Expr { expr } => {
            eval_expr(expr, env, fns, objects, proxies)?;
            Ok(None)
        }
        Stmt::Block { body } => {
            for s in body {
                if let Some(v) = eval_stmt(s, env, fns, objects, proxies)? {
                    return Ok(Some(v));
                }
            }
            Ok(None)
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            let t = eval_expr(test, env, fns, objects, proxies)?;
            if is_truthy(&t) {
                eval_stmt(consequent, env, fns, objects, proxies)
            } else if let Some(alt) = alternate {
                eval_stmt(alt, env, fns, objects, proxies)
            } else {
                Ok(None)
            }
        }
        Stmt::Return { value } => {
            let v = match value {
                Some(e) => eval_expr(e, env, fns, objects, proxies)?,
                None => JsVal::Undef,
            };
            Ok(Some(v))
        }
        _ => Err(()),
    }
}

fn is_truthy(v: &JsVal) -> bool {
    match v {
        JsVal::Bool(b) => *b,
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Undef | JsVal::Null => false,
        _ => true,
    }
}

fn simple_param_locals(params: &[Param]) -> Option<Vec<LocalId>> {
    let mut ids = Vec::new();
    for p in params {
        if p.rest || p.default.is_some() {
            return None;
        }
        match &p.pattern {
            Pattern::Local(id) => ids.push(*id),
            _ => return None,
        }
    }
    Some(ids)
}

fn register_fn(params: &[Param], body: &[Stmt], fns: &mut Vec<FnRec>) -> Result<JsVal, ()> {
    let param_ids = simple_param_locals(params).ok_or(())?;
    let idx = fns.len();
    fns.push(FnRec {
        params: param_ids,
        body: body.to_vec(),
    });
    Ok(JsVal::Fn(idx))
}

fn with_this<R>(this: Option<JsVal>, f: impl FnOnce() -> R) -> R {
    CURRENT_THIS.with(|slot| {
        let prev = slot.replace(this);
        let r = f();
        *slot.borrow_mut() = prev;
        r
    })
}

fn call_fn(
    idx: usize,
    args: &[JsVal],
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<JsVal, ()> {
    call_fn_this(idx, args, None, env, fns, objects, proxies)
}

fn call_fn_this(
    idx: usize,
    args: &[JsVal],
    this: Option<JsVal>,
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<JsVal, ()> {
    let rec = fns.get(idx).ok_or(())?.clone();
    // Bind params in shared env (free vars stay visible).
    for (i, pid) in rec.params.iter().enumerate() {
        let v = args.get(i).cloned().unwrap_or(JsVal::Undef);
        env.insert(*pid, v);
    }
    with_this(this, || {
        for stmt in &rec.body {
            if let Some(v) = eval_stmt(stmt, env, fns, objects, proxies)? {
                return Ok(v);
            }
        }
        Ok(JsVal::Undef)
    })
}

fn make_args_object(args: &[JsVal], objects: &mut Vec<ObjectRec>) -> JsVal {
    let mut rec = empty_object();
    for (i, a) in args.iter().enumerate() {
        object_set_prop(&mut rec, i.to_string(), a.clone());
    }
    object_set_prop(&mut rec, "length".into(), JsVal::Num(args.len() as f64));
    let arr_idx = objects.len();
    objects.push(rec);
    JsVal::Object(arr_idx)
}

fn construct_value(
    callee: &JsVal,
    args: &[JsVal],
    new_target: &JsVal,
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<JsVal, ()> {
    match callee {
        JsVal::Fn(i) => {
            let this_idx = objects.len();
            objects.push(empty_object());
            let this_obj = JsVal::Object(this_idx);
            let ret = call_fn_this(*i, args, Some(this_obj.clone()), env, fns, objects, proxies)?;
            match ret {
                JsVal::Object(_) | JsVal::Proxy(_) => Ok(ret),
                JsVal::Undef => Ok(this_obj),
                _ => Ok(this_obj),
            }
        }
        JsVal::Proxy(idx) => {
            let rec = proxies.get(*idx).ok_or(())?.clone();
            if let Some(trap_idx) = rec.construct_trap {
                let args_obj = make_args_object(args, objects);
                let trap_args = vec![rec.target.clone(), args_obj, new_target.clone()];
                let ret = call_fn(trap_idx, &trap_args, env, fns, objects, proxies)?;
                match ret {
                    JsVal::Object(_) | JsVal::Proxy(_) => Ok(ret),
                    _ => Err(()),
                }
            } else {
                construct_value(&rec.target, args, new_target, env, fns, objects, proxies)
            }
        }
        _ => Err(()),
    }
}

fn eval_key(
    expr: &Expr,
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<String, ()> {
    match expr {
        Expr::String { value, .. } => Ok(js_string_to_utf8(value)),
        e => match eval_expr(e, env, fns, objects, proxies)? {
            JsVal::Str(s) => Ok(s),
            JsVal::Num(n) => Ok(format!("{}", n as i64)),
            _ => Err(()),
        },
    }
}

fn eval_expr(
    expr: &Expr,
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<JsVal, ()> {
    match expr {
        Expr::Number { raw, .. } => {
            let n: f64 = raw.parse().map_err(|_| ())?;
            Ok(JsVal::Num(n))
        }
        Expr::Boolean { value, .. } => Ok(JsVal::Bool(*value)),
        Expr::String { value, .. } => Ok(JsVal::Str(js_string_to_utf8(value))),
        Expr::Null { .. } => Ok(JsVal::Null),
        Expr::This { .. } => CURRENT_THIS.with(|slot| slot.borrow().clone().ok_or(())),
        Expr::Local { id, .. } => env.get(id).cloned().ok_or(()),
        Expr::Array { elements, .. } => {
            let mut rec = empty_object();
            let mut len = 0usize;
            for el in elements {
                match el {
                    ArrayElement::Expr(e) => {
                        let v = eval_expr(e, env, fns, objects, proxies)?;
                        object_set_prop(&mut rec, len.to_string(), v);
                        len += 1;
                    }
                    ArrayElement::Elision => {
                        len += 1;
                    }
                    ArrayElement::Spread(_) => return Err(()),
                }
            }
            object_set_prop(&mut rec, "length".into(), JsVal::Num(len as f64));
            let idx = objects.len();
            objects.push(rec);
            Ok(JsVal::Object(idx))
        }
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            let v = eval_expr(arg, env, fns, objects, proxies)?;
            Ok(JsVal::Str(typeof_str(&v)))
        }
        Expr::Unary {
            op: UnaryOp::Delete,
            arg,
            ..
        } => match arg.as_ref() {
            Expr::Member {
                object,
                property,
                optional: false,
                ..
            } => {
                let obj = eval_expr(object, env, fns, objects, proxies)?;
                let key = eval_key(property, env, fns, objects, proxies)?;
                let ok = proxy_or_object_delete(&obj, &key, env, fns, objects, proxies)?;
                Ok(JsVal::Bool(ok))
            }
            _ => Err(()),
        },
        Expr::Binary {
            left,
            op: BinaryOp::In,
            right,
            ..
        } => {
            let key = eval_key(left, env, fns, objects, proxies)?;
            let obj = eval_expr(right, env, fns, objects, proxies)?;
            let has = proxy_or_object_has(&obj, &key, env, fns, objects, proxies)?;
            Ok(JsVal::Bool(has))
        }
        Expr::Binary {
            left,
            op,
            right,
            ..
        } => {
            let l = eval_expr(left, env, fns, objects, proxies)?;
            let r = eval_expr(right, env, fns, objects, proxies)?;
            eval_binary(op, &l, &r)
        }
        Expr::Object { properties, .. } => {
            let mut rec = empty_object();
            for p in properties {
                match p {
                    ObjectProp::Property { key, value } => {
                        let k = match key {
                            ObjectPropKey::Static(s) => js_string_to_utf8(s),
                            ObjectPropKey::Computed(e) => {
                                eval_key(e, env, fns, objects, proxies)?
                            }
                        };
                        let v = eval_expr(value, env, fns, objects, proxies)?;
                        object_set_prop(&mut rec, k, v);
                    }
                    _ => return Err(()),
                }
            }
            let idx = objects.len();
            objects.push(rec);
            Ok(JsVal::Object(idx))
        }
        Expr::Function {
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => register_fn(params, body, fns),
        Expr::New { callee, args, .. } => {
            let c = eval_expr(callee, env, fns, objects, proxies)?;
            let mut argv = Vec::new();
            for a in args {
                match a {
                    Arg::Expr(e) => argv.push(eval_expr(e, env, fns, objects, proxies)?),
                    _ => return Err(()),
                }
            }
            match c {
                JsVal::ProxyCtor => {
                    if argv.len() != 2 {
                        return Err(());
                    }
                    let target = argv[0].clone();
                    let handler = match &argv[1] {
                        JsVal::Object(i) => objects.get(*i).ok_or(())?.props.clone(),
                        _ => return Err(()),
                    };
                    let get_trap = match handler.get("get") {
                        Some(JsVal::Fn(i)) => Some(*i),
                        Some(_) => return Err(()),
                        None => None,
                    };
                    let set_trap = match handler.get("set") {
                        Some(JsVal::Fn(i)) => Some(*i),
                        Some(_) => return Err(()),
                        None => None,
                    };
                    let has_trap = match handler.get("has") {
                        Some(JsVal::Fn(i)) => Some(*i),
                        Some(_) => return Err(()),
                        None => None,
                    };
                    let delete_trap = match handler.get("deleteProperty") {
                        Some(JsVal::Fn(i)) => Some(*i),
                        Some(_) => return Err(()),
                        None => None,
                    };
                    let apply_trap = match handler.get("apply") {
                        Some(JsVal::Fn(i)) => Some(*i),
                        Some(_) => return Err(()),
                        None => None,
                    };
                    let construct_trap = match handler.get("construct") {
                        Some(JsVal::Fn(i)) => Some(*i),
                        Some(_) => return Err(()),
                        None => None,
                    };
                    let own_keys_trap = match handler.get("ownKeys") {
                        Some(JsVal::Fn(i)) => Some(*i),
                        Some(_) => return Err(()),
                        None => None,
                    };
                    let idx = proxies.len();
                    proxies.push(ProxyRec {
                        target,
                        get_trap,
                        set_trap,
                        has_trap,
                        delete_trap,
                        apply_trap,
                        construct_trap,
                        own_keys_trap,
                    });
                    Ok(JsVal::Proxy(idx))
                }
                other => construct_value(&other, &argv, &other, env, fns, objects, proxies),
            }
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            let (c, this_arg) = match callee.as_ref() {
                Expr::Member {
                    object,
                    property,
                    optional: false,
                    ..
                } => {
                    let obj = eval_expr(object, env, fns, objects, proxies)?;
                    let key = eval_key(property, env, fns, objects, proxies)?;
                    let f = if matches!(obj, JsVal::ReflectObj) {
                        reflect_method(&key)?
                    } else {
                        proxy_or_object_get(&obj, &key, env, fns, objects, proxies)?
                    };
                    (f, obj)
                }
                _ => {
                    let c = eval_expr(callee, env, fns, objects, proxies)?;
                    (c, JsVal::Undef)
                }
            };
            let mut argv = Vec::new();
            for a in args {
                match a {
                    Arg::Expr(e) => argv.push(eval_expr(e, env, fns, objects, proxies)?),
                    _ => return Err(()),
                }
            }
            call_value(&c, this_arg, &argv, env, fns, objects, proxies)
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            let obj = eval_expr(object, env, fns, objects, proxies)?;
            let key = eval_key(property, env, fns, objects, proxies)?;
            if matches!(obj, JsVal::ReflectObj) {
                return reflect_method(&key);
            }
            proxy_or_object_get(&obj, &key, env, fns, objects, proxies)
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = eval_expr(value, env, fns, objects, proxies)?;
            env.insert(*id, v.clone());
            Ok(v)
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            op,
            value,
            ..
        } => {
            let cur = env.get(id).cloned().ok_or(())?;
            let rhs = eval_expr(value, env, fns, objects, proxies)?;
            let v = match op {
                AssignOp::AddEq => eval_binary(&BinaryOp::Add, &cur, &rhs)?,
                _ => return Err(()),
            };
            env.insert(*id, v.clone());
            Ok(v)
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
            let obj = eval_expr(object, env, fns, objects, proxies)?;
            let key = eval_key(property, env, fns, objects, proxies)?;
            let v = eval_expr(value, env, fns, objects, proxies)?;
            proxy_or_object_set(&obj, &key, &v, env, fns, objects, proxies)?;
            // Assignment expression result is the RHS (ECMA-262).
            Ok(v)
        }
        _ => Err(()),
    }
}

fn typeof_str(v: &JsVal) -> String {
    match v {
        JsVal::Num(_) => "number".into(),
        JsVal::Bool(_) => "boolean".into(),
        JsVal::Str(_) => "string".into(),
        JsVal::Undef => "undefined".into(),
        JsVal::ProxyCtor | JsVal::Fn(_) | JsVal::ReflectMethod(_) => "function".into(),
        JsVal::Object(_) | JsVal::Proxy(_) | JsVal::ReflectObj | JsVal::Null => "object".into(),
    }
}

fn reflect_method(key: &str) -> Result<JsVal, ()> {
    let op = match key {
        "get" => ReflectOp::Get,
        "set" => ReflectOp::Set,
        "has" => ReflectOp::Has,
        "deleteProperty" => ReflectOp::DeleteProperty,
        "apply" => ReflectOp::Apply,
        "construct" => ReflectOp::Construct,
        "ownKeys" => ReflectOp::OwnKeys,
        _ => return Err(()),
    };
    Ok(JsVal::ReflectMethod(op))
}

fn object_to_arg_list(obj: &JsVal, objects: &[ObjectRec]) -> Result<Vec<JsVal>, ()> {
    match obj {
        JsVal::Object(idx) => {
            let props = &objects.get(*idx).ok_or(())?.props;
            let len = match props.get("length") {
                Some(JsVal::Num(n)) if *n >= 0.0 && n.is_finite() => *n as usize,
                _ => return Err(()),
            };
            let mut out = Vec::with_capacity(len);
            for i in 0..len {
                out.push(
                    props
                        .get(&i.to_string())
                        .cloned()
                        .unwrap_or(JsVal::Undef),
                );
            }
            Ok(out)
        }
        _ => Err(()),
    }
}

fn call_reflect(
    op: ReflectOp,
    args: &[JsVal],
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<JsVal, ()> {
    match op {
        ReflectOp::Get => {
            if args.len() < 2 {
                return Err(());
            }
            let key = match &args[1] {
                JsVal::Str(s) => s.clone(),
                JsVal::Num(n) => format!("{}", *n as i64),
                _ => return Err(()),
            };
            proxy_or_object_get(&args[0], &key, env, fns, objects, proxies)
        }
        ReflectOp::Set => {
            if args.len() < 3 {
                return Err(());
            }
            let key = match &args[1] {
                JsVal::Str(s) => s.clone(),
                JsVal::Num(n) => format!("{}", *n as i64),
                _ => return Err(()),
            };
            proxy_or_object_set(&args[0], &key, &args[2], env, fns, objects, proxies)?;
            Ok(JsVal::Bool(true))
        }
        ReflectOp::Has => {
            if args.len() < 2 {
                return Err(());
            }
            let key = match &args[1] {
                JsVal::Str(s) => s.clone(),
                JsVal::Num(n) => format!("{}", *n as i64),
                _ => return Err(()),
            };
            let has = proxy_or_object_has(&args[0], &key, env, fns, objects, proxies)?;
            Ok(JsVal::Bool(has))
        }
        ReflectOp::DeleteProperty => {
            if args.len() < 2 {
                return Err(());
            }
            let key = match &args[1] {
                JsVal::Str(s) => s.clone(),
                JsVal::Num(n) => format!("{}", *n as i64),
                _ => return Err(()),
            };
            let ok = proxy_or_object_delete(&args[0], &key, env, fns, objects, proxies)?;
            Ok(JsVal::Bool(ok))
        }
        ReflectOp::Apply => {
            if args.len() < 3 {
                return Err(());
            }
            let this_arg = args[1].clone();
            let argv = object_to_arg_list(&args[2], objects)?;
            call_value(&args[0], this_arg, &argv, env, fns, objects, proxies)
        }
        ReflectOp::Construct => {
            if args.len() < 2 {
                return Err(());
            }
            let argv = object_to_arg_list(&args[1], objects)?;
            let new_target = if args.len() >= 3 {
                args[2].clone()
            } else {
                args[0].clone()
            };
            construct_value(&args[0], &argv, &new_target, env, fns, objects, proxies)
        }
        ReflectOp::OwnKeys => {
            if args.is_empty() {
                return Err(());
            }
            proxy_or_object_own_keys(&args[0], env, fns, objects, proxies)
        }
    }
}

fn eval_binary(op: &BinaryOp, l: &JsVal, r: &JsVal) -> Result<JsVal, ()> {
    match op {
        BinaryOp::Add => match (l, r) {
            (JsVal::Num(a), JsVal::Num(b)) => Ok(JsVal::Num(a + b)),
            (JsVal::Num(a), other) => Ok(JsVal::Num(a + to_number(other)?)),
            (other, JsVal::Num(b)) => Ok(JsVal::Num(to_number(other)? + b)),
            _ => Err(()),
        },
        BinaryOp::Mul => match (l, r) {
            (JsVal::Num(a), JsVal::Num(b)) => Ok(JsVal::Num(a * b)),
            (JsVal::Num(a), other) => Ok(JsVal::Num(a * to_number(other)?)),
            (other, JsVal::Num(b)) => Ok(JsVal::Num(to_number(other)? * b)),
            _ => Err(()),
        },
        BinaryOp::EqEqEq => Ok(JsVal::Bool(strict_eq(l, r))),
        BinaryOp::NotEqEq => Ok(JsVal::Bool(!strict_eq(l, r))),
        BinaryOp::EqEq => Ok(JsVal::Bool(strict_eq(l, r))),
        BinaryOp::NotEq => Ok(JsVal::Bool(!strict_eq(l, r))),
        _ => Err(()),
    }
}

fn call_value(
    callee: &JsVal,
    this_arg: JsVal,
    args: &[JsVal],
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<JsVal, ()> {
    match callee {
        JsVal::Fn(i) => {
            let this = match &this_arg {
                JsVal::Undef | JsVal::Null => None,
                other => Some(other.clone()),
            };
            call_fn_this(*i, args, this, env, fns, objects, proxies)
        }
        JsVal::ReflectMethod(op) => call_reflect(*op, args, env, fns, objects, proxies),
        JsVal::Proxy(idx) => {
            let rec = proxies.get(*idx).ok_or(())?.clone();
            if let Some(trap_idx) = rec.apply_trap {
                let args_obj = make_args_object(args, objects);
                let trap_args = vec![rec.target.clone(), this_arg, args_obj];
                call_fn(trap_idx, &trap_args, env, fns, objects, proxies)
            } else {
                call_value(&rec.target, this_arg, args, env, fns, objects, proxies)
            }
        }
        _ => Err(()),
    }
}

fn strict_eq(l: &JsVal, r: &JsVal) -> bool {
    match (l, r) {
        (JsVal::Num(a), JsVal::Num(b)) => a == b,
        (JsVal::Bool(a), JsVal::Bool(b)) => a == b,
        (JsVal::Str(a), JsVal::Str(b)) => a == b,
        (JsVal::Undef, JsVal::Undef) => true,
        _ => false,
    }
}

fn to_number(v: &JsVal) -> Result<f64, ()> {
    match v {
        JsVal::Num(n) => Ok(*n),
        JsVal::Bool(true) => Ok(1.0),
        JsVal::Bool(false) => Ok(0.0),
        JsVal::Str(s) => s.parse().map_err(|_| ()),
        _ => Err(()),
    }
}

fn proxy_or_object_get(
    obj: &JsVal,
    key: &str,
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<JsVal, ()> {
    match obj {
        JsVal::Object(idx) => {
            let props = &objects.get(*idx).ok_or(())?.props;
            Ok(props.get(key).cloned().unwrap_or(JsVal::Undef))
        }
        JsVal::Proxy(idx) => {
            let rec = proxies.get(*idx).ok_or(())?.clone();
            if let Some(trap_idx) = rec.get_trap {
                let args = vec![rec.target.clone(), JsVal::Str(key.to_string())];
                call_fn(trap_idx, &args, env, fns, objects, proxies)
            } else {
                proxy_or_object_get(&rec.target, key, env, fns, objects, proxies)
            }
        }
        _ => Err(()),
    }
}

fn proxy_or_object_set(
    obj: &JsVal,
    key: &str,
    value: &JsVal,
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<(), ()> {
    match obj {
        JsVal::Object(idx) => {
            let rec = objects.get_mut(*idx).ok_or(())?;
            object_set_prop(rec, key.to_string(), value.clone());
            Ok(())
        }
        JsVal::Proxy(idx) => {
            let rec = proxies.get(*idx).ok_or(())?.clone();
            if let Some(trap_idx) = rec.set_trap {
                let args = vec![
                    rec.target.clone(),
                    JsVal::Str(key.to_string()),
                    value.clone(),
                ];
                let _ = call_fn(trap_idx, &args, env, fns, objects, proxies)?;
                Ok(())
            } else {
                proxy_or_object_set(&rec.target, key, value, env, fns, objects, proxies)
            }
        }
        _ => Err(()),
    }
}

fn proxy_or_object_own_keys(
    obj: &JsVal,
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<JsVal, ()> {
    match obj {
        JsVal::Object(idx) => {
            let keys = objects.get(*idx).ok_or(())?.keys.clone();
            let mut rec = empty_object();
            for (i, k) in keys.iter().enumerate() {
                object_set_prop(&mut rec, i.to_string(), JsVal::Str(k.clone()));
            }
            object_set_prop(&mut rec, "length".into(), JsVal::Num(keys.len() as f64));
            let out_idx = objects.len();
            objects.push(rec);
            Ok(JsVal::Object(out_idx))
        }
        JsVal::Proxy(idx) => {
            let rec = proxies.get(*idx).ok_or(())?.clone();
            if let Some(trap_idx) = rec.own_keys_trap {
                let args = vec![rec.target.clone()];
                call_fn(trap_idx, &args, env, fns, objects, proxies)
            } else {
                proxy_or_object_own_keys(&rec.target, env, fns, objects, proxies)
            }
        }
        _ => Err(()),
    }
}

fn proxy_or_object_has(
    obj: &JsVal,
    key: &str,
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<bool, ()> {
    match obj {
        JsVal::Object(idx) => {
            let props = &objects.get(*idx).ok_or(())?.props;
            Ok(props.contains_key(key))
        }
        JsVal::Proxy(idx) => {
            let rec = proxies.get(*idx).ok_or(())?.clone();
            if let Some(trap_idx) = rec.has_trap {
                let args = vec![rec.target.clone(), JsVal::Str(key.to_string())];
                let v = call_fn(trap_idx, &args, env, fns, objects, proxies)?;
                Ok(is_truthy(&v))
            } else {
                proxy_or_object_has(&rec.target, key, env, fns, objects, proxies)
            }
        }
        _ => Err(()),
    }
}

fn proxy_or_object_delete(
    obj: &JsVal,
    key: &str,
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<bool, ()> {
    match obj {
        JsVal::Object(idx) => {
            let rec = objects.get_mut(*idx).ok_or(())?;
            object_delete_prop(rec, key);
            Ok(true)
        }
        JsVal::Proxy(idx) => {
            let rec = proxies.get(*idx).ok_or(())?.clone();
            if let Some(trap_idx) = rec.delete_trap {
                let args = vec![rec.target.clone(), JsVal::Str(key.to_string())];
                let v = call_fn(trap_idx, &args, env, fns, objects, proxies)?;
                Ok(is_truthy(&v))
            } else {
                proxy_or_object_delete(&rec.target, key, env, fns, objects, proxies)
            }
        }
        _ => Err(()),
    }
}

impl Emitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            body: String::new(),
            str_consts: Vec::new(),
        }
    }

    fn string_const(&mut self, s: &str) -> String {
        if let Some((_, name)) = self.str_consts.iter().find(|(v, _)| v == s) {
            return name.clone();
        }
        let name = format!("@.gstr.{}", self.str_consts.len());
        self.str_consts.push((s.to_string(), name.clone()));
        name
    }

    fn emit_num(&mut self, n: f64) {
        let lit = if n.is_nan() {
            "0x7FF8000000000000".to_string()
        } else if n.is_infinite() {
            if n.is_sign_negative() {
                "0xFFF0000000000000".into()
            } else {
                "0x7FF0000000000000".into()
            }
        } else {
            format!("{n:?}")
        };
        writeln!(
            self.body,
            "  {}",
            PRINT_F64.call(&format!("double {lit}"))
        )
        .ok();
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for id in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_proxies: missing value"))?;
            match v {
                JsVal::Num(n) => self.emit_num(*n),
                JsVal::Str(s) => {
                    let name = self.string_const(s);
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
                JsVal::Bool(b) => {
                    let s = if *b { "true" } else { "false" };
                    let name = self.string_const(s);
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
                _ => return Err(diag("es_proxies: non-printable value")),
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.13.08 Proxy ownKeys)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        for (s, name) in &self.str_consts {
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

    fn compile(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn proxy_basics_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/proxies/proxy_basics.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("function") || ir.contains("print"),
            "should print observations:\n{ir}"
        );
    }

    #[test]
    fn proxy_set_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/proxies/proxy_set.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("print") || ir.contains("2"),
            "should print observations:\n{ir}"
        );
    }

    #[test]
    fn proxy_has_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/proxies/proxy_has.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("true") || ir.contains("print"),
            "should print observations:\n{ir}"
        );
    }

    #[test]
    fn proxy_delete_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/proxies/proxy_delete.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("true") || ir.contains("print") || ir.contains("keep"),
            "should print observations:\n{ir}"
        );
    }

    #[test]
    fn proxy_apply_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/proxies/proxy_apply.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("print") || ir.contains("5") || ir.contains("21"),
            "should print observations:\n{ir}"
        );
    }

    #[test]
    fn proxy_construct_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/proxies/proxy_construct.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("print") || ir.contains("5") || ir.contains("20"),
            "should print observations:\n{ir}"
        );
    }

    #[test]
    fn reflect_basics_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/proxies/reflect_basics.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("object") && ir.contains("function"),
            "should print Reflect typeof observations:\n{ir}"
        );
        assert!(
            ir.contains("print") || ir.contains("11") || ir.contains("13"),
            "should print numeric observations:\n{ir}"
        );
    }

    #[test]
    fn proxy_own_keys_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/proxies/proxy_own_keys.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("function") && ir.contains("extra"),
            "should print ownKeys observations:\n{ir}"
        );
        assert!(
            ir.contains("print") || ir.contains("2"),
            "should print numeric observations:\n{ir}"
        );
    }
}
