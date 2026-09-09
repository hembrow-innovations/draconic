use super::call::*;

use super::*;

pub(super) fn eval_body(
    body: &[Stmt],
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<(), ()> {
    for s in body {
        let _ = eval_stmt(s, env, heap, by_id)?;
    }
    Ok(())
}

/// Returns `Some` when a `return` was executed (function body).
pub(super) fn eval_stmt(
    stmt: &Stmt,
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<Option<JsVal>, ()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => eval_expr(e, env, heap, by_id)?,
                None => JsVal::Undef,
            };
            env.locals.insert(*local, v);
            Ok(None)
        }
        Stmt::Expr { expr } => {
            // Bare "use strict" string expr is a no-op directive.
            if let Expr::String { value, .. } = expr {
                if value.to_string_lossy() == "use strict" {
                    return Ok(None);
                }
            }
            eval_expr(expr, env, heap, by_id)?;
            Ok(None)
        }
        Stmt::Block { body } => {
            for s in body {
                if let Some(v) = eval_stmt(s, env, heap, by_id)? {
                    return Ok(Some(v));
                }
            }
            Ok(None)
        }
        Stmt::Return { value } => {
            let v = match value {
                Some(e) => eval_expr(e, env, heap, by_id)?,
                None => JsVal::Undef,
            };
            Ok(Some(v))
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            if to_boolean(&eval_expr(test, env, heap, by_id)?) {
                eval_stmt(consequent, env, heap, by_id)
            } else if let Some(a) = alternate {
                eval_stmt(a, env, heap, by_id)
            } else {
                Ok(None)
            }
        }
        Stmt::Throw { .. } => Err(()),
        Stmt::Try { block, handler, .. } => match eval_body(block, env, heap, by_id) {
            Ok(()) => Ok(None),
            Err(()) => {
                if let Some(h) = handler {
                    for s in h {
                        if let Some(v) = eval_stmt(s, env, heap, by_id)? {
                            return Ok(Some(v));
                        }
                    }
                    Ok(None)
                } else {
                    Err(())
                }
            }
        },
        _ => Err(()),
    }
}

pub(super) fn eval_expr(
    expr: &Expr,
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<JsVal, ()> {
    match expr {
        Expr::Number { raw, .. } => Ok(JsVal::Num(raw.parse().map_err(|_| ())?)),
        Expr::String { value, .. } => Ok(JsVal::Str(value.to_string_lossy())),
        Expr::Boolean { value, .. } => Ok(JsVal::Bool(*value)),
        Expr::Null { .. } => Ok(JsVal::Null),
        Expr::This { .. } => Ok(env.this.clone()),
        Expr::NewTarget { .. } => Ok(env.new_target.clone()),
        Expr::Local { id, .. } => env.locals.get(id).cloned().ok_or(()),
        Expr::IdentName { name, .. } => resolve_ident(name, env),
        Expr::Function {
            params,
            body,
            is_arrow,
            is_method,
            ..
        } => Ok(alloc_function(
            heap,
            env,
            params,
            body.clone(),
            *is_arrow,
            *is_method,
        )?),
        Expr::Object { properties, .. } => {
            let mut props = HashMap::new();
            let mut proto: Option<JsVal> = None;
            for p in properties {
                match p {
                    ObjectProp::Property {
                        key: ObjectPropKey::Static(k),
                        value,
                    } => {
                        let key = k.to_string_lossy();
                        let v = eval_expr(value, env, heap, by_id)?;
                        if key == "__proto__" {
                            proto = Some(v);
                        } else {
                            props.insert(key, v);
                        }
                    }
                    ObjectProp::Property {
                        key: ObjectPropKey::Computed(ke),
                        value,
                    } => {
                        let key = to_key(&eval_expr(ke, env, heap, by_id)?)?;
                        let v = eval_expr(value, env, heap, by_id)?;
                        props.insert(key, v);
                    }
                    _ => return Err(()),
                }
            }
            let oid = heap.alloc_obj(props);
            if let Some(p) = proto {
                // Store [[Prototype]] under internal key; member get walks it.
                heap.set(oid, "[[Prototype]]", p);
            }
            Ok(JsVal::Obj(oid))
        }
        Expr::Member {
            object,
            property,
            computed,
            optional,
            ..
        } => {
            if *optional {
                return Err(());
            }
            let obj = eval_expr(object, env, heap, by_id)?;
            let key = member_key(property, *computed, env, heap, by_id)?;
            member_get_full(&obj, &key, env, heap, by_id)
        }
        Expr::Array { elements, .. } => {
            // Only empty arrays needed (Reflect.construct heritage args).
            if !elements.is_empty() {
                return Err(());
            }
            Ok(JsVal::Obj(heap.alloc_obj(HashMap::new())))
        }
        Expr::Assign {
            target,
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = eval_expr(value, env, heap, by_id)?;
            put_value(target, v.clone(), env, heap, by_id)?;
            Ok(v)
        }
        Expr::Binary {
            left, op, right, ..
        } => eval_binary(left, *op, right, env, heap, by_id),
        Expr::Unary { op, arg, .. } => eval_unary(*op, arg, env, heap, by_id),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            if to_boolean(&eval_expr(test, env, heap, by_id)?) {
                eval_expr(consequent, env, heap, by_id)
            } else {
                eval_expr(alternate, env, heap, by_id)
            }
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            if *optional {
                return Err(());
            }
            eval_call(callee, args, env, heap, by_id)
        }
        Expr::New { callee, args, .. } => eval_new(callee, args, env, heap, by_id),
        _ => Err(()),
    }
}

