use std::collections::HashMap;

use draconic_ir::LocalId;

use super::*;

impl super::Interp {
    pub(crate) fn member_get(&self, 
        obj: &JsVal,
        key: &str,
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        match obj {
            JsVal::Builtin(BuiltinId::GlobalThis) => match key {
                "Object" => Ok(JsVal::Builtin(BuiltinId::Object)),
                "Function" => Ok(JsVal::Builtin(BuiltinId::Function)),
                "Array" => Ok(JsVal::Builtin(BuiltinId::Array)),
                "String" => Ok(JsVal::Builtin(BuiltinId::String)),
                "Boolean" => Ok(JsVal::Builtin(BuiltinId::Boolean)),
                "Error" => Ok(JsVal::Builtin(BuiltinId::Error)),
                "TypeError" => Ok(JsVal::Builtin(BuiltinId::TypeError)),
                "RangeError" => Ok(JsVal::Builtin(BuiltinId::RangeError)),
                "ReferenceError" => Ok(JsVal::Builtin(BuiltinId::ReferenceError)),
                "SyntaxError" => Ok(JsVal::Builtin(BuiltinId::SyntaxError)),
                "URIError" => Ok(JsVal::Builtin(BuiltinId::UriError)),
                "EvalError" => Ok(JsVal::Builtin(BuiltinId::EvalError)),
                "AggregateError" => Ok(JsVal::Builtin(BuiltinId::AggregateError)),
                "parseInt" => Ok(JsVal::Builtin(BuiltinId::ParseInt)),
                "parseFloat" => Ok(JsVal::Builtin(BuiltinId::ParseFloat)),
                "isNaN" => Ok(JsVal::Builtin(BuiltinId::IsNaN)),
                "isFinite" => Ok(JsVal::Builtin(BuiltinId::IsFinite)),
                "NaN" => Ok(JsVal::Num(f64::NAN)),
                "Infinity" => Ok(JsVal::Num(f64::INFINITY)),
                "encodeURI" => Ok(JsVal::Builtin(BuiltinId::EncodeUri)),
                "decodeURI" => Ok(JsVal::Builtin(BuiltinId::DecodeUri)),
                "encodeURIComponent" => Ok(JsVal::Builtin(BuiltinId::EncodeUriComponent)),
                "decodeURIComponent" => Ok(JsVal::Builtin(BuiltinId::DecodeUriComponent)),
                "escape" => Ok(JsVal::Builtin(BuiltinId::Escape)),
                "unescape" => Ok(JsVal::Builtin(BuiltinId::Unescape)),
                "JSON" => Ok(JsVal::Builtin(BuiltinId::Json)),
                "Date" => Ok(JsVal::Builtin(BuiltinId::Date)),
                "RegExp" => Ok(JsVal::Builtin(BuiltinId::RegExp)),
                "Map" => Ok(JsVal::Builtin(BuiltinId::Map)),
                "Set" => Ok(JsVal::Builtin(BuiltinId::Set)),
                "WeakMap" => Ok(JsVal::Builtin(BuiltinId::WeakMap)),
                "WeakSet" => Ok(JsVal::Builtin(BuiltinId::WeakSet)),
                "ArrayBuffer" => Ok(JsVal::Builtin(BuiltinId::ArrayBuffer)),
                "DataView" => Ok(JsVal::Builtin(BuiltinId::DataView)),
                "Uint8Array" => Ok(JsVal::Builtin(BuiltinId::Uint8Array)),
                "Int32Array" => Ok(JsVal::Builtin(BuiltinId::Int32Array)),
                "Float64Array" => Ok(JsVal::Builtin(BuiltinId::Float64Array)),
                "parseUrl" => Ok(JsVal::Builtin(BuiltinId::ParseUrl)),
                "parseQuery" => Ok(JsVal::Builtin(BuiltinId::ParseQuery)),
                "serializeQuery" => Ok(JsVal::Builtin(BuiltinId::SerializeQuery)),
                "parseFlags" => Ok(JsVal::Builtin(BuiltinId::ParseFlags)),
                "flagHelp" => Ok(JsVal::Builtin(BuiltinId::FlagHelp)),
                "undefined" => Ok(JsVal::Undef),
                "globalThis" => Ok(JsVal::Builtin(BuiltinId::GlobalThis)),
                _ => Err(()),
            },
            JsVal::Builtin(BuiltinId::Object) if key == "prototype" => {
                Ok(JsVal::Builtin(BuiltinId::ObjectPrototype))
            }
            JsVal::Builtin(BuiltinId::String) if key == "prototype" => {
                Ok(JsVal::Builtin(BuiltinId::StringPrototype))
            }
            JsVal::Builtin(BuiltinId::Date) if key == "prototype" => {
                Ok(JsVal::Builtin(BuiltinId::DatePrototype))
            }
            JsVal::Builtin(BuiltinId::RegExp) if key == "prototype" => {
                Ok(JsVal::Builtin(BuiltinId::RegExpPrototype))
            }
            // Annex B.2.5 RegExp constructor statics (getters return strings).
            JsVal::Builtin(BuiltinId::RegExp) => {
                regexp_static_get(self, key).map(JsVal::Str).ok_or(())
            }
            JsVal::Builtin(BuiltinId::RegExpPrototype) => regexp_proto_method_builtin(key)
                .map(JsVal::Builtin)
                .ok_or(()),
            JsVal::Builtin(BuiltinId::Object) if key == "getPrototypeOf" => {
                Ok(JsVal::Builtin(BuiltinId::ObjectGetPrototypeOf))
            }
            JsVal::Builtin(BuiltinId::ObjectPrototype) if key == "hasOwnProperty" => {
                Ok(JsVal::Builtin(BuiltinId::HasOwnProperty))
            }
            JsVal::Builtin(BuiltinId::ObjectPrototype) => object_accessor_legacy_builtin(key)
                .map(JsVal::Builtin)
                .ok_or(()),
            JsVal::Builtin(BuiltinId::StringPrototype) => string_annex_method_builtin(key)
                .map(JsVal::Builtin)
                .ok_or(()),
            JsVal::Str(_) => string_annex_method_builtin(key)
                .map(JsVal::Builtin)
                .ok_or(()),
            JsVal::Builtin(BuiltinId::DatePrototype) => {
                date_proto_method_builtin(key).map(JsVal::Builtin).ok_or(())
            }
            JsVal::Builtin(BuiltinId::Array) if key == "isArray" => {
                Ok(JsVal::Builtin(BuiltinId::ArrayIsArray))
            }
            JsVal::Builtin(BuiltinId::Json) => match key {
                "parse" => Ok(JsVal::Builtin(BuiltinId::JsonParse)),
                "stringify" => Ok(JsVal::Builtin(BuiltinId::JsonStringify)),
                _ => Err(()),
            },
            JsVal::Builtin(BuiltinId::Date) => match key {
                "now" => Ok(JsVal::Builtin(BuiltinId::DateNow)),
                "UTC" => Ok(JsVal::Builtin(BuiltinId::DateUtc)),
                _ => Err(()),
            },
            JsVal::ErrorInst {
                name,
                message,
                errors,
            } => match key {
                "name" => Ok(JsVal::Str(name.clone())),
                "message" => Ok(JsVal::Str(message.clone())),
                "errors" => match errors {
                    Some(a) => Ok(JsVal::Array(a.clone())),
                    None => Err(()),
                },
                _ => Err(()),
            },
            JsVal::DateInst { ms } => {
                let _ = ms;
                // Bound methods via eval_method_call; bare get yields callable for typeof paths.
                date_proto_method_builtin(key).map(JsVal::Builtin).ok_or(())
            }
            JsVal::RegExpInst { source, flags } => match key {
                "source" => Ok(JsVal::Str(source.clone())),
                "flags" => Ok(JsVal::Str(flags.clone())),
                _ => Err(()),
            },
            JsVal::MapInst { entries } if key == "size" => Ok(JsVal::Num(entries.len() as f64)),
            JsVal::SetInst { values } if key == "size" => Ok(JsVal::Num(values.len() as f64)),
            JsVal::ArrayBufferInst { bytes, .. } if key == "byteLength" => {
                Ok(JsVal::Num(bytes.borrow().len() as f64))
            }
            JsVal::TypedArrayInst { length, .. } if key == "length" => {
                Ok(JsVal::Num(*length as f64))
            }
            JsVal::TypedArrayInst {
                kind,
                bytes,
                length,
                ..
            } => {
                let idx = key.parse::<usize>().map_err(|_| ())?;
                if idx >= *length {
                    return Ok(JsVal::Undef);
                }
                let off = idx * kind.bytes_per_element();
                let n = read_ta_elem(*kind, &bytes.borrow(), off)?;
                Ok(JsVal::Num(n))
            }
            JsVal::DataViewInst { byte_length, .. } if key == "byteLength" => {
                Ok(JsVal::Num(*byte_length as f64))
            }
            JsVal::Array(elems) if key == "length" => Ok(JsVal::Num(elems.len() as f64)),
            JsVal::Array(elems) => {
                if let Ok(idx) = key.parse::<usize>() {
                    Ok(elems.get(idx).cloned().unwrap_or(JsVal::Undef))
                } else {
                    Err(())
                }
            }
            JsVal::Object { props, proto, .. } => {
                let own = object_own_slot(&props.borrow(), key).cloned();
                if let Some(slot) = own {
                    return match slot {
                        PropSlot::Data(v) => Ok(v),
                        PropSlot::Accessor { get: Some(g), .. } => {
                            let this = obj.clone();
                            match g {
                                JsVal::UserFn { params, body, .. } => {
                                    self.call_user_fn(&params, &body, this, &[], env)
                                }
                                other => self.eval_call(&other, &[], env),
                            }
                        }
                        PropSlot::Accessor { get: None, .. } => Ok(JsVal::Undef),
                    };
                }
                // Annex B accessor: missing own `__proto__` → [[Prototype]].
                if key == "__proto__" {
                    return Ok((**proto).clone());
                }
                match proto.as_ref() {
                    JsVal::Builtin(BuiltinId::ObjectPrototype) => {
                        if key == "hasOwnProperty" {
                            Ok(JsVal::Builtin(BuiltinId::HasOwnProperty))
                        } else if let Some(b) = object_accessor_legacy_builtin(key) {
                            Ok(JsVal::Builtin(b))
                        } else {
                            // Missing prop on ordinary object → undefined (not error).
                            Ok(JsVal::Undef)
                        }
                    }
                    JsVal::Null => Ok(JsVal::Undef),
                    // Prototype object may hold accessors (class instance get/set).
                    other => {
                        // Prefer own-slot get on proto with `this` = receiver.
                        if let JsVal::Object { props: pprops, .. } = other {
                            if let Some(slot) = object_own_slot(&pprops.borrow(), key).cloned() {
                                return match slot {
                                    PropSlot::Data(v) => Ok(v),
                                    PropSlot::Accessor { get: Some(g), .. } => {
                                        let this = obj.clone();
                                        match g {
                                            JsVal::UserFn { params, body, .. } => {
                                                self.call_user_fn(&params, &body, this, &[], env)
                                            }
                                            gother => self.eval_call(&gother, &[], env),
                                        }
                                    }
                                    PropSlot::Accessor { get: None, .. } => Ok(JsVal::Undef),
                                };
                            }
                        }
                        match self.member_get(&other.clone(), key, env) {
                            Ok(v) => Ok(v),
                            Err(()) => Ok(JsVal::Undef),
                        }
                    }
                }
            }
            JsVal::UserFn { props, .. } => {
                let own = props
                    .borrow()
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, s)| s.clone());
                if let Some(slot) = own {
                    return match slot {
                        PropSlot::Data(v) => Ok(v),
                        PropSlot::Accessor { get: Some(g), .. } => {
                            let this = obj.clone();
                            match g {
                                JsVal::UserFn { params, body, .. } => {
                                    self.call_user_fn(&params, &body, this, &[], env)
                                }
                                other => self.eval_call(&other, &[], env),
                            }
                        }
                        PropSlot::Accessor { get: None, .. } => Ok(JsVal::Undef),
                    };
                }
                // Function [[Prototype]] — missing own props are undefined.
                match self.member_get(&JsVal::Builtin(BuiltinId::FunctionPrototype), key, env) {
                    Ok(v) => Ok(v),
                    Err(()) => Ok(JsVal::Undef),
                }
            }
            JsVal::Builtin(BuiltinId::Function) if key == "prototype" => {
                Ok(JsVal::Builtin(BuiltinId::FunctionPrototype))
            }
            _ => Err(()),
        }
    }

    pub(crate) fn member_set(&self, 
        obj: &mut JsVal,
        key: &str,
        val: JsVal,
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<(), ()> {
        match obj {
            JsVal::TypedArrayInst {
                kind,
                bytes,
                length,
                ..
            } => {
                let idx = key.parse::<usize>().map_err(|_| ())?;
                if idx >= *length {
                    return Err(());
                }
                let n = match val {
                    JsVal::Num(n) => n,
                    _ => return Err(()),
                };
                let off = idx * kind.bytes_per_element();
                write_ta_elem(*kind, &mut bytes.borrow_mut(), off, n)
            }
            JsVal::Object { props, proto, .. } => {
                if key == "__proto__" && !object_own_has(&props.borrow(), "__proto__") {
                    **proto = val;
                    return Ok(());
                }
                let existing = object_own_slot(&props.borrow(), key).cloned();
                match existing {
                    Some(PropSlot::Data(_)) => {
                        object_set_data(&mut props.borrow_mut(), key.to_string(), val);
                        Ok(())
                    }
                    Some(PropSlot::Accessor { set: Some(s), .. }) => {
                        let this = obj.clone();
                        match s {
                            JsVal::UserFn { params, body, .. } => {
                                self.call_user_fn(&params, &body, this, &[val], env)?;
                            }
                            other => {
                                self.eval_call(&other, &[val], env)?;
                            }
                        }
                        Ok(())
                    }
                    Some(PropSlot::Accessor { set: None, .. }) => Ok(()),
                    None => {
                        // Own miss: walk prototype accessors (class instance fields on proto).
                        if let Some(setter) = proto_lookup_setter(proto, key) {
                            let this = obj.clone();
                            match setter {
                                JsVal::UserFn { params, body, .. } => {
                                    self.call_user_fn(&params, &body, this, &[val], env)?;
                                }
                                other => {
                                    self.eval_call(&other, &[val], env)?;
                                }
                            }
                            return Ok(());
                        }
                        props
                            .borrow_mut()
                            .push((key.to_string(), PropSlot::Data(val)));
                        Ok(())
                    }
                }
            }
            JsVal::UserFn { props, .. } => {
                let existing = props
                    .borrow()
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, s)| s.clone());
                match existing {
                    Some(PropSlot::Data(_)) => {
                        object_set_data(&mut props.borrow_mut(), key.to_string(), val);
                        Ok(())
                    }
                    Some(PropSlot::Accessor { set: Some(s), .. }) => {
                        let this = obj.clone();
                        match s {
                            JsVal::UserFn { params, body, .. } => {
                                self.call_user_fn(&params, &body, this, &[val], env)?;
                            }
                            other => {
                                self.eval_call(&other, &[val], env)?;
                            }
                        }
                        Ok(())
                    }
                    Some(PropSlot::Accessor { set: None, .. }) => Ok(()),
                    None => {
                        props
                            .borrow_mut()
                            .push((key.to_string(), PropSlot::Data(val)));
                        Ok(())
                    }
                }
            }
            _ => Err(()),
        }
    }
}
