use std::collections::HashMap;

use draconic_ast::{AccessorKind, AssignOp, BinaryOp, UnaryOp};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, LocalId, ObjectProp, ObjectPropKey, Pattern, Stmt,
};

use super::*;

impl super::Interp {
    pub(crate) fn eval_body(&self, body: &[Stmt], env: &mut HashMap<LocalId, JsVal>) -> Result<Flow, ()> {
        for stmt in body {
            match self.eval_stmt(stmt, env)? {
                Flow::Normal => {}
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }

    pub(crate) fn eval_stmt(&self, stmt: &Stmt, env: &mut HashMap<LocalId, JsVal>) -> Result<Flow, ()> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let v = match init {
                    Some(e) => match self.eval_expr(e, env)? {
                        Ok(v) => v,
                        Err(flow) => return Ok(flow),
                    },
                    None => JsVal::Undef,
                };
                env.insert(*local, v);
                Ok(Flow::Normal)
            }
            Stmt::Expr { expr } => match self.eval_expr(expr, env)? {
                Ok(_) => Ok(Flow::Normal),
                Err(flow) => Ok(flow),
            },
            Stmt::Throw { value } => match self.eval_expr(value, env)? {
                Ok(v) => Ok(Flow::Throw(v)),
                Err(flow) => Ok(flow),
            },
            Stmt::Return { value: None } => Ok(Flow::Return(JsVal::Undef)),
            Stmt::Return { value: Some(e) } => match self.eval_expr(e, env)? {
                Ok(v) => Ok(Flow::Return(v)),
                Err(flow) => Ok(flow),
            },
            Stmt::Try {
                block,
                handler_param,
                handler,
                finalizer,
            } => {
                let mut completion = match self.eval_body(block, env)? {
                    Flow::Throw(exc) => {
                        if let Some(handler) = handler {
                            if let Some(Pattern::Local(pid)) = handler_param {
                                env.insert(*pid, exc);
                            }
                            self.eval_body(handler, env)?
                        } else {
                            Flow::Throw(exc)
                        }
                    }
                    other => other,
                };
                if let Some(fin) = finalizer {
                    match self.eval_body(fin, env)? {
                        Flow::Normal => {}
                        abrupt => completion = abrupt,
                    }
                }
                Ok(completion)
            }
            Stmt::Block { body } => self.eval_body(body, env),
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                let t = match self.eval_expr(test, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(flow),
                };
                if to_boolean(&t) {
                    self.eval_stmt(consequent, env)
                } else if let Some(alt) = alternate {
                    self.eval_stmt(alt, env)
                } else {
                    Ok(Flow::Normal)
                }
            }
            _ => Err(()),
        }
    }

    /// `Ok(Ok(v))` = value; `Ok(Err(flow))` = abrupt throw; `Err(())` = unsupported.
    pub(crate) fn eval_expr(&self, 
        expr: &Expr,
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<Result<JsVal, Flow>, ()> {
        match expr {
            Expr::Number { raw, .. } => {
                let n: f64 = raw.parse().map_err(|_| ())?;
                Ok(Ok(JsVal::Num(n)))
            }
            Expr::Boolean { value, .. } => Ok(Ok(JsVal::Bool(*value))),
            Expr::String { value, .. } => Ok(Ok(JsVal::Str(js_string_to_utf8(value)))),
            Expr::Null { .. } => Ok(Ok(JsVal::Null)),
            Expr::RegExp { pattern, flags, .. } => {
                if !regexp_flags_ok(flags) {
                    return Err(());
                }
                parse_regexp_atoms(pattern)?;
                Ok(Ok(JsVal::RegExpInst {
                    source: pattern.clone(),
                    flags: flags.clone(),
                }))
            }
            Expr::Local { id, .. } => {
                let v = env.get(id).cloned().ok_or(())?;
                Ok(Ok(v))
            }
            Expr::IdentName { name, .. } => {
                let b = builtin_for_name(name).ok_or(())?;
                Ok(Ok(match b {
                    BuiltinId::Undefined => JsVal::Undef,
                    BuiltinId::Nan => JsVal::Num(f64::NAN),
                    BuiltinId::Infinity => JsVal::Num(f64::INFINITY),
                    other => JsVal::Builtin(other),
                }))
            }
            Expr::This { .. } => Ok(Ok(current_this())),
            Expr::NewTarget { .. } => Ok(Ok(current_new_target())),
            Expr::Function { .. } => Ok(Ok(user_fn_from_expr(self, expr).ok_or(())?)),
            Expr::Unary { op, arg, .. } => {
                match op {
                    UnaryOp::Delete => {
                        // `delete obj.prop` — fixture only deletes descriptor fields on plain objects.
                        match arg.as_ref() {
                            Expr::Member {
                                object,
                                property,
                                optional: false,
                                ..
                            } => {
                                let mut obj = match self.eval_expr(object, env)? {
                                    Ok(o) => o,
                                    Err(flow) => return Ok(Err(flow)),
                                };
                                let key = match self.eval_key(property, env)? {
                                    Ok(k) => k,
                                    Err(flow) => return Ok(Err(flow)),
                                };
                                match &mut obj {
                                    JsVal::Object { props, .. } => {
                                        props.borrow_mut().retain(|(k, _)| k != &key);
                                    }
                                    JsVal::UserFn { props, .. } => {
                                        props.borrow_mut().retain(|(k, _)| k != &key);
                                    }
                                    _ => return Err(()),
                                }
                                if let Expr::Local { id, .. } = object.as_ref() {
                                    env.insert(*id, obj);
                                } else {
                                    write_back_value(env, &obj);
                                }
                                Ok(Ok(JsVal::Bool(true)))
                            }
                            _ => Err(()),
                        }
                    }
                    _ => {
                        let v = match self.eval_expr(arg, env)? {
                            Ok(v) => v,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        match op {
                            UnaryOp::TypeOf => Ok(Ok(JsVal::Str(typeof_str(&v)))),
                            UnaryOp::Void => Ok(Ok(JsVal::Undef)),
                            UnaryOp::Not => Ok(Ok(JsVal::Bool(!to_boolean(&v)))),
                            UnaryOp::Minus => Ok(Ok(JsVal::Num(-to_number(&v)?))),
                            UnaryOp::Plus => Ok(Ok(JsVal::Num(to_number(&v)?))),
                            _ => Err(()),
                        }
                    }
                }
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                let l = match self.eval_expr(left, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                match op {
                    BinaryOp::And => {
                        if !to_boolean(&l) {
                            return Ok(Ok(l));
                        }
                        self.eval_expr(right, env)
                    }
                    BinaryOp::Or => {
                        if to_boolean(&l) {
                            return Ok(Ok(l));
                        }
                        self.eval_expr(right, env)
                    }
                    BinaryOp::EqEqEq | BinaryOp::EqEq => {
                        let r = match self.eval_expr(right, env)? {
                            Ok(v) => v,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        Ok(Ok(JsVal::Bool(strict_eq(&l, &r))))
                    }
                    BinaryOp::NotEqEq | BinaryOp::NotEq => {
                        let r = match self.eval_expr(right, env)? {
                            Ok(v) => v,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        Ok(Ok(JsVal::Bool(!strict_eq(&l, &r))))
                    }
                    BinaryOp::Comma => {
                        let _ = l;
                        self.eval_expr(right, env)
                    }
                    BinaryOp::Nullish => {
                        if matches!(l, JsVal::Null | JsVal::Undef) {
                            self.eval_expr(right, env)
                        } else {
                            Ok(Ok(l))
                        }
                    }
                    BinaryOp::Add => {
                        let r = match self.eval_expr(right, env)? {
                            Ok(v) => v,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        match (&l, &r) {
                            (JsVal::Num(a), JsVal::Num(b)) => Ok(Ok(JsVal::Num(a + b))),
                            (JsVal::Str(a), JsVal::Str(b)) => Ok(Ok(JsVal::Str(format!("{a}{b}")))),
                            (JsVal::Str(a), JsVal::Num(b)) => Ok(Ok(JsVal::Str(format!("{a}{b}")))),
                            (JsVal::Num(a), JsVal::Str(b)) => Ok(Ok(JsVal::Str(format!("{a}{b}")))),
                            _ => {
                                // ToNumber fallback for accessor fixtures (this.x + 1).
                                let a = to_number(&l)?;
                                let b = to_number(&r)?;
                                Ok(Ok(JsVal::Num(a + b)))
                            }
                        }
                    }
                    BinaryOp::Sub => {
                        let r = match self.eval_expr(right, env)? {
                            Ok(v) => v,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        Ok(Ok(JsVal::Num(to_number(&l)? - to_number(&r)?)))
                    }
                    BinaryOp::Mul => {
                        let r = match self.eval_expr(right, env)? {
                            Ok(v) => v,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        Ok(Ok(JsVal::Num(to_number(&l)? * to_number(&r)?)))
                    }
                    BinaryOp::Div => {
                        let r = match self.eval_expr(right, env)? {
                            Ok(v) => v,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        Ok(Ok(JsVal::Num(to_number(&l)? / to_number(&r)?)))
                    }
                    BinaryOp::Rem => {
                        let r = match self.eval_expr(right, env)? {
                            Ok(v) => v,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        Ok(Ok(JsVal::Num(to_number(&l)? % to_number(&r)?)))
                    }
                    _ => Err(()),
                }
            }
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => {
                let t = match self.eval_expr(test, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                if to_boolean(&t) {
                    self.eval_expr(consequent, env)
                } else {
                    self.eval_expr(alternate, env)
                }
            }
            Expr::Member {
                object,
                property,
                optional: false,
                ..
            } => {
                let obj = match self.eval_expr(object, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                let key = match self.eval_key(property, env)? {
                    Ok(k) => k,
                    Err(flow) => return Ok(Err(flow)),
                };
                Ok(Ok(self.member_get(&obj, &key, env)?))
            }
            Expr::New { callee, args, .. } => {
                let c = match self.eval_expr(callee, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                let mut arg_vals = Vec::new();
                for a in args {
                    match a {
                        Arg::Expr(e) => match self.eval_expr(e, env)? {
                            Ok(v) => arg_vals.push(v),
                            Err(flow) => return Ok(Err(flow)),
                        },
                        _ => return Err(()),
                    }
                }
                Ok(Ok(self.eval_new(&c, &arg_vals, env)?))
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
                        Arg::Expr(e) => match self.eval_expr(e, env)? {
                            Ok(v) => arg_vals.push(v),
                            Err(flow) => return Ok(Err(flow)),
                        },
                        _ => return Err(()),
                    }
                }
                // Method call: recv.prop(args) — keep `this` for Date/Map/Set instance methods.
                if let Expr::Member {
                    object,
                    property,
                    optional: false,
                    ..
                } = callee.as_ref()
                {
                    let mut obj = match self.eval_expr(object, env)? {
                        Ok(v) => v,
                        Err(flow) => return Ok(Err(flow)),
                    };
                    let key = match self.eval_key(property, env)? {
                        Ok(k) => k,
                        Err(flow) => return Ok(Err(flow)),
                    };
                    let result = self.eval_method_call(&mut obj, &key, &arg_vals, env)?;
                    // Write back mutated Map/Set (and any other instance) to local receiver.
                    if let Expr::Local { id, .. } = object.as_ref() {
                        env.insert(*id, obj);
                    }
                    return Ok(Ok(result));
                }
                let c = match self.eval_expr(callee, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                Ok(Ok(self.eval_call(&c, &arg_vals, env)?))
            }
            Expr::Assign {
                target: AssignTarget::Local(id),
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let v = match self.eval_expr(value, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                env.insert(*id, v.clone());
                Ok(Ok(v))
            }
            Expr::Assign {
                target:
                    AssignTarget::Member {
                        object, property, ..
                    },
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let v = match self.eval_expr(value, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                let mut obj = match self.eval_expr(object, env)? {
                    Ok(o) => o,
                    Err(flow) => return Ok(Err(flow)),
                };
                let key = match self.eval_key(property, env)? {
                    Ok(k) => k,
                    Err(flow) => return Ok(Err(flow)),
                };
                self.member_set(&mut obj, &key, v.clone(), env)?;
                if let Expr::Local { id, .. } = object.as_ref() {
                    env.insert(*id, obj);
                } else if matches!(object.as_ref(), Expr::This { .. }) {
                    CURRENT_THIS.with(|cell| {
                        *cell.borrow_mut() = obj;
                    });
                } else {
                    write_back_value(env, &obj);
                }
                Ok(Ok(v))
            }
            Expr::Array { elements, .. } => {
                let mut out = Vec::new();
                for el in elements {
                    match el {
                        ArrayElement::Expr(e) => match self.eval_expr(e, env)? {
                            Ok(v) => out.push(v),
                            Err(flow) => return Ok(Err(flow)),
                        },
                        ArrayElement::Elision => out.push(JsVal::Undef),
                        ArrayElement::Spread(_) => return Err(()),
                    }
                }
                Ok(Ok(JsVal::Array(out)))
            }
            Expr::Object { properties, .. } => {
                let mut props = Vec::new();
                let mut proto = JsVal::Builtin(BuiltinId::ObjectPrototype);
                for p in properties {
                    match p {
                        ObjectProp::Property {
                            key: ObjectPropKey::Static(k),
                            value,
                        } => {
                            let v = match self.eval_expr(value, env)? {
                                Ok(v) => v,
                                Err(flow) => return Ok(Err(flow)),
                            };
                            let key = js_string_to_utf8(k);
                            // Annex B / ES: static `__proto__` in object literal sets [[Prototype]].
                            if key == "__proto__" {
                                proto = v;
                                continue;
                            }
                            object_set_data(&mut props, key, v);
                        }
                        ObjectProp::Property {
                            key: ObjectPropKey::Computed(ke),
                            value,
                        } => {
                            let key = match self.eval_key(ke, env)? {
                                Ok(k) => k,
                                Err(flow) => return Ok(Err(flow)),
                            };
                            let v = match self.eval_expr(value, env)? {
                                Ok(v) => v,
                                Err(flow) => return Ok(Err(flow)),
                            };
                            // Computed `["__proto__"]` is an own data property (not [[Prototype]]).
                            object_set_data(&mut props, key, v);
                        }
                        ObjectProp::Accessor {
                            kind,
                            key: ObjectPropKey::Static(k),
                            value,
                        } => {
                            let fnv = match self.eval_expr(value, env)? {
                                Ok(v) => v,
                                Err(flow) => return Ok(Err(flow)),
                            };
                            let key = js_string_to_utf8(k);
                            match kind {
                                AccessorKind::Get => object_define_getter(&mut props, key, fnv),
                                AccessorKind::Set => object_define_setter(&mut props, key, fnv),
                            }
                        }
                        ObjectProp::Accessor {
                            kind,
                            key: ObjectPropKey::Computed(ke),
                            value,
                        } => {
                            let key = match self.eval_key(ke, env)? {
                                Ok(k) => k,
                                Err(flow) => return Ok(Err(flow)),
                            };
                            let fnv = match self.eval_expr(value, env)? {
                                Ok(v) => v,
                                Err(flow) => return Ok(Err(flow)),
                            };
                            match kind {
                                AccessorKind::Get => object_define_getter(&mut props, key, fnv),
                                AccessorKind::Set => object_define_setter(&mut props, key, fnv),
                            }
                        }
                        ObjectProp::Spread(_) => return Err(()),
                    }
                }
                Ok(Ok(new_object_with_proto(self, props, proto)))
            }
            _ => Err(()),
        }
    }

    pub(crate) fn eval_key(&self, 
        expr: &Expr,
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<Result<String, Flow>, ()> {
        match expr {
            Expr::String { value, .. } => Ok(Ok(js_string_to_utf8(value))),
            e => match self.eval_expr(e, env)? {
                Ok(JsVal::Str(s)) => Ok(Ok(s)),
                Ok(JsVal::Num(n)) => Ok(Ok(format!("{}", n as i64))),
                Ok(_) => Err(()),
                Err(flow) => Ok(Err(flow)),
            },
        }
    }
}
