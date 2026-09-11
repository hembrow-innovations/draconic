use super::eval::*;

use super::*;

pub(super) fn proxy_or_object_get(
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

pub(super) fn proxy_or_object_set(
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

pub(super) fn proxy_or_object_own_keys(
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

pub(super) fn proxy_or_object_get_prototype_of(
    obj: &JsVal,
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<JsVal, ()> {
    match obj {
        JsVal::Object(idx) => Ok(objects.get(*idx).ok_or(())?.proto.clone()),
        JsVal::Proxy(idx) => {
            let rec = proxies.get(*idx).ok_or(())?.clone();
            if let Some(trap_idx) = rec.get_prototype_of_trap {
                let args = vec![rec.target.clone()];
                call_fn(trap_idx, &args, env, fns, objects, proxies)
            } else {
                proxy_or_object_get_prototype_of(&rec.target, env, fns, objects, proxies)
            }
        }
        _ => Err(()),
    }
}

pub(super) fn proxy_or_object_set_prototype_of(
    obj: &JsVal,
    proto: &JsVal,
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<bool, ()> {
    match proto {
        JsVal::Null | JsVal::Object(_) => {}
        _ => return Err(()),
    }
    match obj {
        JsVal::Object(idx) => {
            let rec = objects.get_mut(*idx).ok_or(())?;
            rec.proto = proto.clone();
            Ok(true)
        }
        JsVal::Proxy(idx) => {
            let rec = proxies.get(*idx).ok_or(())?.clone();
            if let Some(trap_idx) = rec.set_prototype_of_trap {
                let args = vec![rec.target.clone(), proto.clone()];
                let v = call_fn(trap_idx, &args, env, fns, objects, proxies)?;
                Ok(is_truthy(&v))
            } else {
                proxy_or_object_set_prototype_of(&rec.target, proto, env, fns, objects, proxies)
            }
        }
        _ => Err(()),
    }
}

pub(super) fn descriptor_value(desc: &JsVal, objects: &[ObjectRec]) -> Result<JsVal, ()> {
    match desc {
        JsVal::Object(idx) => Ok(objects
            .get(*idx)
            .ok_or(())?
            .props
            .get("value")
            .cloned()
            .unwrap_or(JsVal::Undef)),
        _ => Err(()),
    }
}

pub(super) fn make_data_descriptor(value: JsVal, objects: &mut Vec<ObjectRec>) -> JsVal {
    let mut rec = empty_object();
    object_set_prop(&mut rec, "value".into(), value);
    object_set_prop(&mut rec, "writable".into(), JsVal::Bool(true));
    object_set_prop(&mut rec, "enumerable".into(), JsVal::Bool(true));
    object_set_prop(&mut rec, "configurable".into(), JsVal::Bool(true));
    let idx = objects.len();
    objects.push(rec);
    JsVal::Object(idx)
}

pub(super) fn proxy_or_object_is_extensible(
    obj: &JsVal,
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<bool, ()> {
    match obj {
        JsVal::Object(idx) => Ok(objects.get(*idx).ok_or(())?.extensible),
        JsVal::Proxy(idx) => {
            let rec = proxies.get(*idx).ok_or(())?.clone();
            if let Some(trap_idx) = rec.is_extensible_trap {
                let args = vec![rec.target.clone()];
                let v = call_fn(trap_idx, &args, env, fns, objects, proxies)?;
                Ok(is_truthy(&v))
            } else {
                proxy_or_object_is_extensible(&rec.target, env, fns, objects, proxies)
            }
        }
        _ => Err(()),
    }
}

pub(super) fn proxy_or_object_prevent_extensions(
    obj: &JsVal,
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<bool, ()> {
    match obj {
        JsVal::Object(idx) => {
            let rec = objects.get_mut(*idx).ok_or(())?;
            rec.extensible = false;
            Ok(true)
        }
        JsVal::Proxy(idx) => {
            let rec = proxies.get(*idx).ok_or(())?.clone();
            if let Some(trap_idx) = rec.prevent_extensions_trap {
                let args = vec![rec.target.clone()];
                let v = call_fn(trap_idx, &args, env, fns, objects, proxies)?;
                Ok(is_truthy(&v))
            } else {
                proxy_or_object_prevent_extensions(&rec.target, env, fns, objects, proxies)
            }
        }
        _ => Err(()),
    }
}

pub(super) fn proxy_or_object_define_property(
    obj: &JsVal,
    key: &str,
    desc: &JsVal,
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<bool, ()> {
    match obj {
        JsVal::Object(idx) => {
            let value = descriptor_value(desc, objects)?;
            let rec = objects.get_mut(*idx).ok_or(())?;
            object_set_prop(rec, key.to_string(), value);
            Ok(true)
        }
        JsVal::Proxy(idx) => {
            let rec = proxies.get(*idx).ok_or(())?.clone();
            if let Some(trap_idx) = rec.define_property_trap {
                let args = vec![
                    rec.target.clone(),
                    JsVal::Str(key.to_string()),
                    desc.clone(),
                ];
                let v = call_fn(trap_idx, &args, env, fns, objects, proxies)?;
                Ok(is_truthy(&v))
            } else {
                proxy_or_object_define_property(&rec.target, key, desc, env, fns, objects, proxies)
            }
        }
        _ => Err(()),
    }
}

pub(super) fn proxy_or_object_get_own_property_descriptor(
    obj: &JsVal,
    key: &str,
    env: &mut HashMap<LocalId, JsVal>,
    fns: &mut Vec<FnRec>,
    objects: &mut Vec<ObjectRec>,
    proxies: &mut Vec<ProxyRec>,
) -> Result<JsVal, ()> {
    match obj {
        JsVal::Object(idx) => {
            let value = match objects.get(*idx).ok_or(())?.props.get(key) {
                Some(v) => v.clone(),
                None => return Ok(JsVal::Undef),
            };
            Ok(make_data_descriptor(value, objects))
        }
        JsVal::Proxy(idx) => {
            let rec = proxies.get(*idx).ok_or(())?.clone();
            if let Some(trap_idx) = rec.get_own_property_descriptor_trap {
                let args = vec![rec.target.clone(), JsVal::Str(key.to_string())];
                call_fn(trap_idx, &args, env, fns, objects, proxies)
            } else {
                proxy_or_object_get_own_property_descriptor(
                    &rec.target,
                    key,
                    env,
                    fns,
                    objects,
                    proxies,
                )
            }
        }
        _ => Err(()),
    }
}

pub(super) fn proxy_or_object_has(
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

pub(super) fn proxy_or_object_delete(
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
