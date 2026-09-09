use super::*;

pub(super) fn eval_body(body: &[Stmt], env: &mut HashMap<LocalId, JsVal>) -> Result<Flow, ()> {
    for stmt in body {
        match eval_stmt(stmt, env)? {
            Flow::Normal => {}
            other => return Ok(other),
        }
    }
    Ok(Flow::Normal)
}

pub(super) fn eval_stmt(stmt: &Stmt, env: &mut HashMap<LocalId, JsVal>) -> Result<Flow, ()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => match eval_expr(e, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(flow),
                },
                None => JsVal::Undef,
            };
            env.insert(*local, v);
            Ok(Flow::Normal)
        }
        Stmt::Expr { expr } => match eval_expr(expr, env)? {
            Ok(_) => Ok(Flow::Normal),
            Err(flow) => Ok(flow),
        },
        Stmt::Throw { value } => match eval_expr(value, env)? {
            Ok(v) => Ok(Flow::Throw(v)),
            Err(flow) => Ok(flow),
        },
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            let mut completion = match eval_body(block, env)? {
                Flow::Throw(exc) => {
                    if let Some(handler) = handler {
                        if let Some(Pattern::Local(pid)) = handler_param {
                            env.insert(*pid, exc);
                        }
                        eval_body(handler, env)?
                    } else {
                        Flow::Throw(exc)
                    }
                }
                other => other,
            };
            if let Some(fin) = finalizer {
                match eval_body(fin, env)? {
                    Flow::Normal => {}
                    abrupt => completion = abrupt,
                }
            }
            Ok(completion)
        }
        Stmt::Block { body } => eval_body(body, env),
        _ => Err(()),
    }
}

