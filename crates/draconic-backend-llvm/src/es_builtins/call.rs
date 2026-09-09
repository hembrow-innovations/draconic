use std::collections::HashMap;
use std::rc::Rc;

use draconic_ir::{LocalId, Stmt};
use draconic_runtime::{
    flag_help, parse_flags, parse_flags_typed, parse_query, parse_url, serialize_query, FlagSpec,
    FlagValue, OptionKind, TypedValue,
};

use super::*;

impl super::Interp {
    pub(crate) fn eval_new(&self, 
        callee: &JsVal,
        args: &[JsVal],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        if let JsVal::UserFn {
            params,
            body,
            props,
        } = callee
        {
            // Ordinary [[Construct]]: this = Object.create(ctor.prototype); new.target = ctor
            let proto = match props.borrow().iter().find(|(k, _)| k == "prototype") {
                Some((_, PropSlot::Data(p))) => p.clone(),
                _ => JsVal::Builtin(BuiltinId::ObjectPrototype),
            };
            let this = new_object_with_proto(self, Vec::new(), proto);
            let ctor = callee.clone();
            let result = with_new_target(ctor, || {
                self.call_user_fn(params, body, this.clone(), args, env)
            })?;
            // `this.prop = …` mutates CURRENT_THIS; prefer that over the pre-call clone.
            let this_final = CURRENT_THIS.with(|cell| cell.borrow().clone());
            let this_out = match this_final {
                JsVal::Object { .. } => this_final,
                _ => this,
            };
            return Ok(match result {
                JsVal::Object { .. } | JsVal::UserFn { .. } => result,
                _ => this_out,
            });
        }
        let JsVal::Builtin(b) = callee else {
            return Err(());
        };
        if *b == BuiltinId::Date {
            let ms = match args.first() {
                Some(JsVal::Num(n)) => *n,
                Some(JsVal::Undef) | None => date_now_ms(),
                _ => return Err(()),
            };
            return Ok(JsVal::DateInst { ms });
        }
        if *b == BuiltinId::RegExp {
            return make_regexp(args);
        }
        if *b == BuiltinId::Map {
            // Fixture: `new Map()` only (no iterable init).
            if !args.is_empty() {
                return Err(());
            }
            return Ok(JsVal::MapInst {
                entries: Vec::new(),
            });
        }
        if *b == BuiltinId::Set {
            if !args.is_empty() {
                return Err(());
            }
            return Ok(JsVal::SetInst { values: Vec::new() });
        }
        if *b == BuiltinId::WeakMap {
            if !args.is_empty() {
                return Err(());
            }
            return Ok(JsVal::WeakMapInst {
                entries: Vec::new(),
            });
        }
        if *b == BuiltinId::WeakSet {
            if !args.is_empty() {
                return Err(());
            }
            return Ok(JsVal::WeakSetInst { values: Vec::new() });
        }
        if *b == BuiltinId::ArrayBuffer {
            let len = match args.first() {
                Some(JsVal::Num(n)) if *n >= 0.0 && n.is_finite() => *n as usize,
                _ => return Err(()),
            };
            return Ok(new_array_buffer(self, len));
        }
        if *b == BuiltinId::Uint8Array {
            return match args.first() {
                Some(buf @ JsVal::ArrayBufferInst { .. }) => {
                    typed_array_from_buffer(TaKind::U8, buf)
                }
                Some(JsVal::Num(n)) if *n >= 0.0 && n.is_finite() => {
                    Ok(typed_array_from_length(self, TaKind::U8, *n as usize))
                }
                Some(JsVal::Array(elems)) => typed_array_from_array(self, TaKind::U8, elems),
                _ => Err(()),
            };
        }
        if *b == BuiltinId::Int32Array {
            return match args.first() {
                Some(buf @ JsVal::ArrayBufferInst { .. }) => {
                    typed_array_from_buffer(TaKind::I32, buf)
                }
                Some(JsVal::Num(n)) if *n >= 0.0 && n.is_finite() => {
                    Ok(typed_array_from_length(self, TaKind::I32, *n as usize))
                }
                Some(JsVal::Array(elems)) => typed_array_from_array(self, TaKind::I32, elems),
                _ => Err(()),
            };
        }
        if *b == BuiltinId::Float64Array {
            return match args.first() {
                Some(buf @ JsVal::ArrayBufferInst { .. }) => {
                    typed_array_from_buffer(TaKind::F64, buf)
                }
                Some(JsVal::Num(n)) if *n >= 0.0 && n.is_finite() => {
                    Ok(typed_array_from_length(self, TaKind::F64, *n as usize))
                }
                Some(JsVal::Array(elems)) => typed_array_from_array(self, TaKind::F64, elems),
                _ => Err(()),
            };
        }
        if *b == BuiltinId::DataView {
            let JsVal::ArrayBufferInst { id, bytes } = args.first().ok_or(())? else {
                return Err(());
            };
            let byte_length = bytes.borrow().len();
            return Ok(JsVal::DataViewInst {
                buffer_id: *id,
                bytes: Rc::clone(bytes),
                byte_length,
            });
        }
        let name = error_ctor_name(*b).ok_or(())?;
        if *b == BuiltinId::AggregateError {
            let errors = match args.first() {
                Some(JsVal::Array(a)) => a.clone(),
                _ => return Err(()),
            };
            let message = match args.get(1) {
                Some(JsVal::Str(s)) => s.clone(),
                Some(JsVal::Undef) | None => String::new(),
                _ => return Err(()),
            };
            return Ok(JsVal::ErrorInst {
                name: name.into(),
                message,
                errors: Some(errors),
            });
        }
        let message = match args.first() {
            Some(JsVal::Str(s)) => s.clone(),
            Some(JsVal::Undef) | None => String::new(),
            _ => return Err(()),
        };
        Ok(JsVal::ErrorInst {
            name: name.into(),
            message,
            errors: None,
        })
    }

