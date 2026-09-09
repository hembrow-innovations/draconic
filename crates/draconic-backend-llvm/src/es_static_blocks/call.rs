use super::eval::*;

use super::*;

pub(super) fn eval_call(
    callee: &Expr,
    args: &[Arg],
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<JsVal, ()> {
    // method.call(thisArg, ...args) — Function.prototype.call
    if let Expr::Member {
        object,
        property,
        optional: false,
        ..
    } = callee
    {
        if let Expr::String { value, .. } = property.as_ref() {
            if value.to_string_lossy() == "call" {
                let fval = eval_expr(object, env, heap, by_id)?;
                let mut arg_vals = eval_args(args, env, heap, by_id)?;
                let this_arg = if arg_vals.is_empty() {
                    JsVal::Undef
                } else {
                    arg_vals.remove(0)
                };
                return call_value(&fval, this_arg, &arg_vals, env, heap, by_id);
            }
        }
    }

    // Member call: obj.m(args) / Builtin methods
    if let Expr::Member {
        object,
        property,
        computed,
        optional: false,
        ..
    } = callee
    {
        let obj = eval_expr(object, env, heap, by_id)?;
        let key = member_key(property, *computed, env, heap, by_id)?;
        let arg_vals = eval_args(args, env, heap, by_id)?;
        return call_member(&obj, &key, &arg_vals, env, heap, by_id);
    }

    let fval = eval_expr(callee, env, heap, by_id)?;
    let arg_vals = eval_args(args, env, heap, by_id)?;
    call_value(&fval, JsVal::Undef, &arg_vals, env, heap, by_id)
}

pub(super) fn eval_args(
    args: &[Arg],
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<Vec<JsVal>, ()> {
    let mut out = Vec::new();
    for a in args {
        match a {
            Arg::Expr(e) => out.push(eval_expr(e, env, heap, by_id)?),
            Arg::Spread(_) => return Err(()),
        }
    }
    Ok(out)
}

pub(super) fn call_member(
    obj: &JsVal,
    key: &str,
    args: &[JsVal],
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<JsVal, ()> {
    if let JsVal::Builtin("Object") = obj {
        return match key {
            "defineProperty" => builtin_define_property(args, heap),
            "getOwnPropertyDescriptor" => builtin_gopd(args, heap),
            "setPrototypeOf" => builtin_set_prototype_of(args, heap),
            "isExtensible" => Ok(JsVal::Bool(true)),
            _ => Err(()),
        };
    }
    if let JsVal::WeakMap(wmid) = obj {
        return match key {
            "set" => {
                if args.len() < 2 {
                    return Err(());
                }
                let JsVal::Obj(oid) = &args[0] else {
                    return Err(());
                };
                heap.weakmaps
                    .get_mut(*wmid)
                    .ok_or(())?
                    .insert(*oid, args[1].clone());
                Ok(obj.clone())
            }
            "get" => {
                let JsVal::Obj(oid) = args.first().ok_or(())? else {
                    return Ok(JsVal::Undef);
                };
                Ok(heap
                    .weakmaps
                    .get(*wmid)
                    .and_then(|m| m.get(oid).cloned())
                    .unwrap_or(JsVal::Undef))
            }
            "has" => {
                let JsVal::Obj(oid) = args.first().ok_or(())? else {
                    return Ok(JsVal::Bool(false));
                };
                Ok(JsVal::Bool(
                    heap.weakmaps
                        .get(*wmid)
                        .is_some_and(|m| m.contains_key(oid)),
                ))
            }
            _ => Err(()),
        };
    }
    let method = member_get(obj, key, heap)?;
    call_value(&method, obj.clone(), args, env, heap, by_id)
}

pub(super) fn call_value(
    fval: &JsVal,
    this_arg: JsVal,
    args: &[JsVal],
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<JsVal, ()> {
    match fval {
        JsVal::Builtin("Object.defineProperty") => builtin_define_property(args, heap),
        JsVal::Builtin("Object.getOwnPropertyDescriptor") => builtin_gopd(args, heap),
        JsVal::Builtin("Object.setPrototypeOf") => builtin_set_prototype_of(args, heap),
        JsVal::Builtin("Object.isExtensible") => Ok(JsVal::Bool(true)),
        JsVal::Builtin("Reflect.construct") => {
            // Heritage check: construct empty fn with newTarget=Proxy(parent).
            // Accessing newTarget.prototype runs Proxy get → captures sproto.
            if args.len() < 3 {
                return Err(());
            }
            let new_target = &args[2];
            let _ = member_get_full(new_target, "prototype", env, heap, by_id)?;
            // Return a dummy instance object.
            Ok(JsVal::Obj(heap.alloc_obj(HashMap::new())))
        }
        JsVal::Builtin("Reflect.get") => {
            if args.len() < 2 {
                return Err(());
            }
            let key = to_key(&args[1])?;
            // Reflect.get reads from target (args[0]), not receiver — avoids Proxy trap loops.
            let _receiver = args.get(2);
            member_get(&args[0], &key, heap)
        }
        JsVal::Builtin("WeakMap.set") => {
            let JsVal::WeakMap(wmid) = &this_arg else {
                return Err(());
            };
            if args.len() < 2 {
                return Err(());
            }
            let JsVal::Obj(oid) = &args[0] else {
                return Err(());
            };
            heap.weakmaps
                .get_mut(*wmid)
                .ok_or(())?
                .insert(*oid, args[1].clone());
            Ok(this_arg)
        }
        JsVal::Builtin("WeakMap.get") => {
            let JsVal::WeakMap(wmid) = &this_arg else {
                return Err(());
            };
            let JsVal::Obj(oid) = args.first().ok_or(())? else {
                return Ok(JsVal::Undef);
            };
            Ok(heap
                .weakmaps
                .get(*wmid)
                .and_then(|m| m.get(oid).cloned())
                .unwrap_or(JsVal::Undef))
        }
        JsVal::Builtin("WeakMap.has") => {
            let JsVal::WeakMap(wmid) = &this_arg else {
                return Err(());
            };
            let JsVal::Obj(oid) = args.first().ok_or(())? else {
                return Ok(JsVal::Bool(false));
            };
            Ok(JsVal::Bool(
                heap.weakmaps
                    .get(*wmid)
                    .is_some_and(|m| m.contains_key(oid)),
            ))
        }
        other => {
            let fid = as_callable(other, heap).ok_or(())?;
            let rec = heap.functions.get(fid).cloned().ok_or(())?;
            let mut locals = rec.closure.clone();
            for (k, v) in &env.locals {
                locals.insert(*k, v.clone());
            }
            let mut names = rec.name_closure.clone();
            for (k, v) in &env.names {
                names.insert(k.clone(), v.clone());
            }
            let mut child = Env {
                locals,
                names,
                this: if rec.is_arrow {
                    env.this.clone()
                } else {
                    this_arg
                },
                new_target: if rec.is_arrow {
                    env.new_target.clone()
                } else {
                    JsVal::Undef
                },
            };
            let mut param_locals = Vec::new();
            let mut param_names = Vec::new();
            for (i, pb) in rec.params.iter().enumerate() {
                let v = args.get(i).cloned().unwrap_or(JsVal::Undef);
                match pb {
                    ParamBind::Local(id) => {
                        child.locals.insert(*id, v);
                        param_locals.push(*id);
                    }
                    ParamBind::Name(n) => {
                        child.names.insert(n.clone(), v);
                        param_names.push(n.clone());
                    }
                }
            }
            let mut ret = JsVal::Undef;
            for s in &rec.body {
                if let Some(v) = eval_stmt(s, &mut child, heap, by_id)? {
                    ret = v;
                    break;
                }
            }
            for (k, v) in &child.locals {
                if !param_locals.contains(k) && env.locals.contains_key(k) {
                    env.locals.insert(*k, v.clone());
                }
            }
            if let Some(fr) = heap.functions.get_mut(fid) {
                for (k, v) in &child.locals {
                    if !param_locals.contains(k) && fr.closure.contains_key(k) {
                        fr.closure.insert(*k, v.clone());
                    }
                }
            }
            let _ = param_names;
            Ok(ret)
        }
    }
}

pub(super) fn builtin_define_property(args: &[JsVal], heap: &mut Heap) -> Result<JsVal, ()> {
    if args.len() < 3 {
        return Err(());
    }
    let JsVal::Obj(oid) = &args[0] else {
        return Err(());
    };
    let key = to_key(&args[1])?;
    let JsVal::Obj(desc_id) = &args[2] else {
        return Err(());
    };
    if heap.has_own(*desc_id, "value") {
        let val = heap.get(*desc_id, "value");
        heap.set(*oid, &key, val);
        return Ok(args[0].clone());
    }
    if heap.has_own(*desc_id, "get") {
        let g = heap.get(*desc_id, "get");
        heap.set(*oid, &key, g);
        return Ok(args[0].clone());
    }
    Ok(args[0].clone())
}

pub(super) fn builtin_gopd(args: &[JsVal], heap: &mut Heap) -> Result<JsVal, ()> {
    if args.len() < 2 {
        return Err(());
    }
    let JsVal::Obj(oid) = &args[0] else {
        return Err(());
    };
    let key = to_key(&args[1])?;
    if !heap.has_own(*oid, &key) {
        return Ok(JsVal::Undef);
    }
    let val = heap.get(*oid, &key);
    let mut props = HashMap::new();
    props.insert("value".into(), val);
    props.insert("writable".into(), JsVal::Bool(true));
    props.insert("enumerable".into(), JsVal::Bool(true));
    props.insert("configurable".into(), JsVal::Bool(true));
    Ok(JsVal::Obj(heap.alloc_obj(props)))
}

pub(super) fn builtin_set_prototype_of(args: &[JsVal], heap: &mut Heap) -> Result<JsVal, ()> {
    if args.len() < 2 {
        return Err(());
    }
    let JsVal::Obj(oid) = &args[0] else {
        return Err(());
    };
    heap.set(*oid, "[[Prototype]]", args[1].clone());
    Ok(args[0].clone())
}

pub(super) fn eval_new(
    callee: &Expr,
    args: &[Arg],
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<JsVal, ()> {
    let c = eval_expr(callee, env, heap, by_id)?;
    let arg_vals = eval_args(args, env, heap, by_id)?;
    match c {
        JsVal::Builtin("WeakMap") => Ok(JsVal::WeakMap(heap.alloc_wm())),
        JsVal::Builtin("TypeError") => Err(()),
        JsVal::Builtin("Proxy") => {
            // new Proxy(target, handler)
            if arg_vals.len() < 2 {
                return Err(());
            }
            let target = arg_vals[0].clone();
            let JsVal::Obj(hid) = &arg_vals[1] else {
                return Err(());
            };
            let get_trap = heap.get(*hid, "get");
            let mut props = HashMap::new();
            props.insert("[[ProxyTarget]]".into(), target);
            props.insert("[[ProxyGet]]".into(), get_trap);
            Ok(JsVal::Obj(heap.alloc_obj(props)))
        }
        _ => Err(()),
    }
}
