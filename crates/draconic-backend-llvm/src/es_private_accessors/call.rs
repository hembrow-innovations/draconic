use super::*;

impl Ids {
    pub(super) fn eval_new(
        &self,
        callee: &JsVal,
        args: &[JsVal],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        match callee {
            JsVal::Builtin("WeakMap") => Ok(JsVal::WeakMap(Rc::new(RefCell::new(Vec::new())))),
            JsVal::Builtin("WeakSet") => Ok(JsVal::WeakSet(Rc::new(RefCell::new(Vec::new())))),
            JsVal::Builtin("TypeError")
            | JsVal::Builtin("Error")
            | JsVal::Builtin("ReferenceError") => {
                let msg = match args.first() {
                    Some(JsVal::Str(s)) => s.clone(),
                    _ => String::new(),
                };
                Ok(JsVal::Err { message: msg })
            }
            JsVal::Builtin("Proxy") => {
                let target = args.first().cloned().ok_or(())?;
                let handler = args.get(1).cloned().ok_or(())?;
                Ok(JsVal::Proxy {
                    id: self.next_id(),
                    target: Rc::new(RefCell::new(target)),
                    handler: Rc::new(RefCell::new(handler)),
                })
            }
            JsVal::UserFn {
                params,
                body,
                is_async,
                ..
            } => {
                let new_target = match current_new_target() {
                    JsVal::Undef => callee.clone(),
                    nt => nt,
                };
                let proto = self
                    .member_get(&new_target, "prototype", env)
                    .unwrap_or(JsVal::Builtin("Object.prototype"));
                let this_obj = match proto {
                    JsVal::Object { .. }
                    | JsVal::Null
                    | JsVal::UserFn { .. }
                    | JsVal::Proxy { .. }
                    | JsVal::Builtin(_) => self.new_obj(proto),
                    _ => self.new_obj(JsVal::Builtin("Object.prototype")),
                };
                let params = params.clone();
                let body = body.clone();
                let is_async = *is_async;
                let result = with_new_target(new_target, || {
                    self.call_user(&params, &body, this_obj.clone(), args, env, is_async)
                })?;
                if is_objectish(&result) && !matches!(result, JsVal::Undef) {
                    Ok(result)
                } else {
                    Ok(this_obj)
                }
            }
            _ => Err(()),
        }
    }

