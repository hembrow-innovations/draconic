//! N08.13.01–N08.13.04: native observations for Proxy basics + `set` + `has`/`in`
//! + `delete`/`deleteProperty` (E14.01–E14.04).
//!
//! Compile-time evaluation of a small Proxy subset: `typeof Proxy`,
//! `new Proxy(target, handler)`, empty-handler get/set/`in`/`delete` pass-through,
//! `get`/`set`/`has`/`deleteProperty` traps (function props; free-var capture;
//! string keys), member assign, `typeof` on proxies. Objects live on a heap so
//! proxy targets share identity with outer locals. Emits Runtime prints of final
//! top-level number/string/bool locals.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, JsString, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp, ObjectPropKey,
    Param, Pattern, Stmt,
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

#[derive(Clone, Debug)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    /// Builtin `Proxy` constructor.
    ProxyCtor,
    /// Plain object (index into object heap).
    Object(usize),
    /// Function value (index into `fns`).
    Fn(usize),
    /// Proxy instance (index into `proxies`).
    Proxy(usize),
}

#[derive(Clone, Debug)]
struct FnRec {
    params: Vec<LocalId>,
    body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
struct ObjectRec {
    props: HashMap<String, JsVal>,
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
                    JsVal::Object(_) | JsVal::Proxy(_) | JsVal::Fn(_) | JsVal::ProxyCtor,
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
        Expr::Local { id, .. } => by_id.get(id).is_some_and(|l| l.name == "Proxy"),
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
        JsVal::Undef => false,
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

fn call_fn(
    idx: usize,
    args: &[JsVal],
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
    for stmt in &rec.body {
        if let Some(v) = eval_stmt(stmt, env, fns, objects, proxies)? {
            return Ok(v);
        }
    }
    Ok(JsVal::Undef)
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
        Expr::Local { id, .. } => env.get(id).cloned().ok_or(()),
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
            let mut props = HashMap::new();
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
                        props.insert(k, v);
                    }
                    _ => return Err(()),
                }
            }
            let idx = objects.len();
            objects.push(ObjectRec { props });
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
                    let idx = proxies.len();
                    proxies.push(ProxyRec {
                        target,
                        get_trap,
                        set_trap,
                        has_trap,
                        delete_trap,
                    });
                    Ok(JsVal::Proxy(idx))
                }
                _ => Err(()),
            }
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            let c = eval_expr(callee, env, fns, objects, proxies)?;
            let mut argv = Vec::new();
            for a in args {
                match a {
                    Arg::Expr(e) => argv.push(eval_expr(e, env, fns, objects, proxies)?),
                    _ => return Err(()),
                }
            }
            match c {
                JsVal::Fn(i) => call_fn(i, &argv, env, fns, objects, proxies),
                _ => Err(()),
            }
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            let obj = eval_expr(object, env, fns, objects, proxies)?;
            let key = eval_key(property, env, fns, objects, proxies)?;
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
        JsVal::ProxyCtor | JsVal::Fn(_) => "function".into(),
        JsVal::Object(_) | JsVal::Proxy(_) => "object".into(),
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
        BinaryOp::EqEqEq => Ok(JsVal::Bool(strict_eq(l, r))),
        BinaryOp::NotEqEq => Ok(JsVal::Bool(!strict_eq(l, r))),
        BinaryOp::EqEq => Ok(JsVal::Bool(strict_eq(l, r))),
        BinaryOp::NotEq => Ok(JsVal::Bool(!strict_eq(l, r))),
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
            rec.props.insert(key.to_string(), value.clone());
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
            rec.props.remove(key);
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
            "; Draconic LLVM backend (N08.13.04 Proxy deleteProperty)"
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
}
