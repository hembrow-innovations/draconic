use super::*;

impl super::World {
    pub(super) fn new(module: &Module) -> Self {
        let mut env = HashMap::new();
        let mut by_name = HashMap::new();
        for loc in &module.locals {
            by_name.insert(loc.name.clone(), loc.id);
            if loc.name == "undefined" {
                env.insert(loc.id, JsVal::Undef);
            } else if let Some(b) = builtin_for_name(&loc.name) {
                env.insert(loc.id, JsVal::Builtin(b));
            }
        }
        let objects = vec![
            ObjectRec {
                props: HashMap::new(),
                keys: Vec::new(),
                proto: JsVal::Null,
                extensible: true,
            },
            ObjectRec {
                props: HashMap::new(),
                keys: Vec::new(),
                proto: JsVal::Object(OBJECT_PROTOTYPE_IDX),
                extensible: true,
            },
        ];
        Self {
            env,
            name_env: HashMap::new(),
            fns: Vec::new(),
            objects,
            proxies: Vec::new(),
            weak_maps: Vec::new(),
            weak_sets: Vec::new(),
            by_name,
        }
    }

    pub(super) fn eval_body(&mut self, body: &[Stmt]) -> Result<Flow, ()> {
        for stmt in body {
            match self.eval_stmt(stmt)? {
                Flow::Normal => {}
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }

    pub(super) fn eval_stmt(&mut self, stmt: &Stmt) -> Result<Flow, ()> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let v = match init {
                    Some(e) => match self.eval_expr(e)? {
                        Ok(v) => v,
                        Err(flow) => return Ok(flow),
                    },
                    None => JsVal::Undef,
                };
                self.env.insert(*local, v);
                Ok(Flow::Normal)
            }
            Stmt::Expr { expr } => match self.eval_expr(expr)? {
                Ok(_) => Ok(Flow::Normal),
                Err(flow) => Ok(flow),
            },
            Stmt::Block { body } => self.eval_body(body),
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                let t = match self.eval_expr(test)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(flow),
                };
                if is_truthy(&t) {
                    self.eval_stmt(consequent)
                } else if let Some(a) = alternate {
                    self.eval_stmt(a)
                } else {
                    Ok(Flow::Normal)
                }
            }
            Stmt::Return { value: None } => Ok(Flow::Return(JsVal::Undef)),
            Stmt::Return { value: Some(e) } => match self.eval_expr(e)? {
                Ok(v) => Ok(Flow::Return(v)),
                Err(flow) => Ok(flow),
            },
            Stmt::Throw { value } => match self.eval_expr(value)? {
                Ok(v) => Ok(Flow::Throw(v)),
                Err(flow) => Ok(flow),
            },
            Stmt::Labeled { body, .. } => self.eval_stmt(body),
            Stmt::Function {
                local,
                params,
                body,
                is_async: false,
                is_generator: false,
            } => {
                let f = self.register_fn(params, body, false)?;
                self.env.insert(*local, f);
                Ok(Flow::Normal)
            }
            Stmt::Try {
                block,
                handler_param,
                handler,
                finalizer,
            } => {
                let mut flow = self.eval_body(block)?;
                if let Flow::Throw(exc) = flow {
                    if let Some(h) = handler {
                        if let Some(Pattern::Local(pid)) = handler_param {
                            self.env.insert(*pid, exc);
                        }
                        flow = self.eval_body(h)?;
                    } else {
                        flow = Flow::Throw(exc);
                    }
                }
                if let Some(f) = finalizer {
                    let fin = self.eval_body(f)?;
                    if !matches!(fin, Flow::Normal) {
                        return Ok(fin);
                    }
                }
                Ok(flow)
            }
            _ => Err(()),
        }
    }

    pub(super) fn register_fn(
        &mut self,
        params: &[Param],
        body: &[Stmt],
        is_arrow: bool,
    ) -> Result<JsVal, ()> {
        let mut precs = Vec::new();
        for p in params {
            if p.default.is_some() {
                return Err(());
            }
            let bind = match &p.pattern {
                Pattern::Local(id) => ParamBind::Local(*id),
                Pattern::Name(n) => ParamBind::Name(n.clone()),
                _ => return Err(()),
            };
            precs.push(ParamRec { bind, rest: p.rest });
        }
        let fn_idx = self.fns.len();
        self.fns.push(FnRec {
            params: precs,
            body: body.to_vec(),
            is_arrow,
        });
        let mut rec = ObjectRec {
            props: HashMap::new(),
            keys: Vec::new(),
            proto: JsVal::Object(FUNCTION_PROTOTYPE_IDX),
            extensible: true,
        };
        // Default .prototype for constructors.
        let proto_idx = self.objects.len() + 1; // after we push fn object
        let fn_obj_idx = self.objects.len();
        // placeholder proto object
        let mut proto_rec = empty_object();
        object_set_prop(
            &mut proto_rec,
            "constructor".into(),
            JsVal::Fn {
                fn_idx,
                obj_idx: fn_obj_idx,
            },
        );
        object_set_prop(&mut rec, "prototype".into(), JsVal::Object(proto_idx));
        self.objects.push(rec);
        self.objects.push(proto_rec);
        // fix constructor circular — already set with correct fn_obj_idx
        let _ = proto_idx;
        Ok(JsVal::Fn {
            fn_idx,
            obj_idx: fn_obj_idx,
        })
    }

    pub(super) fn eval_expr(&mut self, expr: &Expr) -> Result<Result<JsVal, Flow>, ()> {
        match expr {
            Expr::Number { raw, .. } => {
                let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
                let n: f64 = cleaned.parse().map_err(|_| ())?;
                Ok(Ok(JsVal::Num(n)))
            }
            Expr::Boolean { value, .. } => Ok(Ok(JsVal::Bool(*value))),
            Expr::String { value, .. } => Ok(Ok(JsVal::Str(js_string_to_utf8(value)))),
            Expr::Null { .. } => Ok(Ok(JsVal::Null)),
            Expr::Local { id, .. } => Ok(Ok(self.env.get(id).cloned().unwrap_or(JsVal::Undef))),
            Expr::IdentName { name, .. } => {
                if let Some(v) = self.name_env.get(name) {
                    return Ok(Ok(v.clone()));
                }
                if let Some(id) = self.by_name.get(name) {
                    if let Some(v) = self.env.get(id) {
                        return Ok(Ok(v.clone()));
                    }
                }
                if let Some(b) = builtin_for_name(name) {
                    return Ok(Ok(JsVal::Builtin(b)));
                }
                if name == "undefined" {
                    return Ok(Ok(JsVal::Undef));
                }
                Err(())
            }
            Expr::This { .. } => Ok(Ok(CURRENT_THIS.with(|c| c.borrow().clone()))),
            Expr::NewTarget { .. } => Ok(Ok(CURRENT_NEW_TARGET.with(|c| c.borrow().clone()))),
            Expr::Function {
                params,
                body,
                is_async: false,
                is_generator: false,
                is_arrow,
                ..
            } => Ok(Ok(self.register_fn(params, body, *is_arrow)?)),
            Expr::Unary { op, arg, .. } => {
                let v = match self.eval_expr(arg)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                match op {
                    UnaryOp::TypeOf => Ok(Ok(JsVal::Str(typeof_str(&v)))),
                    UnaryOp::Not => Ok(Ok(JsVal::Bool(!is_truthy(&v)))),
                    UnaryOp::Void => Ok(Ok(JsVal::Undef)),
                    UnaryOp::Delete => {
                        // delete obj.prop — arg should be Member
                        if let Expr::Member {
                            object,
                            property,
                            optional: false,
                            ..
                        } = arg.as_ref()
                        {
                            let obj = match self.eval_expr(object)? {
                                Ok(v) => v,
                                Err(f) => return Ok(Err(f)),
                            };
                            let key = match self.eval_key(property)? {
                                Ok(k) => k,
                                Err(f) => return Ok(Err(f)),
                            };
                            Ok(Ok(JsVal::Bool(self.object_delete(&obj, &key)?)))
                        } else {
                            Ok(Ok(JsVal::Bool(true)))
                        }
                    }
                    UnaryOp::Minus => match v {
                        JsVal::Num(n) => Ok(Ok(JsVal::Num(-n))),
                        _ => Err(()),
                    },
                    UnaryOp::Plus => Ok(Ok(JsVal::Num(to_number(&v)?))),
                    _ => Err(()),
                }
            }
            Expr::Binary {
                left, op, right, ..
            } => match op {
                BinaryOp::And => {
                    let l = match self.eval_expr(left)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    };
                    if !is_truthy(&l) {
                        return Ok(Ok(l));
                    }
                    self.eval_expr(right)
                }
                BinaryOp::Or => {
                    let l = match self.eval_expr(left)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    };
                    if is_truthy(&l) {
                        return Ok(Ok(l));
                    }
                    self.eval_expr(right)
                }
                BinaryOp::Comma => {
                    let _ = match self.eval_expr(left)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    };
                    self.eval_expr(right)
                }
                BinaryOp::EqEqEq | BinaryOp::EqEq => {
                    let l = match self.eval_expr(left)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    };
                    let r = match self.eval_expr(right)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    };
                    Ok(Ok(JsVal::Bool(strict_eq(&l, &r))))
                }
                BinaryOp::NotEqEq | BinaryOp::NotEq => {
                    let l = match self.eval_expr(left)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    };
                    let r = match self.eval_expr(right)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    };
                    Ok(Ok(JsVal::Bool(!strict_eq(&l, &r))))
                }
                _ => Err(()),
            },
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => {
                let t = match self.eval_expr(test)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                if is_truthy(&t) {
                    self.eval_expr(consequent)
                } else {
                    self.eval_expr(alternate)
                }
            }
            Expr::Assign {
                target: AssignTarget::Local(id),
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let v = match self.eval_expr(value)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                self.env.insert(*id, v.clone());
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
                let v = match self.eval_expr(value)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                let obj = match self.eval_expr(object)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                let key = match self.eval_key(property)? {
                    Ok(k) => k,
                    Err(f) => return Ok(Err(f)),
                };
                self.object_set(&obj, &key, v.clone())?;
                Ok(Ok(v))
            }
            Expr::Member {
                object,
                property,
                optional: false,
                ..
            } => {
                let obj = match self.eval_expr(object)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                let key = match self.eval_key(property)? {
                    Ok(k) => k,
                    Err(f) => return Ok(Err(f)),
                };
                Ok(Ok(self.object_get(&obj, &key)?))
            }
            Expr::Object { properties, .. } => {
                let mut rec = empty_object();
                let mut proto = JsVal::Object(OBJECT_PROTOTYPE_IDX);
                for p in properties {
                    match p {
                        ObjectProp::Property {
                            key: ObjectPropKey::Static(k),
                            value,
                        } => {
                            let key = js_string_to_utf8(k);
                            let v = match self.eval_expr(value)? {
                                Ok(v) => v,
                                Err(f) => return Ok(Err(f)),
                            };
                            if key == "__proto__" {
                                proto = v;
                            } else {
                                object_set_prop(&mut rec, key, v);
                            }
                        }
                        ObjectProp::Property {
                            key: ObjectPropKey::Computed(ke),
                            value,
                        } => {
                            let key = match self.eval_key(ke)? {
                                Ok(k) => k,
                                Err(f) => return Ok(Err(f)),
                            };
                            let v = match self.eval_expr(value)? {
                                Ok(v) => v,
                                Err(f) => return Ok(Err(f)),
                            };
                            object_set_prop(&mut rec, key, v);
                        }
                        _ => return Err(()),
                    }
                }
                rec.proto = proto;
                let idx = self.objects.len();
                self.objects.push(rec);
                Ok(Ok(JsVal::Object(idx)))
            }
            Expr::Array { elements, .. } => {
                let mut rec = empty_object();
                let mut len = 0usize;
                for el in elements {
                    match el {
                        ArrayElement::Expr(e) => {
                            let v = match self.eval_expr(e)? {
                                Ok(v) => v,
                                Err(f) => return Ok(Err(f)),
                            };
                            object_set_prop(&mut rec, len.to_string(), v);
                            len += 1;
                        }
                        ArrayElement::Elision => len += 1,
                        ArrayElement::Spread(_) => return Err(()),
                    }
                }
                object_set_prop(&mut rec, "length".into(), JsVal::Num(len as f64));
                let idx = self.objects.len();
                self.objects.push(rec);
                Ok(Ok(JsVal::Object(idx)))
            }
            Expr::New { callee, args, .. } => {
                let c = match self.eval_expr(callee)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                let mut argv = Vec::new();
                for a in args {
                    match a {
                        Arg::Expr(e) => match self.eval_expr(e)? {
                            Ok(v) => argv.push(v),
                            Err(f) => return Ok(Err(f)),
                        },
                        Arg::Spread(e) => {
                            let arr = match self.eval_expr(e)? {
                                Ok(v) => v,
                                Err(f) => return Ok(Err(f)),
                            };
                            argv.extend(self.array_to_vec(&arr)?);
                        }
                    }
                }
                match self.construct(&c, &argv, &c) {
                    Ok(v) => Ok(Ok(v)),
                    Err(()) => Err(()),
                }
            }
            Expr::Call {
                callee,
                args,
                optional: false,
                ..
            } => {
                let (func, this_arg) = match callee.as_ref() {
                    Expr::Member {
                        object,
                        property,
                        optional: false,
                        ..
                    } => {
                        let obj = match self.eval_expr(object)? {
                            Ok(v) => v,
                            Err(f) => return Ok(Err(f)),
                        };
                        let key = match self.eval_key(property)? {
                            Ok(k) => k,
                            Err(f) => return Ok(Err(f)),
                        };
                        let f = self.object_get(&obj, &key)?;
                        (f, obj)
                    }
                    _ => {
                        let f = match self.eval_expr(callee)? {
                            Ok(v) => v,
                            Err(f) => return Ok(Err(f)),
                        };
                        (f, JsVal::Undef)
                    }
                };
                let mut argv = Vec::new();
                for a in args {
                    match a {
                        Arg::Expr(e) => match self.eval_expr(e)? {
                            Ok(v) => argv.push(v),
                            Err(f) => return Ok(Err(f)),
                        },
                        Arg::Spread(e) => {
                            let arr = match self.eval_expr(e)? {
                                Ok(v) => v,
                                Err(f) => return Ok(Err(f)),
                            };
                            argv.extend(self.array_to_vec(&arr)?);
                        }
                    }
                }
                match self.call(&func, this_arg, &argv) {
                    Ok(v) => Ok(Ok(v)),
                    Err(()) => Err(()),
                }
            }
            _ => Err(()),
        }
    }

}
