use std::rc::Rc;

use super::*;

impl Ids {
    pub(super) fn eval_body(
        &self,
        body: &[Stmt],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<Flow, ()> {
        for s in body {
            match self.eval_stmt(s, env)? {
                Flow::Normal => {}
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }

    pub(super) fn eval_stmt(
        &self,
        stmt: &Stmt,
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<Flow, ()> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let v = match init {
                    Some(e) => self.ok_val(self.eval_expr(e, env)?)?,
                    None => JsVal::Undef,
                };
                env.insert(*local, v);
                Ok(Flow::Normal)
            }
            Stmt::Expr { expr } => {
                let _ = self.ok_val(self.eval_expr(expr, env)?)?;
                Ok(Flow::Normal)
            }
            Stmt::Throw { value } => Ok(Flow::Throw(self.ok_val(self.eval_expr(value, env)?)?)),
            Stmt::Return { value: None } => Ok(Flow::Return(JsVal::Undef)),
            Stmt::Return { value: Some(e) } => {
                Ok(Flow::Return(self.ok_val(self.eval_expr(e, env)?)?))
            }
            Stmt::Function {
                local,
                params,
                body,
                is_async,
                ..
            } => {
                env.insert(*local, self.new_fn(params.clone(), body.clone(), *is_async));
                Ok(Flow::Normal)
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                let t = self.ok_val(self.eval_expr(test, env)?)?;
                if self.to_bool(&t) {
                    self.eval_stmt(consequent, env)
                } else if let Some(a) = alternate {
                    self.eval_stmt(a, env)
                } else {
                    Ok(Flow::Normal)
                }
            }
            Stmt::Block { body } => self.eval_body(body, env),
            Stmt::Try {
                block,
                handler_param,
                handler,
                finalizer,
            } => {
                let completion = match self.eval_body(block, env)? {
                    Flow::Throw(exc) => {
                        if let Some(h) = handler {
                            if let Some(Pattern::Local(pid)) = handler_param {
                                env.insert(*pid, exc);
                            }
                            self.eval_body(h, env)?
                        } else {
                            Flow::Throw(exc)
                        }
                    }
                    other => other,
                };
                if let Some(fin) = finalizer {
                    match self.eval_body(fin, env)? {
                        Flow::Normal => Ok(completion),
                        abrupt => Ok(abrupt),
                    }
                } else {
                    Ok(completion)
                }
            }
            _ => Err(()),
        }
    }

    pub(super) fn ok_val(&self, r: Result<JsVal, Flow>) -> Result<JsVal, ()> {
        match r {
            Ok(v) => Ok(v),
            Err(_) => Err(()),
        }
    }

    pub(super) fn eval_expr(
        &self,
        expr: &Expr,
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<Result<JsVal, Flow>, ()> {
        match expr {
            Expr::Number { raw, .. } => Ok(Ok(JsVal::Num(raw.parse().map_err(|_| ())?))),
            Expr::Boolean { value, .. } => Ok(Ok(JsVal::Bool(*value))),
            Expr::String { value, .. } => Ok(Ok(JsVal::Str(value.to_string_lossy()))),
            Expr::Null { .. } => Ok(Ok(JsVal::Null)),
            Expr::Local { id, .. } => Ok(Ok(env.get(id).cloned().ok_or(())?)),
            Expr::This { .. } => Ok(Ok(current_this())),
            Expr::NewTarget { .. } => Ok(Ok(current_new_target())),
            Expr::IdentName { name, .. } => {
                Ok(Ok(lookup_name(name).or_else(|| builtin(name)).ok_or(())?))
            }
            Expr::Super { .. } => Ok(Ok(current_this())),
            Expr::Function {
                params,
                body,
                is_async,
                ..
            } => Ok(Ok(self.new_fn(params.clone(), body.clone(), *is_async))),
            Expr::Unary { op, arg, .. } => self.eval_unary(*op, arg, env),
            Expr::Binary {
                left, op, right, ..
            } => self.eval_binary(left, *op, right, env),
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => {
                let t = match self.eval_expr(test, env)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                if self.to_bool(&t) {
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
                    Err(f) => return Ok(Err(f)),
                };
                let key = match self.eval_key(property, env)? {
                    Ok(k) => k,
                    Err(f) => return Ok(Err(f)),
                };
                Ok(Ok(self.member_get(&obj, &key, env)?))
            }
            Expr::New { callee, args, .. } => {
                let c = match self.eval_expr(callee, env)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                let av = self.eval_args(args, env)?;
                Ok(Ok(self.eval_new(&c, &av, env)?))
            }
            Expr::Call {
                callee,
                args,
                optional: false,
                ..
            } => {
                let av = match self.eval_args_flow(args, env)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                if let Expr::Member {
                    object,
                    property,
                    optional: false,
                    ..
                } = callee.as_ref()
                {
                    let obj = match self.eval_expr(object, env)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    };
                    let key = match self.eval_key(property, env)? {
                        Ok(k) => k,
                        Err(f) => return Ok(Err(f)),
                    };
                    let mut obj = obj;
                    let result = self.method_call(&mut obj, &key, &av, env)?;
                    if let Expr::Local { id, .. } = object.as_ref() {
                        env.insert(*id, obj);
                    }
                    return Ok(Ok(result));
                }
                let c = match self.eval_expr(callee, env)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                Ok(Ok(self.call_val(&c, &av, JsVal::Undef, env)?))
            }
            Expr::Assign {
                target: AssignTarget::Local(id),
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let v = match self.eval_expr(value, env)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
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
                    Err(f) => return Ok(Err(f)),
                };
                let mut obj = match self.eval_expr(object, env)? {
                    Ok(o) => o,
                    Err(f) => return Ok(Err(f)),
                };
                let key = match self.eval_key(property, env)? {
                    Ok(k) => k,
                    Err(f) => return Ok(Err(f)),
                };
                self.member_set(&mut obj, &key, v.clone(), env)?;
                if let Expr::Local { id, .. } = object.as_ref() {
                    env.insert(*id, obj);
                }
                Ok(Ok(v))
            }
            Expr::Array { elements, .. } => {
                // unused in fixture observations
                let mut out = Vec::new();
                for el in elements {
                    match el {
                        ArrayElement::Expr(e) => out.push(match self.eval_expr(e, env)? {
                            Ok(v) => v,
                            Err(f) => return Ok(Err(f)),
                        }),
                        ArrayElement::Elision => out.push(JsVal::Undef),
                        ArrayElement::Spread(_) => return Err(()),
                    }
                }
                Ok(Ok(JsVal::Object {
                    id: self.next_id(),
                    props: Rc::new(RefCell::new(Vec::new())),
                    proto: Rc::new(RefCell::new(JsVal::Builtin("Array.prototype"))),
                }))
            }
            Expr::Object { properties, .. } => {
                let mut props = Vec::new();
                let mut proto = JsVal::Builtin("Object.prototype");
                for p in properties {
                    match p {
                        ObjectProp::Property {
                            key: ObjectPropKey::Static(k),
                            value,
                        } => {
                            let v = match self.eval_expr(value, env)? {
                                Ok(v) => v,
                                Err(f) => return Ok(Err(f)),
                            };
                            let key = k.to_string_lossy();
                            if key == "__proto__" {
                                proto = v;
                                continue;
                            }
                            props.push((key, Slot::Data(v)));
                        }
                        ObjectProp::Property {
                            key: ObjectPropKey::Computed(ke),
                            value,
                        } => {
                            let key = match self.eval_key(ke, env)? {
                                Ok(k) => k,
                                Err(f) => return Ok(Err(f)),
                            };
                            let v = match self.eval_expr(value, env)? {
                                Ok(v) => v,
                                Err(f) => return Ok(Err(f)),
                            };
                            props.push((key, Slot::Data(v)));
                        }
                        ObjectProp::Accessor {
                            key, value, kind, ..
                        } => {
                            let key = match key {
                                ObjectPropKey::Static(k) => k.to_string_lossy(),
                                ObjectPropKey::Computed(ke) => match self.eval_key(ke, env)? {
                                    Ok(k) => k,
                                    Err(f) => return Ok(Err(f)),
                                },
                            };
                            let v = match self.eval_expr(value, env)? {
                                Ok(v) => v,
                                Err(f) => return Ok(Err(f)),
                            };
                            let existing = props.iter().position(|(k, _)| k == &key);
                            let (mut get, mut set) =
                                match existing.and_then(|i| match &props[i].1 {
                                    Slot::Accessor { get, set } => Some((get.clone(), set.clone())),
                                    _ => None,
                                }) {
                                    Some(pair) => pair,
                                    None => (None, None),
                                };
                            match kind {
                                draconic_ast::AccessorKind::Get => get = Some(v),
                                draconic_ast::AccessorKind::Set => set = Some(v),
                            }
                            let slot = Slot::Accessor { get, set };
                            if let Some(i) = existing {
                                props[i].1 = slot;
                            } else {
                                props.push((key, slot));
                            }
                        }
                        _ => return Err(()),
                    }
                }
                Ok(Ok(JsVal::Object {
                    id: self.next_id(),
                    props: Rc::new(RefCell::new(props)),
                    proto: Rc::new(RefCell::new(proto)),
                }))
            }
            _ => Err(()),
        }
    }