    pub(super) fn method_call(
        &self,
        recv: &mut JsVal,
        key: &str,
        args: &[JsVal],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        // WeakMap / WeakSet methods
        match recv {
            JsVal::WeakMap(entries) => match key {
                "set" => {
                    let k = args.first().ok_or(())?;
                    let id = obj_id(k).ok_or(())?;
                    let v = args.get(1).cloned().unwrap_or(JsVal::Undef);
                    {
                        let mut e = entries.borrow_mut();
                        if let Some((_, slot)) = e.iter_mut().find(|(i, _)| *i == id) {
                            *slot = v;
                        } else {
                            e.push((id, v));
                        }
                    }
                    return Ok(recv.clone());
                }
                "get" => {
                    let k = args.first().ok_or(())?;
                    let id = match obj_id(k) {
                        Some(i) => i,
                        None => return Ok(JsVal::Undef),
                    };
                    return Ok(entries
                        .borrow()
                        .iter()
                        .find(|(i, _)| *i == id)
                        .map(|(_, v)| v.clone())
                        .unwrap_or(JsVal::Undef));
                }
                "has" => {
                    let k = args.first().ok_or(())?;
                    let id = match obj_id(k) {
                        Some(i) => i,
                        None => return Ok(JsVal::Bool(false)),
                    };
                    return Ok(JsVal::Bool(entries.borrow().iter().any(|(i, _)| *i == id)));
                }
                _ => {}
            },
            JsVal::WeakSet(values) => match key {
                "add" => {
                    let v = args.first().ok_or(())?;
                    let id = obj_id(v).ok_or(())?;
                    {
                        let mut e = values.borrow_mut();
                        if !e.contains(&id) {
                            e.push(id);
                        }
                    }
                    return Ok(recv.clone());
                }
                "has" => {
                    let v = args.first().ok_or(())?;
                    let id = match obj_id(v) {
                        Some(i) => i,
                        None => return Ok(JsVal::Bool(false)),
                    };
                    return Ok(JsVal::Bool(values.borrow().contains(&id)));
                }
                _ => {}
            },
            JsVal::UserFn {
                params,
                body,
                is_async,
                ..
            } if key == "call" => {
                let this_arg = args.first().cloned().unwrap_or(JsVal::Undef);
                let rest: Vec<_> = args.iter().skip(1).cloned().collect();
                return self.call_user(
                    &params.clone(),
                    &body.clone(),
                    this_arg,
                    &rest,
                    env,
                    *is_async,
                );
            }
            JsVal::Promise { result, .. } if key == "then" => {
                return self.promise_then(result.as_ref(), args, env);
            }
            JsVal::Builtin("Promise") if key == "resolve" => {
                return Ok(self.promise_resolve(args));
            }
            JsVal::Builtin("Object") if key == "setPrototypeOf" => {
                return self.object_set_prototype_of(args);
            }
            JsVal::Builtin("Reflect") => match key {
                "construct" => return self.reflect_construct(args, env),
                "get" => return self.reflect_get(args, env),
                _ => {}
            },
            JsVal::Builtin("Object") => match key {
                "isExtensible" => return Ok(JsVal::Bool(true)),
                "getPrototypeOf" => {
                    let t = args.first().ok_or(())?;
                    return match t {
                        JsVal::Object { proto, .. } => Ok(proto.borrow().clone()),
                        JsVal::UserFn { props, .. } => {
                            Ok(get_data(&props.borrow(), "prototype").unwrap_or(JsVal::Undef))
                        }
                        _ => Ok(JsVal::Null),
                    };
                }
                "getOwnPropertyDescriptor" => {
                    let t = args.first().ok_or(())?;
                    let k = match args.get(1) {
                        Some(JsVal::Str(s)) => s.as_str(),
                        _ => return Err(()),
                    };
                    return self.get_own_desc(t, k);
                }
                "defineProperty" => {
                    let mut t = args.first().cloned().ok_or(())?;
                    let k = match args.get(1) {
                        Some(JsVal::Str(s)) => s.clone(),
                        _ => return Err(()),
                    };
                    let desc = args.get(2).ok_or(())?;
                    self.define_prop(&mut t, &k, desc, env)?;
                    // writeback by id
                    if let Some(id) = obj_id(&t) {
                        for v in env.values_mut() {
                            if obj_id(v) == Some(id) {
                                *v = t.clone();
                            }
                        }
                    }
                    return Ok(t);
                }
                _ => {}
            },
            _ => {}
        }
        let c = self.member_get(recv, key, env)?;
        match c {
            JsVal::UserFn {
                params,
                body,
                is_async,
                ..
            } => self.call_user(&params, &body, recv.clone(), args, env, is_async),
            JsVal::Builtin(name) => self.call_builtin(name, args, env),
            JsVal::Undef => Err(()),
            other => self.call_val(&other, args, JsVal::Undef, env),
        }
    }