pub(super) fn resolve_ident(name: &str, env: &Env) -> Result<JsVal, ()> {
    if let Some(v) = env.names.get(name) {
        return Ok(v.clone());
    }
    match name {
        "undefined" => Ok(JsVal::Undef),
        "Object" => Ok(JsVal::Builtin("Object")),
        "Function" => Ok(JsVal::Builtin("Function")),
        "WeakMap" => Ok(JsVal::Builtin("WeakMap")),
        "TypeError" => Ok(JsVal::Builtin("TypeError")),
        "Reflect" => Ok(JsVal::Builtin("Reflect")),
        "Proxy" => Ok(JsVal::Builtin("Proxy")),
        "arguments" => Ok(JsVal::Builtin("arguments")),
        _ => Err(()),
    }
}

/// Functions are objects with `[[Call]]` + default `prototype` (class ctor shape).
pub(super) fn alloc_function(
    heap: &mut Heap,
    env: &Env,
    params: &[Param],
    body: Vec<Stmt>,
    is_arrow: bool,
    is_method: bool, // retained for future method-home checks
) -> Result<JsVal, ()> {
    let param_ids = simple_params(params)?;
    let fid = heap.alloc_fn(FnRec {
        params: param_ids,
        body,
        is_arrow,
        is_method,
        closure: env.locals.clone(),
        name_closure: env.names.clone(),
    });
    let mut props = HashMap::new();
    props.insert("[[Call]]".into(), JsVal::Fn(fid));
    if !is_arrow && !is_method {
        let proto = heap.alloc_obj(HashMap::new());
        props.insert("prototype".into(), JsVal::Obj(proto));
    }
    Ok(JsVal::Obj(heap.alloc_obj(props)))
}

pub(super) fn as_callable(v: &JsVal, heap: &Heap) -> Option<usize> {
    match v {
        JsVal::Fn(fid) => Some(*fid),
        JsVal::Obj(oid) => match heap.get(*oid, "[[Call]]") {
            JsVal::Fn(fid) => Some(fid),
            _ => None,
        },
        _ => None,
    }
}

#[derive(Clone)]
pub(super) enum ParamBind {
    Local(LocalId),
    Name(String),
}

pub(super) fn simple_params(params: &[Param]) -> Result<Vec<ParamBind>, ()> {
    let mut out = Vec::new();
    for p in params {
        if p.rest || p.default.is_some() {
            return Err(());
        }
        match &p.pattern {
            Pattern::Local(id) => out.push(ParamBind::Local(*id)),
            Pattern::Name(n) => out.push(ParamBind::Name(n.clone())),
            _ => return Err(()),
        }
    }
    Ok(out)
}