    pub(super) fn eval_args(
        &self,
        args: &[Arg],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<Vec<JsVal>, ()> {
        match self.eval_args_flow(args, env)? {
            Ok(v) => Ok(v),
            Err(_) => Err(()),
        }
    }

    pub(super) fn eval_args_flow(
        &self,
        args: &[Arg],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<Result<Vec<JsVal>, Flow>, ()> {
        let mut out = Vec::new();
        for a in args {
            match a {
                Arg::Expr(e) => match self.eval_expr(e, env)? {
                    Ok(v) => out.push(v),
                    Err(f) => return Ok(Err(f)),
                },
                _ => return Err(()),
            }
        }
        Ok(Ok(out))
    }

    pub(super) fn eval_key(
        &self,
        expr: &Expr,
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<Result<String, Flow>, ()> {
        match expr {
            Expr::String { value, .. } => Ok(Ok(value.to_string_lossy())),
            e => match self.eval_expr(e, env)? {
                Ok(JsVal::Str(s)) => Ok(Ok(s)),
                Ok(JsVal::Num(n)) => Ok(Ok(format!("{}", n as i64))),
                Ok(_) => Err(()),
                Err(f) => Ok(Err(f)),
            },
        }
    }

    pub(super) fn eval_unary(
        &self,
        op: UnaryOp,
        arg: &Expr,
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<Result<JsVal, Flow>, ()> {
        match op {
            UnaryOp::Delete => {
                if let Expr::Member {
                    object,
                    property,
                    optional: false,
                    ..
                } = arg
                {
                    let obj = match self.eval_expr(object, env)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    };
                    let key = match self.eval_key(property, env)? {
                        Ok(k) => k,
                        Err(f) => return Ok(Err(f)),
                    };
                    match &obj {
                        JsVal::Object { props, .. } | JsVal::UserFn { props, .. } => {
                            delete_key(props, &key);
                        }
                        _ => {}
                    }
                    if let Expr::Local { id, .. } = object.as_ref() {
                        env.insert(*id, obj);
                    }
                    return Ok(Ok(JsVal::Bool(true)));
                }
                let _ = match self.eval_expr(arg, env)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                Ok(Ok(JsVal::Bool(true)))
            }
            UnaryOp::Void | UnaryOp::Await | UnaryOp::Yield | UnaryOp::YieldStar => {
                let v = match self.eval_expr(arg, env)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                if matches!(op, UnaryOp::Void) {
                    Ok(Ok(JsVal::Undef))
                } else if matches!(op, UnaryOp::Await) {
                    match v {
                        JsVal::Promise { result, .. } => match result.as_ref() {
                            Ok(inner) => Ok(Ok(inner.clone())),
                            Err(exc) => Ok(Err(Flow::Throw(exc.clone()))),
                        },
                        other => Ok(Ok(other)),
                    }
                } else {
                    Ok(Ok(v))
                }
            }
            _ => {
                let v = match self.eval_expr(arg, env)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                match op {
                    UnaryOp::TypeOf => Ok(Ok(JsVal::Str(self.typeof_str(&v)))),
                    UnaryOp::Not => Ok(Ok(JsVal::Bool(!self.to_bool(&v)))),
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
        }
    }

    pub(super) fn eval_binary(
        &self,
        left: &Expr,
        op: BinaryOp,
        right: &Expr,
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<Result<JsVal, Flow>, ()> {
        let l = match self.eval_expr(left, env)? {
            Ok(v) => v,
            Err(f) => return Ok(Err(f)),
        };
        match op {
            BinaryOp::And => {
                if !self.to_bool(&l) {
                    return Ok(Ok(l));
                }
                self.eval_expr(right, env)
            }
            BinaryOp::Or => {
                if self.to_bool(&l) {
                    return Ok(Ok(l));
                }
                self.eval_expr(right, env)
            }
            BinaryOp::Comma => {
                let r = match self.eval_expr(right, env)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                Ok(Ok(r))
            }
            BinaryOp::EqEqEq | BinaryOp::EqEq => {
                let r = match self.eval_expr(right, env)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                Ok(Ok(JsVal::Bool(self.strict_eq(&l, &r))))
            }
            BinaryOp::NotEqEq | BinaryOp::NotEq => {
                let r = match self.eval_expr(right, env)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                Ok(Ok(JsVal::Bool(!self.strict_eq(&l, &r))))
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
                let r = match self.eval_expr(right, env)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                if matches!(op, BinaryOp::Add)
                    && (matches!(l, JsVal::Str(_)) || matches!(r, JsVal::Str(_)))
                {
                    return Ok(Ok(JsVal::Str(format!(
                        "{}{}",
                        self.to_str(&l),
                        self.to_str(&r)
                    ))));
                }
                let ln = self.to_num(&l)?;
                let rn = self.to_num(&r)?;
                let n = match op {
                    BinaryOp::Add => ln + rn,
                    BinaryOp::Sub => ln - rn,
                    BinaryOp::Mul => ln * rn,
                    BinaryOp::Div => ln / rn,
                    BinaryOp::Rem => ln % rn,
                    _ => unreachable!(),
                };
                Ok(Ok(JsVal::Num(n)))
            }
            _ => Err(()),
        }
    }

    pub(super) fn to_bool(&self, v: &JsVal) -> bool {
        match v {
            JsVal::Bool(b) => *b,
            JsVal::Undef | JsVal::Null => false,
            JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
            JsVal::Str(s) => !s.is_empty(),
            _ => true,
        }
    }

    pub(super) fn to_num(&self, v: &JsVal) -> Result<f64, ()> {
        match v {
            JsVal::Num(n) => Ok(*n),
            JsVal::Bool(true) => Ok(1.0),
            JsVal::Bool(false) => Ok(0.0),
            _ => Err(()),
        }
    }

    pub(super) fn to_str(&self, v: &JsVal) -> String {
        match v {
            JsVal::Str(s) => s.clone(),
            JsVal::Num(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    n.to_string()
                }
            }
            JsVal::Bool(true) => "true".into(),
            JsVal::Bool(false) => "false".into(),
            JsVal::Undef => "undefined".into(),
            JsVal::Null => "null".into(),
            _ => "[object Object]".into(),
        }
    }

    pub(super) fn typeof_str(&self, v: &JsVal) -> String {
        match v {
            JsVal::Num(_) => "number".into(),
            JsVal::Bool(_) => "boolean".into(),
            JsVal::Str(_) => "string".into(),
            JsVal::Undef => "undefined".into(),
            JsVal::UserFn { .. }
            | JsVal::Builtin(
                "Object" | "Function" | "WeakMap" | "WeakSet" | "TypeError" | "Error"
                | "ReferenceError" | "Proxy" | "Promise",
            ) => "function".into(),
            JsVal::Builtin("Reflect") => "object".into(),
            JsVal::Promise { .. } => "object".into(),
            JsVal::Builtin("Object.prototype" | "Function.prototype" | "Array.prototype") => {
                "object".into()
            }
            _ => "object".into(),
        }
    }

    pub(super) fn strict_eq(&self, a: &JsVal, b: &JsVal) -> bool {
        match (a, b) {
            (JsVal::Num(x), JsVal::Num(y)) => x == y,
            (JsVal::Bool(x), JsVal::Bool(y)) => x == y,
            (JsVal::Str(x), JsVal::Str(y)) => x == y,
            (JsVal::Undef, JsVal::Undef) => true,
            (JsVal::Null, JsVal::Null) => true,
            (JsVal::Builtin(x), JsVal::Builtin(y)) => x == y,
            (JsVal::Object { id: x, .. }, JsVal::Object { id: y, .. }) => x == y,
            (JsVal::UserFn { id: x, .. }, JsVal::UserFn { id: y, .. }) => x == y,
            (JsVal::WeakMap(x), JsVal::WeakMap(y)) => Rc::ptr_eq(x, y),
            (JsVal::WeakSet(x), JsVal::WeakSet(y)) => Rc::ptr_eq(x, y),
            _ => false,
        }
    }

    pub(super) fn member_get(
        &self,
        obj: &JsVal,
        key: &str,
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        match obj {
            JsVal::Object { props, proto, .. } => {
                if let Some((_, slot)) = props.borrow().iter().find(|(k, _)| k == key) {
                    return match slot {
                        Slot::Data(v) => Ok(v.clone()),
                        Slot::Accessor { get: Some(g), .. } => {
                            self.call_val(g, &[], obj.clone(), env)
                        }
                        Slot::Accessor { get: None, .. } => Ok(JsVal::Undef),
                    };
                }
                if key == "__proto__" {
                    return Ok(proto.borrow().clone());
                }
                let p = proto.borrow().clone();
                if matches!(p, JsVal::Null) {
                    return Ok(JsVal::Undef);
                }
                self.member_get(&p, key, env)
            }
            JsVal::UserFn { props, .. } => {
                if let Some(v) = get_data(&props.borrow(), key) {
                    return Ok(v);
                }
                Ok(JsVal::Undef)
            }
            JsVal::Proxy {
                target, handler, ..
            } => self.proxy_get(target, handler, obj, key, env),
            JsVal::Builtin("Object") => match key {
                "prototype" => Ok(JsVal::Builtin("Object.prototype")),
                "defineProperty" => Ok(JsVal::Builtin("Object.defineProperty")),
                "getOwnPropertyDescriptor" => Ok(JsVal::Builtin("Object.getOwnPropertyDescriptor")),
                "isExtensible" => Ok(JsVal::Builtin("Object.isExtensible")),
                "getPrototypeOf" => Ok(JsVal::Builtin("Object.getPrototypeOf")),
                "setPrototypeOf" => Ok(JsVal::Builtin("Object.setPrototypeOf")),
                _ => Ok(JsVal::Undef),
            },
            JsVal::Builtin("Reflect") => match key {
                "construct" => Ok(JsVal::Builtin("Reflect.construct")),
                "get" => Ok(JsVal::Builtin("Reflect.get")),
                _ => Ok(JsVal::Undef),
            },
            JsVal::Builtin("Promise") => match key {
                "resolve" => Ok(JsVal::Builtin("Promise.resolve")),
                _ => Ok(JsVal::Undef),
            },
            JsVal::Builtin("Function") => match key {
                "prototype" => Ok(JsVal::Builtin("Function.prototype")),
                _ => Ok(JsVal::Undef),
            },
            JsVal::Builtin("Object.prototype") | JsVal::Builtin("Function.prototype") => {
                Ok(JsVal::Undef)
            }
            _ => Ok(JsVal::Undef),
        }
    }

    pub(super) fn member_set(
        &self,
        obj: &mut JsVal,
        key: &str,
        val: JsVal,
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<(), ()> {
        match obj {
            JsVal::Object { props, proto, .. } => {
                if key == "__proto__" {
                    *proto.borrow_mut() = val;
                    return Ok(());
                }
                let existing = props
                    .borrow()
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, s)| s.clone());
                match existing {
                    Some(Slot::Accessor { set: Some(s), .. }) => {
                        self.call_val(&s, &[val], obj.clone(), env)?;
                        Ok(())
                    }
                    Some(Slot::Accessor { set: None, .. }) => Ok(()),
                    _ => {
                        set_data(props, key.to_string(), val);
                        Ok(())
                    }
                }
            }
            JsVal::UserFn { props, .. } => {
                set_data(props, key.to_string(), val);
                Ok(())
            }
            _ => Err(()),
        }
    }
}