    pub(super) fn get_own_desc(&self, target: &JsVal, key: &str) -> Result<JsVal, ()> {
        let slot = match target {
            JsVal::Object { props, .. } | JsVal::UserFn { props, .. } => props
                .borrow()
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, s)| s.clone()),
            _ => None,
        };
        match slot {
            Some(Slot::Data(v)) => Ok(JsVal::Object {
                id: self.next_id(),
                props: Rc::new(RefCell::new(vec![
                    ("value".into(), Slot::Data(v)),
                    ("writable".into(), Slot::Data(JsVal::Bool(true))),
                    ("enumerable".into(), Slot::Data(JsVal::Bool(true))),
                    ("configurable".into(), Slot::Data(JsVal::Bool(true))),
                ])),
                proto: Rc::new(RefCell::new(JsVal::Builtin("Object.prototype"))),
            }),
            Some(Slot::Accessor { get, set }) => {
                let mut props = vec![
                    ("enumerable".into(), Slot::Data(JsVal::Bool(true))),
                    ("configurable".into(), Slot::Data(JsVal::Bool(true))),
                ];
                if let Some(g) = get {
                    props.push(("get".into(), Slot::Data(g)));
                }
                if let Some(s) = set {
                    props.push(("set".into(), Slot::Data(s)));
                }
                Ok(JsVal::Object {
                    id: self.next_id(),
                    props: Rc::new(RefCell::new(props)),
                    proto: Rc::new(RefCell::new(JsVal::Builtin("Object.prototype"))),
                })
            }
            None => Ok(JsVal::Undef),
        }
    }

    pub(super) fn define_prop(
        &self,
        target: &mut JsVal,
        key: &str,
        desc: &JsVal,
        _env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<(), ()> {
        let JsVal::Object { props: dprops, .. } = desc else {
            return Err(());
        };
        let d = dprops.borrow();
        let value = get_data(&d, "value");
        let get = get_data(&d, "get");
        let set = get_data(&d, "set");
        let props = match target {
            JsVal::Object { props, .. } | JsVal::UserFn { props, .. } => props.clone(),
            _ => return Err(()),
        };
        if get.is_some() || set.is_some() {
            let slot = Slot::Accessor {
                get: get.filter(|g| !matches!(g, JsVal::Undef)),
                set: set.filter(|s| !matches!(s, JsVal::Undef)),
            };
            let mut p = props.borrow_mut();
            if let Some((_, s)) = p.iter_mut().find(|(k, _)| k == key) {
                *s = slot;
            } else {
                p.push((key.to_string(), slot));
            }
        } else if let Some(v) = value {
            set_data(&props, key.to_string(), v);
        }
        Ok(())
    }

    pub(super) fn call_val(
        &self,
        callee: &JsVal,
        args: &[JsVal],
        this: JsVal,
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        match callee {
            JsVal::UserFn {
                params,
                body,
                is_async,
                ..
            } => self.call_user(params, body, this, args, env, *is_async),
            JsVal::Builtin(name) => self.call_builtin(name, args, env),
            _ => Err(()),
        }
    }

    pub(super) fn call_builtin(
        &self,
        name: &str,
        args: &[JsVal],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        match name {
            "Object.isExtensible" => Ok(JsVal::Bool(true)),
            "Object.getPrototypeOf" => {
                let t = args.first().ok_or(())?;
                match t {
                    JsVal::Object { proto, .. } => Ok(proto.borrow().clone()),
                    JsVal::UserFn { props, .. } => {
                        Ok(get_data(&props.borrow(), "prototype").unwrap_or(JsVal::Undef))
                    }
                    _ => Ok(JsVal::Null),
                }
            }
            "Object.getOwnPropertyDescriptor" => {
                let t = args.first().ok_or(())?;
                let k = match args.get(1) {
                    Some(JsVal::Str(s)) => s.as_str(),
                    _ => return Err(()),
                };
                self.get_own_desc(t, k)
            }
            "Object.defineProperty" => {
                let mut t = args.first().cloned().ok_or(())?;
                let k = match args.get(1) {
                    Some(JsVal::Str(s)) => s.clone(),
                    _ => return Err(()),
                };
                let desc = args.get(2).ok_or(())?;
                self.define_prop(&mut t, &k, desc, env)?;
                if let Some(id) = obj_id(&t) {
                    for v in env.values_mut() {
                        if obj_id(v) == Some(id) {
                            *v = t.clone();
                        }
                    }
                }
                Ok(t)
            }
            "Object.setPrototypeOf" => self.object_set_prototype_of(args),
            "Reflect.construct" => self.reflect_construct(args, env),
            "Reflect.get" => self.reflect_get(args, env),
            "Promise.resolve" => Ok(self.promise_resolve(args)),
            "TypeError" | "Error" | "ReferenceError" => {
                let msg = match args.first() {
                    Some(JsVal::Str(s)) => s.clone(),
                    _ => String::new(),
                };
                Ok(JsVal::Err { message: msg })
            }
            _ => Err(()),
        }
    }

    pub(super) fn call_user(
        &self,
        params: &[Param],
        body: &[Stmt],
        this: JsVal,
        args: &[JsVal],
        env: &mut HashMap<LocalId, JsVal>,
        is_async: bool,
    ) -> Result<JsVal, ()> {
        let mut saved = Vec::new();
        let mut names = HashMap::new();
        for (i, p) in params.iter().enumerate() {
            let v = args.get(i).cloned().unwrap_or(JsVal::Undef);
            match &p.pattern {
                Pattern::Local(id) => {
                    saved.push((*id, env.get(id).cloned()));
                    env.insert(*id, v);
                }
                Pattern::Name(n) => {
                    names.insert(n.clone(), v);
                }
                _ => return Err(()),
            }
        }
        names
            .entry("arguments".into())
            .or_insert_with(|| self.new_obj(JsVal::Builtin("Array.prototype")));
        let flow = with_this(this, || {
            with_name_frame(names, || self.eval_body(body, env))
        })?;
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
            Flow::Normal => Ok(if is_async {
                self.fulfilled(JsVal::Undef)
            } else {
                JsVal::Undef
            }),
            Flow::Return(v) => Ok(if is_async { self.fulfilled(v) } else { v }),
            Flow::Throw(e) => {
                if is_async {
                    Ok(self.rejected(e))
                } else {
                    Err(())
                }
            }
        }
    }

    pub(super) fn promise_resolve(&self, args: &[JsVal]) -> JsVal {
        match args.first() {
            Some(p @ JsVal::Promise { .. }) => p.clone(),
            Some(v) => self.fulfilled(v.clone()),
            None => self.fulfilled(JsVal::Undef),
        }
    }

    pub(super) fn promise_then(
        &self,
        result: &Result<JsVal, JsVal>,
        args: &[JsVal],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        match result {
            Ok(v) => match args.first() {
                Some(f) if !matches!(f, JsVal::Undef | JsVal::Null) => {
                    self.call_val(f, &[v.clone()], JsVal::Undef, env)
                }
                _ => Ok(self.fulfilled(v.clone())),
            },
            Err(e) => match args.get(1) {
                Some(r) if !matches!(r, JsVal::Undef | JsVal::Null) => {
                    self.call_val(r, &[e.clone()], JsVal::Undef, env)
                }
                _ => Ok(self.rejected(e.clone())),
            },
        }
    }

    pub(super) fn proxy_get(
        &self,
        target: &Rc<RefCell<JsVal>>,
        handler: &Rc<RefCell<JsVal>>,
        receiver: &JsVal,
        key: &str,
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        let trap = match handler.borrow().clone() {
            JsVal::Object { props, .. } => get_data(&props.borrow(), "get"),
            _ => None,
        };
        if let Some(get) = trap {
            return self.call_val(
                &get,
                &[
                    target.borrow().clone(),
                    JsVal::Str(key.to_string()),
                    receiver.clone(),
                ],
                JsVal::Undef,
                env,
            );
        }
        self.member_get(&target.borrow(), key, env)
    }

    pub(super) fn object_set_prototype_of(&self, args: &[JsVal]) -> Result<JsVal, ()> {
        let obj = args.first().cloned().ok_or(())?;
        let proto = args.get(1).cloned().unwrap_or(JsVal::Null);
        match &obj {
            JsVal::Object { proto: slot, .. } => {
                *slot.borrow_mut() = proto;
            }
            _ => {}
        }
        Ok(obj)
    }

    pub(super) fn reflect_construct(
        &self,
        args: &[JsVal],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        let ctor = args.first().cloned().ok_or(())?;
        let new_target = args.get(2).cloned().unwrap_or_else(|| ctor.clone());
        with_new_target(new_target, || self.eval_new(&ctor, &[], env))
    }

    pub(super) fn reflect_get(
        &self,
        args: &[JsVal],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        let target = args.first().ok_or(())?;
        let key = match args.get(1) {
            Some(JsVal::Str(s)) => s.as_str(),
            _ => return Err(()),
        };
        self.member_get(target, key, env)
    }
}
