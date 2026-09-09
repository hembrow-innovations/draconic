use super::traps::*;

use super::*;

pub(super) fn eval_body(
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

pub(super) fn eval_stmt(
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

pub(super) fn is_truthy(v: &JsVal) -> bool {
    match v {
        JsVal::Bool(b) => *b,
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Undef | JsVal::Null => false,
        _ => true,
    }
}

pub(super) fn simple_param_locals(params: &[Param]) -> Option<Vec<LocalId>> {
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

pub(super) fn register_fn(params: &[Param], body: &[Stmt], fns: &mut Vec<FnRec>) -> Result<JsVal, ()> {
    let param_ids = simple_param_locals(params).ok_or(())?;
    let idx = fns.len();
    fns.push(FnRec {
        params: param_ids,
        body: body.to_vec(),
    });
    Ok(JsVal::Fn(idx))
}

pub(super) fn with_this<R>(this: Option<JsVal>, f: impl FnOnce() -> R) -> R {
    CURRENT_THIS.with(|slot| {
        let prev = slot.replace(this);
        let r = f();
        *slot.borrow_mut() = prev;
        r
    })
}

pub(super) fn call_fn(
    idx: usize,
    args: &[JsVal],
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<JsVal, ()> {
    call_fn_this(idx, args, None, env, fns, objects, proxies)
}

pub(super) fn call_fn_this(
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

pub(super) fn make_args_object(args: &[JsVal], objects: &mut Vec<ObjectRec>) -> JsVal {
    let mut rec = empty_object();
    for (i, a) in args.iter().enumerate() {
        object_set_prop(&mut rec, i.to_string(), a.clone());
    }
    object_set_prop(&mut rec, "length".into(), JsVal::Num(args.len() as f64));
    let arr_idx = objects.len();
    objects.push(rec);
    JsVal::Object(arr_idx)
}

pub(super) fn construct_value(
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

pub(super) fn eval_key(
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

pub(super) fn eval_expr(
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
            op: UnaryOp::Void,
            arg,
            ..
        } => {
            let _ = eval_expr(arg, env, fns, objects, proxies)?;
            Ok(JsVal::Undef)
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
            left, op, right, ..
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
                            ObjectPropKey::Computed(e) => eval_key(e, env, fns, objects, proxies)?,
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
                    let get_prototype_of_trap = match handler.get("getPrototypeOf") {
                        Some(JsVal::Fn(i)) => Some(*i),
                        Some(_) => return Err(()),
                        None => None,
                    };
                    let set_prototype_of_trap = match handler.get("setPrototypeOf") {
                        Some(JsVal::Fn(i)) => Some(*i),
                        Some(_) => return Err(()),
                        None => None,
                    };
                    let define_property_trap = match handler.get("defineProperty") {
                        Some(JsVal::Fn(i)) => Some(*i),
                        Some(_) => return Err(()),
                        None => None,
                    };
                    let get_own_property_descriptor_trap =
                        match handler.get("getOwnPropertyDescriptor") {
                            Some(JsVal::Fn(i)) => Some(*i),
                            Some(_) => return Err(()),
                            None => None,
                        };
                    let is_extensible_trap = match handler.get("isExtensible") {
                        Some(JsVal::Fn(i)) => Some(*i),
                        Some(_) => return Err(()),
                        None => None,
                    };
                    let prevent_extensions_trap = match handler.get("preventExtensions") {
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
                        get_prototype_of_trap,
                        set_prototype_of_trap,
                        define_property_trap,
                        get_own_property_descriptor_trap,
                        is_extensible_trap,
                        prevent_extensions_trap,
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
            target: AssignTarget::Member {
                object, property, ..
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

pub(super) fn typeof_str(v: &JsVal) -> String {
    match v {
        JsVal::Num(_) => "number".into(),
        JsVal::Bool(_) => "boolean".into(),
        JsVal::Str(_) => "string".into(),
        JsVal::Undef => "undefined".into(),
        JsVal::ProxyCtor | JsVal::Fn(_) | JsVal::ReflectMethod(_) => "function".into(),
        JsVal::Object(_) | JsVal::Proxy(_) | JsVal::ReflectObj | JsVal::Null => "object".into(),
    }
}

pub(super) fn reflect_method(key: &str) -> Result<JsVal, ()> {
    let op = match key {
        "get" => ReflectOp::Get,
        "set" => ReflectOp::Set,
        "has" => ReflectOp::Has,
        "deleteProperty" => ReflectOp::DeleteProperty,
        "apply" => ReflectOp::Apply,
        "construct" => ReflectOp::Construct,
        "ownKeys" => ReflectOp::OwnKeys,
        "getPrototypeOf" => ReflectOp::GetPrototypeOf,
        "setPrototypeOf" => ReflectOp::SetPrototypeOf,
        "defineProperty" => ReflectOp::DefineProperty,
        "getOwnPropertyDescriptor" => ReflectOp::GetOwnPropertyDescriptor,
        "isExtensible" => ReflectOp::IsExtensible,
        "preventExtensions" => ReflectOp::PreventExtensions,
        _ => return Err(()),
    };
    Ok(JsVal::ReflectMethod(op))
}

pub(super) fn object_to_arg_list(obj: &JsVal, objects: &[ObjectRec]) -> Result<Vec<JsVal>, ()> {
    match obj {
        JsVal::Object(idx) => {
            let props = &objects.get(*idx).ok_or(())?.props;
            let len = match props.get("length") {
                Some(JsVal::Num(n)) if *n >= 0.0 && n.is_finite() => *n as usize,
                _ => return Err(()),
            };
            let mut out = Vec::with_capacity(len);
            for i in 0..len {
                out.push(props.get(&i.to_string()).cloned().unwrap_or(JsVal::Undef));
            }
            Ok(out)
        }
        _ => Err(()),
    }
}

pub(super) fn call_reflect(
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
        ReflectOp::GetPrototypeOf => {
            if args.is_empty() {
                return Err(());
            }
            proxy_or_object_get_prototype_of(&args[0], env, fns, objects, proxies)
        }
        ReflectOp::SetPrototypeOf => {
            if args.len() < 2 {
                return Err(());
            }
            let ok =
                proxy_or_object_set_prototype_of(&args[0], &args[1], env, fns, objects, proxies)?;
            Ok(JsVal::Bool(ok))
        }
        ReflectOp::DefineProperty => {
            if args.len() < 3 {
                return Err(());
            }
            let key = match &args[1] {
                JsVal::Str(s) => s.clone(),
                JsVal::Num(n) => format!("{}", *n as i64),
                _ => return Err(()),
            };
            let ok = proxy_or_object_define_property(
                &args[0], &key, &args[2], env, fns, objects, proxies,
            )?;
            Ok(JsVal::Bool(ok))
        }
        ReflectOp::GetOwnPropertyDescriptor => {
            if args.len() < 2 {
                return Err(());
            }
            let key = match &args[1] {
                JsVal::Str(s) => s.clone(),
                JsVal::Num(n) => format!("{}", *n as i64),
                _ => return Err(()),
            };
            proxy_or_object_get_own_property_descriptor(&args[0], &key, env, fns, objects, proxies)
        }
        ReflectOp::IsExtensible => {
            if args.is_empty() {
                return Err(());
            }
            let ok = proxy_or_object_is_extensible(&args[0], env, fns, objects, proxies)?;
            Ok(JsVal::Bool(ok))
        }
        ReflectOp::PreventExtensions => {
            if args.is_empty() {
                return Err(());
            }
            let ok = proxy_or_object_prevent_extensions(&args[0], env, fns, objects, proxies)?;
            Ok(JsVal::Bool(ok))
        }
    }
}

pub(super) fn eval_binary(op: &BinaryOp, l: &JsVal, r: &JsVal) -> Result<JsVal, ()> {
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

pub(super) fn call_value(
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

pub(super) fn strict_eq(l: &JsVal, r: &JsVal) -> bool {
    match (l, r) {
        (JsVal::Num(a), JsVal::Num(b)) => a == b,
        (JsVal::Bool(a), JsVal::Bool(b)) => a == b,
        (JsVal::Str(a), JsVal::Str(b)) => a == b,
        (JsVal::Undef, JsVal::Undef) => true,
        (JsVal::Null, JsVal::Null) => true,
        (JsVal::Object(a), JsVal::Object(b)) => a == b,
        (JsVal::Proxy(a), JsVal::Proxy(b)) => a == b,
        (JsVal::Fn(a), JsVal::Fn(b)) => a == b,
        _ => false,
    }
}

pub(super) fn to_number(v: &JsVal) -> Result<f64, ()> {
    match v {
        JsVal::Num(n) => Ok(*n),
        JsVal::Bool(true) => Ok(1.0),
        JsVal::Bool(false) => Ok(0.0),
        JsVal::Str(s) => s.parse().map_err(|_| ()),
        _ => Err(()),
    }
}