pub(super) fn member_key(
    property: &Expr,
    computed: bool,
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<String, ()> {
    if !computed {
        match property {
            Expr::String { value, .. } => Ok(value.to_string_lossy()),
            Expr::IdentName { name, .. } => Ok(name.clone()),
            _ => Err(()),
        }
    } else {
        to_key(&eval_expr(property, env, heap, by_id)?)
    }
}

pub(super) fn to_key(v: &JsVal) -> Result<String, ()> {
    match v {
        JsVal::Str(s) => Ok(s.clone()),
        JsVal::Num(n) => {
            if n.fract() == 0.0 && n.is_finite() {
                Ok(format!("{}", *n as i64))
            } else {
                Ok(format!("{n}"))
            }
        }
        JsVal::Bool(b) => Ok(if *b { "true".into() } else { "false".into() }),
        JsVal::Undef => Ok("undefined".into()),
        JsVal::Null => Ok("null".into()),
        _ => Err(()),
    }
}

pub(super) fn member_get(obj: &JsVal, key: &str, heap: &Heap) -> Result<JsVal, ()> {
    match obj {
        JsVal::Obj(oid) => {
            if key.starts_with("[[") {
                return Ok(heap.get(*oid, key));
            }
            // Proxy without trap call (own props only) — traps need env; use member_get_full.
            if heap.has_own(*oid, "[[ProxyTarget]]") {
                // Defer to target own/proto without trap when called from pure heap context.
                let target = heap.get(*oid, "[[ProxyTarget]]");
                return member_get(&target, key, heap);
            }
            if heap.has_own(*oid, key) {
                return Ok(heap.get(*oid, key));
            }
            match heap.get(*oid, "[[Prototype]]") {
                JsVal::Obj(p) => member_get(&JsVal::Obj(p), key, heap),
                JsVal::Builtin("Function.prototype") | JsVal::Undef | JsVal::Null => {
                    Ok(JsVal::Undef)
                }
                other => member_get(&other, key, heap),
            }
        }
        JsVal::Builtin("Function") if key == "prototype" => {
            Ok(JsVal::Builtin("Function.prototype"))
        }
        JsVal::Builtin("Object") => match key {
            "defineProperty" => Ok(JsVal::Builtin("Object.defineProperty")),
            "getOwnPropertyDescriptor" => Ok(JsVal::Builtin("Object.getOwnPropertyDescriptor")),
            "setPrototypeOf" => Ok(JsVal::Builtin("Object.setPrototypeOf")),
            "isExtensible" => Ok(JsVal::Builtin("Object.isExtensible")),
            "prototype" => Ok(JsVal::Builtin("Object.prototype")),
            _ => Ok(JsVal::Undef),
        },
        JsVal::Builtin("Reflect") => match key {
            "construct" => Ok(JsVal::Builtin("Reflect.construct")),
            "get" => Ok(JsVal::Builtin("Reflect.get")),
            _ => Ok(JsVal::Undef),
        },
        JsVal::Fn(_) => Ok(JsVal::Undef),
        JsVal::WeakMap(_) => match key {
            "set" | "get" | "has" => Ok(JsVal::Builtin(match key {
                "set" => "WeakMap.set",
                "get" => "WeakMap.get",
                _ => "WeakMap.has",
            })),
            _ => Ok(JsVal::Undef),
        },
        _ => Ok(JsVal::Undef),
    }
}

/// Member get that runs Proxy `get` traps when present.
pub(super) fn member_get_full(
    obj: &JsVal,
    key: &str,
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<JsVal, ()> {
    if let JsVal::Obj(oid) = obj {
        if heap.has_own(*oid, "[[ProxyTarget]]") {
            let target = heap.get(*oid, "[[ProxyTarget]]");
            if let Some(fid) = as_callable(&heap.get(*oid, "[[ProxyGet]]"), heap) {
                return call_value(
                    &JsVal::Fn(fid),
                    JsVal::Undef,
                    &[target, JsVal::Str(key.to_string()), obj.clone()],
                    env,
                    heap,
                    by_id,
                );
            }
            return member_get_full(&target, key, env, heap, by_id);
        }
    }
    member_get(obj, key, heap)
}

pub(super) fn put_value(
    target: &AssignTarget,
    val: JsVal,
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<(), ()> {
    match target {
        AssignTarget::Local(id) => {
            env.locals.insert(*id, val);
            Ok(())
        }
        AssignTarget::Member {
            object,
            property,
            computed,
        } => {
            let obj = eval_expr(object, env, heap, by_id)?;
            let key = member_key(property, *computed, env, heap, by_id)?;
            match obj {
                JsVal::Obj(oid) => {
                    heap.set(oid, &key, val);
                    Ok(())
                }
                _ => Err(()),
            }
        }
        _ => Err(()),
    }
}

pub(super) fn eval_unary(
    op: UnaryOp,
    arg: &Expr,
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<JsVal, ()> {
    match op {
        UnaryOp::TypeOf => {
            if let Expr::Member {
                object,
                property,
                computed,
                optional: false,
                ..
            } = arg
            {
                let obj = eval_expr(object, env, heap, by_id)?;
                let key = member_key(property, *computed, env, heap, by_id)?;
                let v = member_get(&obj, &key, heap)?;
                return Ok(JsVal::Str(typeof_val(&v, heap)));
            }
            let v = eval_expr(arg, env, heap, by_id)?;
            Ok(JsVal::Str(typeof_val(&v, heap)))
        }
        UnaryOp::Void => {
            let _ = eval_expr(arg, env, heap, by_id)?;
            Ok(JsVal::Undef)
        }
        UnaryOp::Not => Ok(JsVal::Bool(!to_boolean(&eval_expr(arg, env, heap, by_id)?))),
        UnaryOp::Delete => match arg {
            Expr::Member {
                object,
                property,
                computed,
                ..
            } => {
                let obj = eval_expr(object, env, heap, by_id)?;
                let key = member_key(property, *computed, env, heap, by_id)?;
                match obj {
                    JsVal::Obj(oid) => Ok(JsVal::Bool(heap.delete(oid, &key))),
                    _ => Ok(JsVal::Bool(true)),
                }
            }
            _ => Ok(JsVal::Bool(true)),
        },
        UnaryOp::Minus => match eval_expr(arg, env, heap, by_id)? {
            JsVal::Num(n) => Ok(JsVal::Num(-n)),
            _ => Err(()),
        },
        UnaryOp::Plus => match eval_expr(arg, env, heap, by_id)? {
            JsVal::Num(n) => Ok(JsVal::Num(n)),
            JsVal::Str(s) => Ok(JsVal::Num(s.parse().unwrap_or(f64::NAN))),
            JsVal::Bool(b) => Ok(JsVal::Num(if b { 1.0 } else { 0.0 })),
            JsVal::Undef => Ok(JsVal::Num(f64::NAN)),
            JsVal::Null => Ok(JsVal::Num(0.0)),
            _ => Err(()),
        },
        _ => Err(()),
    }
}

pub(super) fn typeof_str(v: &JsVal) -> String {
    match v {
        JsVal::Num(_) => "number".into(),
        JsVal::Str(_) => "string".into(),
        JsVal::Bool(_) => "boolean".into(),
        JsVal::Undef => "undefined".into(),
        JsVal::Null => "object".into(),
        JsVal::Obj(_) => {
            // Callables (class ctors / methods boxed as objects) are typeof "function".
            "object".into() // refined by typeof_str_heap when heap available
        }
        JsVal::Fn(_) | JsVal::Builtin(_) => "function".into(),
        JsVal::WeakMap(_) => "object".into(),
    }
}

pub(super) fn typeof_val(v: &JsVal, heap: &Heap) -> String {
    match v {
        JsVal::Obj(oid) if as_callable(v, heap).is_some() => "function".into(),
        JsVal::Obj(_) => "object".into(),
        other => typeof_str(other),
    }
}

pub(super) fn eval_binary(
    left: &Expr,
    op: BinaryOp,
    right: &Expr,
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<JsVal, ()> {
    match op {
        BinaryOp::Comma => {
            let _ = eval_expr(left, env, heap, by_id)?;
            eval_expr(right, env, heap, by_id)
        }
        BinaryOp::And => {
            let l = eval_expr(left, env, heap, by_id)?;
            if !to_boolean(&l) {
                Ok(l)
            } else {
                eval_expr(right, env, heap, by_id)
            }
        }
        BinaryOp::Or => {
            let l = eval_expr(left, env, heap, by_id)?;
            if to_boolean(&l) {
                Ok(l)
            } else {
                eval_expr(right, env, heap, by_id)
            }
        }
        BinaryOp::EqEqEq => {
            let l = eval_expr(left, env, heap, by_id)?;
            let r = eval_expr(right, env, heap, by_id)?;
            Ok(JsVal::Bool(strict_eq(&l, &r)))
        }
        BinaryOp::NotEqEq => {
            let l = eval_expr(left, env, heap, by_id)?;
            let r = eval_expr(right, env, heap, by_id)?;
            Ok(JsVal::Bool(!strict_eq(&l, &r)))
        }
        BinaryOp::EqEq => {
            let l = eval_expr(left, env, heap, by_id)?;
            let r = eval_expr(right, env, heap, by_id)?;
            Ok(JsVal::Bool(loose_eq(&l, &r)))
        }
        BinaryOp::NotEq => {
            let l = eval_expr(left, env, heap, by_id)?;
            let r = eval_expr(right, env, heap, by_id)?;
            Ok(JsVal::Bool(!loose_eq(&l, &r)))
        }
        BinaryOp::Add => {
            let l = eval_expr(left, env, heap, by_id)?;
            let r = eval_expr(right, env, heap, by_id)?;
            match (&l, &r) {
                (JsVal::Str(a), JsVal::Str(b)) => Ok(JsVal::Str(format!("{a}{b}"))),
                (JsVal::Str(a), b) => Ok(JsVal::Str(format!("{a}{}", to_string_js(b)))),
                (a, JsVal::Str(b)) => Ok(JsVal::Str(format!("{}{b}", to_string_js(a)))),
                _ => Ok(JsVal::Num(to_number(&l)? + to_number(&r)?)),
            }
        }
        BinaryOp::Sub => Ok(JsVal::Num(
            to_number(&eval_expr(left, env, heap, by_id)?)?
                - to_number(&eval_expr(right, env, heap, by_id)?)?,
        )),
        BinaryOp::Mul => Ok(JsVal::Num(
            to_number(&eval_expr(left, env, heap, by_id)?)?
                * to_number(&eval_expr(right, env, heap, by_id)?)?,
        )),
        BinaryOp::Div => Ok(JsVal::Num(
            to_number(&eval_expr(left, env, heap, by_id)?)?
                / to_number(&eval_expr(right, env, heap, by_id)?)?,
        )),
        _ => Err(()),
    }
}

pub(super) fn strict_eq(a: &JsVal, b: &JsVal) -> bool {
    match (a, b) {
        (JsVal::Num(x), JsVal::Num(y)) => x == y,
        (JsVal::Str(x), JsVal::Str(y)) => x == y,
        (JsVal::Bool(x), JsVal::Bool(y)) => x == y,
        (JsVal::Undef, JsVal::Undef) => true,
        (JsVal::Null, JsVal::Null) => true,
        (JsVal::Obj(x), JsVal::Obj(y)) => x == y,
        (JsVal::Fn(x), JsVal::Fn(y)) => x == y,
        (JsVal::WeakMap(x), JsVal::WeakMap(y)) => x == y,
        (JsVal::Builtin(x), JsVal::Builtin(y)) => x == y,
        _ => false,
    }
}

pub(super) fn loose_eq(a: &JsVal, b: &JsVal) -> bool {
    // Enough for `x != null` brand checks.
    match (a, b) {
        (JsVal::Null, JsVal::Undef) | (JsVal::Undef, JsVal::Null) => true,
        _ => strict_eq(a, b),
    }
}

pub(super) fn to_string_js(v: &JsVal) -> String {
    match v {
        JsVal::Num(n) => {
            if n.fract() == 0.0 && n.is_finite() {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        JsVal::Str(s) => s.clone(),
        JsVal::Bool(b) => if *b { "true" } else { "false" }.into(),
        JsVal::Undef => "undefined".into(),
        JsVal::Null => "null".into(),
        JsVal::Obj(_) | JsVal::WeakMap(_) => "[object Object]".into(),
        JsVal::Fn(_) | JsVal::Builtin(_) => "function".into(),
    }
}

pub(super) fn to_number(v: &JsVal) -> Result<f64, ()> {
    match v {
        JsVal::Num(n) => Ok(*n),
        JsVal::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
        JsVal::Null => Ok(0.0),
        JsVal::Undef => Ok(f64::NAN),
        JsVal::Str(s) => Ok(s.parse().unwrap_or(f64::NAN)),
        _ => Err(()),
    }
}

pub(super) fn to_boolean(v: &JsVal) -> bool {
    match v {
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Bool(b) => *b,
        JsVal::Undef | JsVal::Null => false,
        JsVal::Obj(_) | JsVal::Fn(_) | JsVal::WeakMap(_) | JsVal::Builtin(_) => true,
    }
}