pub(super) fn eval_expr(expr: &Expr, env: &mut HashMap<LocalId, JsVal>) -> Result<Result<JsVal, Flow>, ()> {
    match expr {
        Expr::Number { raw, .. } => Ok(Ok(JsVal::Num(raw.parse().map_err(|_| ())?))),
        Expr::Boolean { value, .. } => Ok(Ok(JsVal::Bool(*value))),
        Expr::String { value, .. } => Ok(Ok(JsVal::Str(js_string_to_utf8(value)))),
        Expr::Null { .. } => Ok(Ok(JsVal::Undef)),
        Expr::Local { id, .. } => Ok(Ok(env.get(id).cloned().ok_or(())?)),
        Expr::Unary { op, arg, .. } => {
            let v = match eval_expr(arg, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            match op {
                UnaryOp::TypeOf => Ok(Ok(JsVal::Str(typeof_str(&v)))),
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
        Expr::Binary {
            op, left, right, ..
        } => {
            let l = match eval_expr(left, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            let r = match eval_expr(right, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            match op {
                BinaryOp::EqEqEq | BinaryOp::EqEq => Ok(Ok(JsVal::Bool(strict_eq(&l, &r)))),
                BinaryOp::NotEqEq | BinaryOp::NotEq => Ok(Ok(JsVal::Bool(!strict_eq(&l, &r)))),
                BinaryOp::Add => match (&l, &r) {
                    (JsVal::Num(a), JsVal::Num(b)) => Ok(Ok(JsVal::Num(a + b))),
                    (JsVal::Str(a), JsVal::Str(b)) => Ok(Ok(JsVal::Str(format!("{a}{b}")))),
                    _ => Err(()),
                },
                BinaryOp::Sub => match (&l, &r) {
                    (JsVal::Num(a), JsVal::Num(b)) => Ok(Ok(JsVal::Num(a - b))),
                    _ => Err(()),
                },
                BinaryOp::Mul => match (&l, &r) {
                    (JsVal::Num(a), JsVal::Num(b)) => Ok(Ok(JsVal::Num(a * b))),
                    _ => Err(()),
                },
                BinaryOp::Div => match (&l, &r) {
                    (JsVal::Num(a), JsVal::Num(b)) => Ok(Ok(JsVal::Num(a / b))),
                    _ => Err(()),
                },
                _ => Err(()),
            }
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            let t = match eval_expr(test, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            if to_boolean(&t) {
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
                Err(flow) => return Ok(Err(flow)),
            };
            let key = match eval_key(property, env)? {
                Ok(k) => k,
                Err(flow) => return Ok(Err(flow)),
            };
            Ok(Ok(member_get(&obj, &key)?))
        }
        Expr::New { callee, args, .. } => {
            let c = match eval_expr(callee, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            let mut arg_vals = Vec::new();
            for a in args {
                match a {
                    Arg::Expr(e) => match eval_expr(e, env)? {
                        Ok(v) => arg_vals.push(v),
                        Err(flow) => return Ok(Err(flow)),
                    },
                    _ => return Err(()),
                }
            }
            match eval_new(&c, &arg_vals) {
                Ok(v) => Ok(Ok(v)),
                Err(Some(flow)) => Ok(Err(flow)),
                Err(None) => Err(()),
            }
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            let mut arg_vals = Vec::new();
            for a in args {
                match a {
                    Arg::Expr(e) => match eval_expr(e, env)? {
                        Ok(v) => arg_vals.push(v),
                        Err(flow) => return Ok(Err(flow)),
                    },
                    _ => return Err(()),
                }
            }
            if let Expr::Member {
                object,
                property,
                optional: false,
                ..
            } = callee.as_ref()
            {
                let obj = match eval_expr(object, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                let key = match eval_key(property, env)? {
                    Ok(k) => k,
                    Err(flow) => return Ok(Err(flow)),
                };
                return match eval_method_call(&obj, &key, &arg_vals) {
                    Ok(v) => Ok(Ok(v)),
                    Err(Some(flow)) => Ok(Err(flow)),
                    Err(None) => Err(()),
                };
            }
            let c = match eval_expr(callee, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            match eval_call_fn(&c, &arg_vals) {
                Ok(v) => Ok(Ok(v)),
                Err(Some(flow)) => Ok(Err(flow)),
                Err(None) => Err(()),
            }
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = match eval_expr(value, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            env.insert(*id, v.clone());
            Ok(Ok(v))
        }
        Expr::Array { elements, .. } => {
            let mut out = Vec::new();
            for el in elements {
                match el {
                    ArrayElement::Expr(e) => match eval_expr(e, env)? {
                        Ok(v) => out.push(v),
                        Err(flow) => return Ok(Err(flow)),
                    },
                    ArrayElement::Elision => out.push(JsVal::Undef),
                    ArrayElement::Spread(_) => return Err(()),
                }
            }
            Ok(Ok(JsVal::Object {
                props: out
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| (i.to_string(), v))
                    .collect(),
            }))
        }
        Expr::Object { properties, .. } => {
            let mut props = Vec::new();
            for p in properties {
                match p {
                    ObjectProp::Property {
                        key: ObjectPropKey::Static(s),
                        value,
                    } => {
                        let v = match eval_expr(value, env)? {
                            Ok(v) => v,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        props.push((js_string_to_utf8(s), v));
                    }
                    ObjectProp::Property {
                        key: ObjectPropKey::Computed(ke),
                        value,
                    } => {
                        let key = match eval_key(ke, env)? {
                            Ok(k) => k,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        let v = match eval_expr(value, env)? {
                            Ok(v) => v,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        props.push((key, v));
                    }
                    _ => return Err(()),
                }
            }
            Ok(Ok(JsVal::Object { props }))
        }
        _ => Err(()),
    }
}

pub(super) fn eval_key(expr: &Expr, env: &mut HashMap<LocalId, JsVal>) -> Result<Result<String, Flow>, ()> {
    match expr {
        Expr::String { value, .. } => Ok(Ok(js_string_to_utf8(value))),
        e => match eval_expr(e, env)? {
            Ok(JsVal::Str(s)) => Ok(Ok(s)),
            Ok(JsVal::Num(n)) => Ok(Ok(format!("{}", n as i64))),
            Ok(_) => Err(()),
            Err(flow) => Ok(Err(flow)),
        },
    }
}

pub(super) fn eval_call_fn(callee: &JsVal, args: &[JsVal]) -> Result<JsVal, Option<Flow>> {
    match callee {
        JsVal::Builtin(BuiltinId::Sha256) => {
            let bytes = match args.first() {
                Some(JsVal::Uint8ArrayInst { bytes }) => bytes.borrow().clone(),
                _ => {
                    return Err(Some(Flow::Throw(JsVal::ErrorInst {
                        name: "TypeError".into(),
                        message: "sha256 expects Uint8Array".into(),
                    })))
                }
            };
            Ok(JsVal::Uint8ArrayInst {
                bytes: Rc::new(RefCell::new(sha256::digest(&bytes).to_vec())),
            })
        }
        JsVal::Builtin(BuiltinId::RandomBytes) => {
            let n = match args.first() {
                Some(JsVal::Num(n)) => *n,
                _ => {
                    return Err(Some(Flow::Throw(JsVal::ErrorInst {
                        name: "TypeError".into(),
                        message: "randomBytes expects a length".into(),
                    })))
                }
            };
            if !n.is_finite() {
                return Err(Some(Flow::Throw(JsVal::ErrorInst {
                    name: "TypeError".into(),
                    message: "randomBytes expects a length".into(),
                })));
            }
            if n < 0.0 || n != n.trunc() || n > 65536.0 {
                return Err(Some(Flow::Throw(JsVal::ErrorInst {
                    name: "RangeError".into(),
                    message: "randomBytes length must be a non-negative integer".into(),
                })));
            }
            let len = n as usize;
            let mut buf = vec![0u8; len];
            if draconic_runtime::crypto::fill_random(&mut buf).is_err() {
                return Err(Some(Flow::Throw(JsVal::ErrorInst {
                    name: "TypeError".into(),
                    message: "randomBytes unavailable".into(),
                })));
            }
            Ok(JsVal::Uint8ArrayInst {
                bytes: Rc::new(RefCell::new(buf)),
            })
        }
        JsVal::Builtin(BuiltinId::HmacSha256) => {
            let key = match args.first() {
                Some(JsVal::Uint8ArrayInst { bytes }) => bytes.borrow().clone(),
                _ => {
                    return Err(Some(Flow::Throw(JsVal::ErrorInst {
                        name: "TypeError".into(),
                        message: "hmacSha256 expects Uint8Array key and message".into(),
                    })))
                }
            };
            let message = match args.get(1) {
                Some(JsVal::Uint8ArrayInst { bytes }) => bytes.borrow().clone(),
                _ => {
                    return Err(Some(Flow::Throw(JsVal::ErrorInst {
                        name: "TypeError".into(),
                        message: "hmacSha256 expects Uint8Array key and message".into(),
                    })))
                }
            };
            Ok(JsVal::Uint8ArrayInst {
                bytes: Rc::new(RefCell::new(hmac::hmac_sha256(&key, &message).to_vec())),
            })
        }
        JsVal::Builtin(id @ (BuiltinId::AeadEncrypt | BuiltinId::AeadDecrypt)) => {
            eval_aead(*id, args)
        }
        JsVal::Builtin(
            id @ (BuiltinId::Gzip | BuiltinId::Gunzip | BuiltinId::Deflate | BuiltinId::Inflate),
        ) => eval_compression(*id, args),
        _ => Err(None),
    }
}

pub(super) fn eval_aead(id: BuiltinId, args: &[JsVal]) -> Result<JsVal, Option<Flow>> {
    let encrypt = matches!(id, BuiltinId::AeadEncrypt);
    let name = if encrypt {
        "aeadEncrypt"
    } else {
        "aeadDecrypt"
    };
    let type_msg = if encrypt {
        "aeadEncrypt expects Uint8Array key, nonce, and plaintext"
    } else {
        "aeadDecrypt expects Uint8Array key, nonce, and ciphertext"
    };
    let key = match args.first() {
        Some(JsVal::Uint8ArrayInst { bytes }) => bytes.borrow().clone(),
        _ => {
            return Err(Some(Flow::Throw(JsVal::ErrorInst {
                name: "TypeError".into(),
                message: type_msg.into(),
            })))
        }
    };
    let nonce = match args.get(1) {
        Some(JsVal::Uint8ArrayInst { bytes }) => bytes.borrow().clone(),
        _ => {
            return Err(Some(Flow::Throw(JsVal::ErrorInst {
                name: "TypeError".into(),
                message: type_msg.into(),
            })))
        }
    };
    let data = match args.get(2) {
        Some(JsVal::Uint8ArrayInst { bytes }) => bytes.borrow().clone(),
        _ => {
            return Err(Some(Flow::Throw(JsVal::ErrorInst {
                name: "TypeError".into(),
                message: type_msg.into(),
            })))
        }
    };
    let out = if encrypt {
        aead::encrypt(&key, &nonce, &data)
    } else {
        aead::decrypt(&key, &nonce, &data)
    };
    match out {
        Ok(buf) => Ok(JsVal::Uint8ArrayInst {
            bytes: Rc::new(RefCell::new(buf)),
        }),
        Err(
            aead::AeadError::KeyLen | aead::AeadError::NonceLen | aead::AeadError::CiphertextLen,
        ) => Err(Some(Flow::Throw(JsVal::ErrorInst {
            name: "RangeError".into(),
            message: format!("{name}: invalid key, nonce, or ciphertext length"),
        }))),
        Err(aead::AeadError::Auth) => Err(Some(Flow::Throw(JsVal::ErrorInst {
            name: "Error".into(),
            message: format!("{name}: authentication failed"),
        }))),
    }
}

pub(super) fn eval_compression(id: BuiltinId, args: &[JsVal]) -> Result<JsVal, Option<Flow>> {
    let name = match id {
        BuiltinId::Gzip => "gzip",
        BuiltinId::Gunzip => "gunzip",
        BuiltinId::Deflate => "deflate",
        BuiltinId::Inflate => "inflate",
        _ => return Err(None),
    };
    let bytes = match args.first() {
        Some(JsVal::Uint8ArrayInst { bytes }) => bytes.borrow().clone(),
        _ => {
            return Err(Some(Flow::Throw(JsVal::ErrorInst {
                name: "TypeError".into(),
                message: format!("{name} expects Uint8Array"),
            })))
        }
    };
    let out = match id {
        BuiltinId::Gzip => compression::gzip(&bytes),
        BuiltinId::Gunzip => compression::gunzip(&bytes),
        BuiltinId::Deflate => compression::deflate(&bytes),
        BuiltinId::Inflate => compression::inflate(&bytes),
        _ => return Err(None),
    };
    match out {
        Ok(buf) => Ok(JsVal::Uint8ArrayInst {
            bytes: Rc::new(RefCell::new(buf)),
        }),
        Err(()) => Err(Some(Flow::Throw(JsVal::ErrorInst {
            name: "Error".into(),
            message: format!("{name}: invalid or truncated input"),
        }))),
    }
}

pub(super) fn eval_new(callee: &JsVal, args: &[JsVal]) -> Result<JsVal, Option<Flow>> {
    let JsVal::Builtin(b) = callee else {
        return Err(None);
    };
    match b {
        BuiltinId::TextEncoder => {
            if !args.is_empty() {
                return Err(None);
            }
            Ok(JsVal::TextEncoderInst)
        }
        BuiltinId::TextDecoder => {
            let mut fatal = false;
            if let Some(label) = args.first() {
                match label {
                    JsVal::Str(s) => {
                        let t = s.to_ascii_lowercase();
                        if !(t.is_empty() || t == "utf-8" || t == "utf8") {
                            return Err(None);
                        }
                    }
                    JsVal::Undef => {}
                    _ => return Err(None),
                }
            }
            if let Some(opts) = args.get(1) {
                match opts {
                    JsVal::Object { props } => {
                        if let Some((_, v)) = props.iter().find(|(k, _)| k == "fatal") {
                            fatal = match v {
                                JsVal::Bool(b) => *b,
                                _ => return Err(None),
                            };
                        }
                    }
                    JsVal::Undef => {}
                    _ => return Err(None),
                }
            }
            Ok(JsVal::TextDecoderInst { fatal })
        }
        BuiltinId::Uint8Array => {
            let first = args.first().ok_or(None)?;
            match first {
                JsVal::Object { props } => {
                    let mut pairs: Vec<(usize, u8)> = Vec::new();
                    for (k, v) in props {
                        let idx: usize = k.parse().map_err(|_| None)?;
                        let n = match v {
                            JsVal::Num(n) => *n as u8,
                            _ => return Err(None),
                        };
                        pairs.push((idx, n));
                    }
                    pairs.sort_by_key(|(i, _)| *i);
                    let mut bytes = vec![0u8; pairs.len()];
                    for (i, (idx, b)) in pairs.iter().enumerate() {
                        if *idx != i {
                            return Err(None);
                        }
                        bytes[i] = *b;
                    }
                    Ok(JsVal::Uint8ArrayInst {
                        bytes: Rc::new(RefCell::new(bytes)),
                    })
                }
                JsVal::Num(n) if *n >= 0.0 && n.is_finite() => Ok(JsVal::Uint8ArrayInst {
                    bytes: Rc::new(RefCell::new(vec![0u8; *n as usize])),
                }),
                _ => Err(None),
            }
        }
        BuiltinId::TypeError => {
            let message = match args.first() {
                Some(JsVal::Str(s)) => s.clone(),
                Some(JsVal::Undef) | None => String::new(),
                _ => return Err(None),
            };
            Ok(JsVal::ErrorInst {
                name: "TypeError".into(),
                message,
            })
        }
        _ => Err(None),
    }
}

pub(super) fn eval_method_call(recv: &JsVal, key: &str, args: &[JsVal]) -> Result<JsVal, Option<Flow>> {
    match recv {
        JsVal::TextEncoderInst if key == "encode" => {
            let s = match args.first() {
                Some(JsVal::Str(s)) => s.as_str(),
                Some(JsVal::Undef) | None => "",
                _ => return Err(None),
            };
            Ok(JsVal::Uint8ArrayInst {
                bytes: Rc::new(RefCell::new(s.as_bytes().to_vec())),
            })
        }
        JsVal::TextDecoderInst { fatal } if key == "decode" => {
            let bytes = match args.first() {
                Some(JsVal::Uint8ArrayInst { bytes }) => bytes.borrow().clone(),
                Some(JsVal::Undef) | None => Vec::new(),
                _ => return Err(None),
            };
            match std::str::from_utf8(&bytes) {
                Ok(s) => Ok(JsVal::Str(s.to_string())),
                Err(_) if *fatal => Err(Some(Flow::Throw(JsVal::ErrorInst {
                    name: "TypeError".into(),
                    message: "The encoded data was not valid for encoding utf-8".into(),
                }))),
                Err(_) => Ok(JsVal::Str(String::from_utf8_lossy(&bytes).into_owned())),
            }
        }
        JsVal::Uint8ArrayInst { bytes } if key == "toBase64" => {
            if !args.is_empty() {
                return Err(None);
            }
            Ok(JsVal::Str(base64::encode(&bytes.borrow())))
        }
        JsVal::Uint8ArrayInst { bytes } if key == "toHex" => {
            if !args.is_empty() {
                return Err(None);
            }
            Ok(JsVal::Str(hex::encode(&bytes.borrow())))
        }
        JsVal::Builtin(BuiltinId::Uint8Array) if key == "fromHex" => {
            let s = match args.first() {
                Some(JsVal::Str(s)) => s.as_str(),
                Some(JsVal::Undef) | None => "",
                _ => return Err(None),
            };
            match hex::decode(s) {
                Ok(out) => Ok(JsVal::Uint8ArrayInst {
                    bytes: Rc::new(RefCell::new(out)),
                }),
                Err(()) => Err(Some(Flow::Throw(JsVal::ErrorInst {
                    name: "SyntaxError".into(),
                    message: "Invalid hex string".into(),
                }))),
            }
        }
        JsVal::Builtin(BuiltinId::Uint8Array) if key == "fromBase64" => {
            let s = match args.first() {
                Some(JsVal::Str(s)) => s.as_str(),
                Some(JsVal::Undef) | None => "",
                _ => return Err(None),
            };
            match base64::decode(s) {
                Ok(out) => Ok(JsVal::Uint8ArrayInst {
                    bytes: Rc::new(RefCell::new(out)),
                }),
                Err(()) => Err(Some(Flow::Throw(JsVal::ErrorInst {
                    name: "SyntaxError".into(),
                    message: "Invalid base64 string".into(),
                }))),
            }
        }
        _ => Err(None),
    }
}

pub(super) fn member_get(obj: &JsVal, key: &str) -> Result<JsVal, ()> {
    match obj {
        JsVal::Builtin(BuiltinId::GlobalThis) => match key {
            "TextEncoder" => Ok(JsVal::Builtin(BuiltinId::TextEncoder)),
            "TextDecoder" => Ok(JsVal::Builtin(BuiltinId::TextDecoder)),
            "Uint8Array" => Ok(JsVal::Builtin(BuiltinId::Uint8Array)),
            "TypeError" => Ok(JsVal::Builtin(BuiltinId::TypeError)),
            "sha256" => Ok(JsVal::Builtin(BuiltinId::Sha256)),
            "randomBytes" => Ok(JsVal::Builtin(BuiltinId::RandomBytes)),
            "hmacSha256" => Ok(JsVal::Builtin(BuiltinId::HmacSha256)),
            "aeadEncrypt" => Ok(JsVal::Builtin(BuiltinId::AeadEncrypt)),
            "aeadDecrypt" => Ok(JsVal::Builtin(BuiltinId::AeadDecrypt)),
            "gzip" => Ok(JsVal::Builtin(BuiltinId::Gzip)),
            "gunzip" => Ok(JsVal::Builtin(BuiltinId::Gunzip)),
            "deflate" => Ok(JsVal::Builtin(BuiltinId::Deflate)),
            "inflate" => Ok(JsVal::Builtin(BuiltinId::Inflate)),
            "globalThis" => Ok(JsVal::Builtin(BuiltinId::GlobalThis)),
            _ => Err(()),
        },
        JsVal::Uint8ArrayInst { bytes } if key == "length" => {
            Ok(JsVal::Num(bytes.borrow().len() as f64))
        }
        JsVal::Uint8ArrayInst { bytes } => {
            let idx: usize = key.parse().map_err(|_| ())?;
            let b = bytes.borrow();
            if idx >= b.len() {
                return Ok(JsVal::Undef);
            }
            Ok(JsVal::Num(b[idx] as f64))
        }
        JsVal::ErrorInst { name, message } => match key {
            "name" => Ok(JsVal::Str(name.clone())),
            "message" => Ok(JsVal::Str(message.clone())),
            _ => Err(()),
        },
        JsVal::Object { props } => props
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .ok_or(()),
        _ => Err(()),
    }
}

pub(super) fn typeof_str(v: &JsVal) -> String {
    match v {
        JsVal::Num(_) => "number".into(),
        JsVal::Bool(_) => "boolean".into(),
        JsVal::Str(_) => "string".into(),
        JsVal::Undef => "undefined".into(),
        JsVal::ErrorInst { .. }
        | JsVal::TextEncoderInst
        | JsVal::TextDecoderInst { .. }
        | JsVal::Uint8ArrayInst { .. }
        | JsVal::Object { .. } => "object".into(),
        JsVal::Builtin(
            BuiltinId::TextEncoder
            | BuiltinId::TextDecoder
            | BuiltinId::Uint8Array
            | BuiltinId::TypeError
            | BuiltinId::Sha256
            | BuiltinId::RandomBytes
            | BuiltinId::HmacSha256
            | BuiltinId::AeadEncrypt
            | BuiltinId::AeadDecrypt
            | BuiltinId::Gzip
            | BuiltinId::Gunzip
            | BuiltinId::Deflate
            | BuiltinId::Inflate,
        ) => "function".into(),
        JsVal::Builtin(BuiltinId::GlobalThis) => "object".into(),
    }
}

pub(super) fn strict_eq(a: &JsVal, b: &JsVal) -> bool {
    match (a, b) {
        (JsVal::Num(x), JsVal::Num(y)) => x == y,
        (JsVal::Bool(x), JsVal::Bool(y)) => x == y,
        (JsVal::Str(x), JsVal::Str(y)) => x == y,
        (JsVal::Undef, JsVal::Undef) => true,
        (JsVal::Builtin(x), JsVal::Builtin(y)) => x == y,
        (
            JsVal::ErrorInst {
                name: n1,
                message: m1,
            },
            JsVal::ErrorInst {
                name: n2,
                message: m2,
            },
        ) => n1 == n2 && m1 == m2,
        _ => false,
    }
}

pub(super) fn to_boolean(v: &JsVal) -> bool {
    match v {
        JsVal::Bool(b) => *b,
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Undef => false,
        _ => true,
    }
}