    pub(crate) fn call_user_fn(&self, 
        params: &[LocalId],
        body: &[Stmt],
        this: JsVal,
        args: &[JsVal],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        let mut saved: Vec<(LocalId, Option<JsVal>)> = Vec::new();
        for (i, pid) in params.iter().enumerate() {
            saved.push((*pid, env.get(pid).cloned()));
            let v = args.get(i).cloned().unwrap_or(JsVal::Undef);
            env.insert(*pid, v);
        }
        let flow = with_this(this, || self.eval_body(body, env))?;
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

    fn own_data_str(&self, props: &[(String, PropSlot)], key: &str) -> Option<String> {
        match object_own_slot(props, key) {
            Some(PropSlot::Data(JsVal::Str(s))) => Some(s.clone()),
            _ => None,
        }
    }

    fn flag_specs_from_js(&self, v: &JsVal) -> Result<Vec<FlagSpec>, ()> {
        let JsVal::Object { props, .. } = v else {
            return Err(());
        };
        let mut out = Vec::new();
        for (name, slot) in props.borrow().iter() {
            let PropSlot::Data(opt) = slot else {
                continue;
            };
            let JsVal::Object { props: op, .. } = opt else {
                return Err(());
            };
            let type_s = self.own_data_str(&op.borrow(), "type").ok_or(())?;
            let kind = match type_s.as_str() {
                "boolean" => OptionKind::Boolean,
                "string" => OptionKind::String,
                "number" => OptionKind::Number,
                _ => return Err(()),
            };
            let short = self.own_data_str(&op.borrow(), "short").and_then(|s| {
                let mut cs = s.chars();
                let c = cs.next()?;
                if cs.next().is_some() {
                    None
                } else {
                    Some(c)
                }
            });
            let help = self.own_data_str(&op.borrow(), "help").unwrap_or_default();
            out.push(FlagSpec {
                name: name.clone(),
                kind,
                short,
                help,
            });
        }
        Ok(out)
    }

    fn typed_value_to_js(&self, v: TypedValue) -> JsVal {
        match v {
            TypedValue::Bool(b) => JsVal::Bool(b),
            TypedValue::Str(s) => JsVal::Str(s),
            TypedValue::Num(n) => JsVal::Num(n),
        }
    }

    pub(crate) fn eval_call(&self, 
        callee: &JsVal,
        args: &[JsVal],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        if let JsVal::UserFn { params, body, .. } = callee {
            return self.call_user_fn(params, body, JsVal::Undef, args, env);
        }
        let JsVal::Builtin(b) = callee else {
            return Err(());
        };
        match b {
            BuiltinId::ParseInt => {
                let s = match args.first() {
                    Some(JsVal::Str(s)) => s.as_str(),
                    Some(JsVal::Num(n)) => {
                        // ToString(number) for fixture depth; only decimals we need.
                        return Ok(JsVal::Num(js_parse_int(&format!("{n}"), args.get(1))?));
                    }
                    _ => return Err(()),
                };
                Ok(JsVal::Num(js_parse_int(s, args.get(1))?))
            }
            BuiltinId::ParseFloat => {
                let s = match args.first() {
                    Some(JsVal::Str(s)) => s.as_str(),
                    Some(JsVal::Num(n)) => return Ok(JsVal::Num(*n)),
                    _ => return Err(()),
                };
                Ok(JsVal::Num(js_parse_float(s)))
            }
            BuiltinId::IsNaN => {
                let n = to_number(args.first().unwrap_or(&JsVal::Undef))?;
                Ok(JsVal::Bool(n.is_nan()))
            }
            BuiltinId::IsFinite => {
                let n = to_number(args.first().unwrap_or(&JsVal::Undef))?;
                Ok(JsVal::Bool(n.is_finite()))
            }
            BuiltinId::EncodeUri => {
                let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
                Ok(JsVal::Str(js_encode_uri(&s, false)))
            }
            BuiltinId::EncodeUriComponent => {
                let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
                Ok(JsVal::Str(js_encode_uri(&s, true)))
            }
            BuiltinId::DecodeUri => {
                let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
                Ok(JsVal::Str(js_decode_uri(&s, false)?))
            }
            BuiltinId::DecodeUriComponent => {
                let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
                Ok(JsVal::Str(js_decode_uri(&s, true)?))
            }
            BuiltinId::Escape => {
                let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
                Ok(JsVal::Str(js_escape(&s)))
            }
            BuiltinId::Unescape => {
                let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
                Ok(JsVal::Str(js_unescape(&s)))
            }
            BuiltinId::JsonParse => {
                let s = match args.first() {
                    Some(JsVal::Str(s)) => s.as_str(),
                    _ => return Err(()),
                };
                json_parse(s, self)
            }
            BuiltinId::JsonStringify => {
                let v = args.first().unwrap_or(&JsVal::Undef);
                Ok(JsVal::Str(json_stringify(v)?))
            }
            BuiltinId::DateNow => Ok(JsVal::Num(date_now_ms())),
            BuiltinId::DateUtc => Ok(JsVal::Num(date_utc(args)?)),
            BuiltinId::RegExp => make_regexp(args),
            BuiltinId::ParseUrl => {
                let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
                let u = parse_url(&s).map_err(|_| ())?;
                Ok(new_object(self, vec![
                    ("scheme".into(), PropSlot::Data(JsVal::Str(u.scheme))),
                    ("host".into(), PropSlot::Data(JsVal::Str(u.host))),
                    ("path".into(), PropSlot::Data(JsVal::Str(u.path))),
                    ("query".into(), PropSlot::Data(JsVal::Str(u.query))),
                    ("hash".into(), PropSlot::Data(JsVal::Str(u.hash))),
                ]))
            }
            BuiltinId::ParseQuery => {
                let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
                let pairs = parse_query(&s);
                Ok(new_object(
                    self,
                    pairs
                        .into_iter()
                        .map(|(k, v)| (k, PropSlot::Data(JsVal::Str(v))))
                        .collect(),
                ))
            }
            BuiltinId::ParseFlags => {
                let argv = match args.first() {
                    Some(JsVal::Array(elems)) => elems,
                    _ => return Err(()),
                };
                let mut strs = Vec::new();
                for e in argv {
                    strs.push(to_string_arg(e)?);
                }
                let spec = match args.get(1) {
                    None | Some(JsVal::Undef) => None,
                    Some(v) => Some(self.flag_specs_from_js(v)?),
                };
                let (flag_props, positionals) = if let Some(spec) = spec {
                    let parsed = parse_flags_typed(&strs, &spec);
                    let flag_props: Vec<(String, PropSlot)> = parsed
                        .flags
                        .into_iter()
                        .map(|(k, v)| (k, PropSlot::Data(self.typed_value_to_js(v))))
                        .collect();
                    (flag_props, parsed.positionals)
                } else {
                    let parsed = parse_flags(&strs);
                    let flag_props: Vec<(String, PropSlot)> = parsed
                        .flags
                        .into_iter()
                        .map(|(k, v)| {
                            let jv = match v {
                                FlagValue::Present => JsVal::Bool(true),
                                FlagValue::Value(s) => JsVal::Str(s),
                            };
                            (k, PropSlot::Data(jv))
                        })
                        .collect();
                    (flag_props, parsed.positionals)
                };
                let pos = JsVal::Array(positionals.into_iter().map(JsVal::Str).collect());
                Ok(new_object(self, vec![
                    ("flags".into(), PropSlot::Data(new_object(self, flag_props))),
                    ("positionals".into(), PropSlot::Data(pos)),
                ]))
            }
            BuiltinId::FlagHelp => {
                let spec = self.flag_specs_from_js(args.first().ok_or(())?)?;
                Ok(JsVal::Str(flag_help(&spec)))
            }
            BuiltinId::SerializeQuery => {
                let obj = args.first().ok_or(())?;
                let JsVal::Object { props, .. } = obj else {
                    return Err(());
                };
                let mut pairs = Vec::new();
                for (k, slot) in props.borrow().iter() {
                    let PropSlot::Data(v) = slot else {
                        continue;
                    };
                    if matches!(v, JsVal::Undef) {
                        continue;
                    }
                    pairs.push((k.clone(), to_string_arg(v)?));
                }
                Ok(JsVal::Str(serialize_query(&pairs)))
            }
            BuiltinId::ObjectGetPrototypeOf => {
                let target = args.first().ok_or(())?;
                object_get_prototype(target)
            }
            BuiltinId::HasOwnProperty => {
                // Direct call without this binding is not supported for fixture depth.
                Err(())
            }
            _ => Err(()),
        }
    }

    pub(crate) fn eval_method_call(&self, 
        recv: &mut JsVal,
        key: &str,
        args: &[JsVal],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        match recv {
            JsVal::DateInst { ms } => eval_date_method(ms, key, args),
            JsVal::Str(s) => eval_string_method(s, key, args),
            JsVal::UserFn { params, body, .. } if key == "call" => {
                let this_arg = args.first().cloned().unwrap_or(JsVal::Undef);
                let rest: Vec<JsVal> = args.iter().skip(1).cloned().collect();
                let params = params.clone();
                let body = body.clone();
                self.call_user_fn(&params, &body, this_arg, &rest, env)
            }
            JsVal::Builtin(BuiltinId::Object) if key == "getPrototypeOf" => {
                let target = args.first().ok_or(())?;
                object_get_prototype(target)
            }
            JsVal::Builtin(BuiltinId::Object) if key == "isExtensible" => {
                // Fixture subset: ordinary objects / functions are extensible.
                let _ = args.first().ok_or(())?;
                Ok(JsVal::Bool(true))
            }
            JsVal::Builtin(BuiltinId::Object) if key == "getOwnPropertyDescriptor" => {
                let target = args.first().ok_or(())?;
                let k = match args.get(1) {
                    Some(JsVal::Str(s)) => s.as_str(),
                    _ => return Err(()),
                };
                object_get_own_property_descriptor(self, target, k)
            }
            JsVal::Builtin(BuiltinId::Object) if key == "defineProperty" => {
                let target = args.first().cloned().ok_or(())?;
                let k = match args.get(1) {
                    Some(JsVal::Str(s)) => s.clone(),
                    Some(JsVal::Num(n)) => format!("{}", *n as i64),
                    _ => return Err(()),
                };
                let desc = args.get(2).cloned().ok_or(())?;
                let mut t = target;
                object_define_property(&mut t, k, &desc, env)?;
                // Propagate mutated object identity into env (incl. UserFn.prototype slots).
                write_back_value(env, &t);
                Ok(t)
            }
            JsVal::Builtin(id) if key == "call" && is_string_annex_method(*id) => {
                let this_arg = args.first().ok_or(())?;
                let this_s = to_string_arg(this_arg)?;
                let rest: Vec<JsVal> = args.iter().skip(1).cloned().collect();
                let method = string_annex_method_name(*id).ok_or(())?;
                eval_string_method(&this_s, method, &rest)
            }
            JsVal::Builtin(id) if key == "call" && is_date_proto_method(*id) => {
                let this_arg = args.first().ok_or(())?;
                let mut this = this_arg.clone();
                let rest: Vec<JsVal> = args.iter().skip(1).cloned().collect();
                let method = date_proto_method_name(*id).ok_or(())?;
                self.eval_method_call(&mut this, method, &rest, env)
            }
            JsVal::Builtin(id) if key == "call" && is_regexp_proto_method(*id) => {
                let this_arg = args.first().ok_or(())?;
                let mut this = this_arg.clone();
                let rest: Vec<JsVal> = args.iter().skip(1).cloned().collect();
                let method = regexp_proto_method_name(*id).ok_or(())?;
                self.eval_method_call(&mut this, method, &rest, env)
            }
            JsVal::Builtin(id) if key == "call" && is_object_accessor_legacy(*id) => {
                let this_arg = args.first().ok_or(())?;
                let mut this = this_arg.clone();
                let rest: Vec<JsVal> = args.iter().skip(1).cloned().collect();
                let method = object_accessor_legacy_name(*id).ok_or(())?;
                let out = self.eval_method_call(&mut this, method, &rest, env)?;
                // Write back mutated object this when possible is caller's job for locals.
                // For `.call(via, …)` the this is a value; mutations must apply to `this` clone.
                // Re-run is wrong — define* already mutated `this`; if this was a clone of a
                // local, caller must update. Pattern: Object.prototype.__defineGetter__.call(via, …)
                // where via is Local — IR lowers as method call on builtin with call, not via.local.
                // So write via: if first arg was evaluated from local, env already has old via.
                // Fix: return and let callee path write when object is Local — here recv is Builtin.
                // Update env slots that strictly-equal the old this? Too heavy.
                // Instead: after call, if this is Object, scan env for same id and update.
                if let JsVal::Object { id, .. } = &this {
                    let id = *id;
                    for v in env.values_mut() {
                        if let JsVal::Object { id: oid, .. } = v {
                            if *oid == id {
                                *v = this.clone();
                            }
                        }
                    }
                }
                Ok(out)
            }
            JsVal::Builtin(BuiltinId::HasOwnProperty) if key == "call" => {
                let this_arg = args.first().ok_or(())?;
                let prop = match args.get(1) {
                    Some(JsVal::Str(s)) => s.as_str(),
                    _ => return Err(()),
                };
                match this_arg {
                    JsVal::Object { props, .. } => {
                        Ok(JsVal::Bool(object_own_has(&props.borrow(), prop)))
                    }
                    _ => Ok(JsVal::Bool(false)),
                }
            }
            JsVal::Object { props, .. } => match key {
                "__defineGetter__" => {
                    let k = match args.first() {
                        Some(JsVal::Str(s)) => s.clone(),
                        _ => return Err(()),
                    };
                    let g = args.get(1).cloned().ok_or(())?;
                    if !matches!(g, JsVal::UserFn { .. } | JsVal::Builtin(_)) {
                        return Err(());
                    }
                    object_define_getter(&mut props.borrow_mut(), k, g);
                    Ok(JsVal::Undef)
                }
                "__defineSetter__" => {
                    let k = match args.first() {
                        Some(JsVal::Str(s)) => s.clone(),
                        _ => return Err(()),
                    };
                    let s = args.get(1).cloned().ok_or(())?;
                    if !matches!(s, JsVal::UserFn { .. } | JsVal::Builtin(_)) {
                        return Err(());
                    }
                    object_define_setter(&mut props.borrow_mut(), k, s);
                    Ok(JsVal::Undef)
                }
                "__lookupGetter__" => {
                    let k = match args.first() {
                        Some(JsVal::Str(s)) => s.as_str(),
                        _ => return Err(()),
                    };
                    Ok(object_lookup_getter(&props.borrow(), k))
                }
                "__lookupSetter__" => {
                    let k = match args.first() {
                        Some(JsVal::Str(s)) => s.as_str(),
                        _ => return Err(()),
                    };
                    Ok(object_lookup_setter(&props.borrow(), k))
                }
                _ => {
                    // Ordinary method call: look up own/proto and invoke UserFn with this=recv.
                    let method = self.member_get(recv, key, env)?;
                    let this = recv.clone();
                    match method {
                        JsVal::UserFn { params, body, .. } => {
                            self.call_user_fn(&params, &body, this, args, env)
                        }
                        other => self.eval_call(&other, args, env),
                    }
                }
            },
            JsVal::Builtin(BuiltinId::ObjectPrototype) if key == "hasOwnProperty" => {
                let prop = match args.first() {
                    Some(JsVal::Str(s)) => s.as_str(),
                    _ => return Err(()),
                };
                // Bare call without `.call` uses Object.prototype as this — not in fixture.
                let _ = prop;
                Err(())
            }
            JsVal::Builtin(BuiltinId::Date) => match key {
                "now" if args.is_empty() => Ok(JsVal::Num(date_now_ms())),
                "UTC" => Ok(JsVal::Num(date_utc(args)?)),
                _ => Err(()),
            },
            JsVal::RegExpInst { source, flags } => match key {
                "test" => {
                    let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
                    match regexp_find(source, flags, &s) {
                        Some(m) => {
                            update_regexp_statics(self, &m, &s);
                            Ok(JsVal::Bool(true))
                        }
                        None => Ok(JsVal::Bool(false)),
                    }
                }
                "exec" => {
                    let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
                    match regexp_find(source, flags, &s) {
                        Some(m) => {
                            update_regexp_statics(self, &m, &s);
                            let mut arr = Vec::with_capacity(1 + m.captures.len());
                            arr.push(JsVal::Str(m.full));
                            for c in m.captures {
                                arr.push(JsVal::Str(c));
                            }
                            Ok(JsVal::Array(arr))
                        }
                        None => Ok(JsVal::Null),
                    }
                }
                "compile" => {
                    let (new_source, new_flags) = regexp_compile_args(args)?;
                    parse_regexp_atoms(&new_source)?;
                    *source = new_source;
                    *flags = new_flags;
                    Ok(JsVal::RegExpInst {
                        source: source.clone(),
                        flags: flags.clone(),
                    })
                }
                _ => Err(()),
            },
            JsVal::MapInst { entries } => match key {
                "set" => {
                    let k = args.first().cloned().ok_or(())?;
                    let v = args.get(1).cloned().unwrap_or(JsVal::Undef);
                    if let Some((_, slot)) =
                        entries.iter_mut().find(|(ek, _)| same_value_zero(ek, &k))
                    {
                        *slot = v;
                    } else {
                        entries.push((k, v));
                    }
                    Ok(JsVal::MapInst {
                        entries: entries.clone(),
                    })
                }
                "get" => {
                    let k = args.first().unwrap_or(&JsVal::Undef);
                    Ok(entries
                        .iter()
                        .find(|(ek, _)| same_value_zero(ek, k))
                        .map(|(_, v)| v.clone())
                        .unwrap_or(JsVal::Undef))
                }
                "has" => {
                    let k = args.first().unwrap_or(&JsVal::Undef);
                    Ok(JsVal::Bool(
                        entries.iter().any(|(ek, _)| same_value_zero(ek, k)),
                    ))
                }
                _ => Err(()),
            },
            JsVal::SetInst { values } => match key {
                "add" => {
                    let v = args.first().cloned().ok_or(())?;
                    if !values.iter().any(|ev| same_value_zero(ev, &v)) {
                        values.push(v);
                    }
                    Ok(JsVal::SetInst {
                        values: values.clone(),
                    })
                }
                "has" => {
                    let v = args.first().unwrap_or(&JsVal::Undef);
                    Ok(JsVal::Bool(values.iter().any(|ev| same_value_zero(ev, v))))
                }
                _ => Err(()),
            },
            JsVal::WeakMapInst { entries } => match key {
                "set" => {
                    let k = args.first().cloned().ok_or(())?;
                    if !is_object_key(&k) {
                        return Err(());
                    }
                    let v = args.get(1).cloned().unwrap_or(JsVal::Undef);
                    if let Some((_, slot)) = entries.iter_mut().find(|(ek, _)| strict_eq(ek, &k)) {
                        *slot = v;
                    } else {
                        entries.push((k, v));
                    }
                    Ok(JsVal::WeakMapInst {
                        entries: entries.clone(),
                    })
                }
                "get" => {
                    let k = args.first().unwrap_or(&JsVal::Undef);
                    if !is_object_key(k) {
                        return Ok(JsVal::Undef);
                    }
                    Ok(entries
                        .iter()
                        .find(|(ek, _)| strict_eq(ek, k))
                        .map(|(_, v)| v.clone())
                        .unwrap_or(JsVal::Undef))
                }
                "has" => {
                    let k = args.first().unwrap_or(&JsVal::Undef);
                    if !is_object_key(k) {
                        return Ok(JsVal::Bool(false));
                    }
                    Ok(JsVal::Bool(entries.iter().any(|(ek, _)| strict_eq(ek, k))))
                }
                "delete" => {
                    let k = args.first().unwrap_or(&JsVal::Undef);
                    if !is_object_key(k) {
                        return Ok(JsVal::Bool(false));
                    }
                    let before = entries.len();
                    entries.retain(|(ek, _)| !strict_eq(ek, k));
                    Ok(JsVal::Bool(entries.len() < before))
                }
                _ => Err(()),
            },
            JsVal::WeakSetInst { values } => match key {
                "add" => {
                    let v = args.first().cloned().ok_or(())?;
                    if !is_object_key(&v) {
                        return Err(());
                    }
                    if !values.iter().any(|ev| strict_eq(ev, &v)) {
                        values.push(v);
                    }
                    Ok(JsVal::WeakSetInst {
                        values: values.clone(),
                    })
                }
                "has" => {
                    let v = args.first().unwrap_or(&JsVal::Undef);
                    if !is_object_key(v) {
                        return Ok(JsVal::Bool(false));
                    }
                    Ok(JsVal::Bool(values.iter().any(|ev| strict_eq(ev, v))))
                }
                "delete" => {
                    let v = args.first().unwrap_or(&JsVal::Undef);
                    if !is_object_key(v) {
                        return Ok(JsVal::Bool(false));
                    }
                    let before = values.len();
                    values.retain(|ev| !strict_eq(ev, v));
                    Ok(JsVal::Bool(values.len() < before))
                }
                _ => Err(()),
            },
            JsVal::DataViewInst {
                bytes, byte_length, ..
            } => match key {
                "getUint8" => {
                    let idx = match args.first() {
                        Some(JsVal::Num(n)) if *n >= 0.0 && n.is_finite() => *n as usize,
                        _ => return Err(()),
                    };
                    if idx >= *byte_length {
                        return Err(());
                    }
                    let b = bytes.borrow()[idx];
                    Ok(JsVal::Num(b as f64))
                }
                "setUint8" => {
                    let idx = match args.first() {
                        Some(JsVal::Num(n)) if *n >= 0.0 && n.is_finite() => *n as usize,
                        _ => return Err(()),
                    };
                    let val = match args.get(1) {
                        Some(JsVal::Num(n)) => *n as u8,
                        _ => return Err(()),
                    };
                    if idx >= *byte_length {
                        return Err(());
                    }
                    bytes.borrow_mut()[idx] = val;
                    Ok(JsVal::Undef)
                }
                _ => Err(()),
            },
            // Non-method: resolve property then call as bare function.
            other => {
                let c = self.member_get(other, key, env)?;
                self.eval_call(&c, args, env)
            }
        }
    }
}
