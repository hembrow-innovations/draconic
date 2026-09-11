use super::*;

impl super::World {
    pub(super) fn eval_key(&mut self, expr: &Expr) -> Result<Result<String, Flow>, ()> {
        match expr {
            Expr::String { value, .. } => Ok(Ok(js_string_to_utf8(value))),
            e => match self.eval_expr(e)? {
                Ok(JsVal::Str(s)) => Ok(Ok(s)),
                Ok(JsVal::Num(n)) => Ok(Ok(format!("{}", n as i64))),
                Ok(_) => Err(()),
                Err(f) => Ok(Err(f)),
            },
        }
    }

    pub(super) fn array_to_vec(&self, v: &JsVal) -> Result<Vec<JsVal>, ()> {
        let JsVal::Object(idx) = v else {
            return Err(());
        };
        let rec = self.objects.get(*idx).ok_or(())?;
        let len = match rec.props.get("length") {
            Some(JsVal::Num(n)) => *n as usize,
            _ => 0,
        };
        let mut out = Vec::new();
        for i in 0..len {
            out.push(
                rec.props
                    .get(&i.to_string())
                    .cloned()
                    .unwrap_or(JsVal::Undef),
            );
        }
        Ok(out)
    }

    pub(super) fn object_id(v: &JsVal) -> Option<usize> {
        match v {
            JsVal::Object(i) => Some(*i),
            JsVal::Fn { obj_idx, .. } => Some(*obj_idx),
            JsVal::Proxy(i) => Some(10_000 + *i), // distinct namespace
            _ => None,
        }
    }

    pub(super) fn object_get(&mut self, obj: &JsVal, key: &str) -> Result<JsVal, ()> {
        match obj {
            JsVal::Builtin(Builtin::Object) => match key {
                "defineProperty" => Ok(JsVal::Builtin(Builtin::ObjectDefineProperty)),
                "getOwnPropertyDescriptor" => {
                    Ok(JsVal::Builtin(Builtin::ObjectGetOwnPropertyDescriptor))
                }
                "isExtensible" => Ok(JsVal::Builtin(Builtin::ObjectIsExtensible)),
                "setPrototypeOf" => Ok(JsVal::Builtin(Builtin::ObjectSetPrototypeOf)),
                "prototype" => Ok(JsVal::Object(OBJECT_PROTOTYPE_IDX)),
                _ => Ok(JsVal::Undef),
            },
            JsVal::Builtin(Builtin::Function) => match key {
                "prototype" => Ok(JsVal::Object(FUNCTION_PROTOTYPE_IDX)),
                _ => Ok(JsVal::Undef),
            },
            JsVal::Builtin(Builtin::Reflect) => match key {
                "construct" => Ok(JsVal::Builtin(Builtin::ReflectConstruct)),
                "get" => Ok(JsVal::Builtin(Builtin::ReflectGet)),
                _ => Ok(JsVal::Undef),
            },
            JsVal::WeakMap(_) => match key {
                "set" | "get" | "has" | "delete" => Ok(JsVal::Str(format!("__wm_{key}"))),
                _ => Ok(JsVal::Undef),
            },
            JsVal::WeakSet(_) => match key {
                "add" | "has" | "delete" => Ok(JsVal::Str(format!("__ws_{key}"))),
                _ => Ok(JsVal::Undef),
            },
            JsVal::Fn { obj_idx, fn_idx } => {
                // Own data props (e.g. static method named `call`) win over
                // Function.prototype.call.
                if let Some(v) = self.objects.get(*obj_idx).and_then(|o| o.props.get(key)) {
                    return Ok(v.clone());
                }
                if key == "call" {
                    return Ok(JsVal::Str(format!("__fn_call_{fn_idx}")));
                }
                // Walk [[Prototype]] of the function object.
                let proto = self
                    .objects
                    .get(*obj_idx)
                    .map(|o| o.proto.clone())
                    .unwrap_or(JsVal::Null);
                if matches!(proto, JsVal::Null) {
                    return Ok(JsVal::Undef);
                }
                self.object_get(&proto, key)
            }
            JsVal::Object(idx) => {
                let mut cur = *idx;
                loop {
                    let rec = self.objects.get(cur).ok_or(())?;
                    if let Some(v) = rec.props.get(key) {
                        return Ok(v.clone());
                    }
                    match &rec.proto {
                        JsVal::Object(p) => cur = *p,
                        JsVal::Null => return Ok(JsVal::Undef),
                        JsVal::Fn { obj_idx, .. } => cur = *obj_idx,
                        _ => return Ok(JsVal::Undef),
                    }
                    if cur == OBJECT_PROTOTYPE_IDX && !self.objects[cur].props.contains_key(key) {
                        // also check Function.prototype for call
                        if key == "call" {
                            // not on plain objects
                        }
                        return Ok(JsVal::Undef);
                    }
                }
            }
            JsVal::Proxy(idx) => {
                let rec = self.proxies.get(*idx).ok_or(())?.clone();
                if let Some(trap) = rec.get_trap {
                    let args = vec![
                        rec.target.clone(),
                        JsVal::Str(key.to_string()),
                        JsVal::Proxy(*idx),
                    ];
                    return self.call_fn_idx(trap, JsVal::Undef, &args);
                }
                self.object_get(&rec.target, key)
            }
            _ => Ok(JsVal::Undef),
        }
    }

    pub(super) fn object_set(&mut self, obj: &JsVal, key: &str, val: JsVal) -> Result<(), ()> {
        let idx = match obj {
            JsVal::Object(i) => *i,
            JsVal::Fn { obj_idx, .. } => *obj_idx,
            _ => return Err(()),
        };
        let rec = self.objects.get_mut(idx).ok_or(())?;
        object_set_prop(rec, key.to_string(), val);
        Ok(())
    }

    pub(super) fn object_delete(&mut self, obj: &JsVal, key: &str) -> Result<bool, ()> {
        let idx = match obj {
            JsVal::Object(i) => *i,
            JsVal::Fn { obj_idx, .. } => *obj_idx,
            _ => return Ok(true),
        };
        let rec = self.objects.get_mut(idx).ok_or(())?;
        if rec.props.remove(key).is_some() {
            rec.keys.retain(|k| k != key);
            Ok(true)
        } else {
            Ok(true)
        }
    }

    pub(super) fn construct(
        &mut self,
        callee: &JsVal,
        args: &[JsVal],
        new_target: &JsVal,
    ) -> Result<JsVal, ()> {
        match callee {
            JsVal::Builtin(Builtin::WeakMap) => {
                let idx = self.weak_maps.len();
                self.weak_maps.push(WeakMapRec {
                    entries: Vec::new(),
                });
                Ok(JsVal::WeakMap(idx))
            }
            JsVal::Builtin(Builtin::WeakSet) => {
                let idx = self.weak_sets.len();
                self.weak_sets.push(WeakSetRec { keys: Vec::new() });
                Ok(JsVal::WeakSet(idx))
            }
            JsVal::Builtin(Builtin::TypeError) | JsVal::Builtin(Builtin::ReferenceError) => {
                let msg = match args.first() {
                    Some(JsVal::Str(s)) => s.clone(),
                    _ => String::new(),
                };
                let mut rec = empty_object();
                object_set_prop(&mut rec, "message".into(), JsVal::Str(msg));
                let idx = self.objects.len();
                self.objects.push(rec);
                Ok(JsVal::Object(idx))
            }
            JsVal::Builtin(Builtin::Proxy) => {
                if args.len() < 2 {
                    return Err(());
                }
                let target = args[0].clone();
                let handler = &args[1];
                let get_trap = match self.object_get(handler, "get")? {
                    JsVal::Fn { fn_idx, .. } => Some(fn_idx),
                    JsVal::Undef => None,
                    _ => None,
                };
                let idx = self.proxies.len();
                self.proxies.push(ProxyRec { target, get_trap });
                Ok(JsVal::Proxy(idx))
            }
            JsVal::Fn { fn_idx, .. } => {
                // [[Prototype]] of instance = newTarget.prototype
                let proto = self.object_get(new_target, "prototype")?;
                let mut rec = empty_object();
                rec.proto = match proto {
                    JsVal::Object(_) | JsVal::Fn { .. } | JsVal::Null => proto,
                    _ => JsVal::Object(OBJECT_PROTOTYPE_IDX),
                };
                let this_idx = self.objects.len();
                self.objects.push(rec);
                let this_obj = JsVal::Object(this_idx);
                let ret =
                    self.call_fn_idx_new(*fn_idx, this_obj.clone(), args, new_target.clone())?;
                match ret {
                    JsVal::Object(_) | JsVal::Fn { .. } | JsVal::Proxy(_) => Ok(ret),
                    JsVal::Undef => Ok(this_obj),
                    _ => Ok(this_obj),
                }
            }
            JsVal::Proxy(idx) => {
                let rec = self.proxies.get(*idx).ok_or(())?.clone();
                self.construct(&rec.target, args, new_target)
            }
            _ => Err(()),
        }
    }

    pub(super) fn call(
        &mut self,
        func: &JsVal,
        this_arg: JsVal,
        args: &[JsVal],
    ) -> Result<JsVal, ()> {
        // WeakMap/WeakSet methods encoded as magic strings
        if let JsVal::Str(s) = func {
            if let Some(method) = s.strip_prefix("__wm_") {
                return self.weak_map_method(&this_arg, method, args);
            }
            if let Some(method) = s.strip_prefix("__ws_") {
                return self.weak_set_method(&this_arg, method, args);
            }
            if let Some(rest) = s.strip_prefix("__fn_call_") {
                let fn_idx: usize = rest.parse().map_err(|_| ())?;
                let this = args.first().cloned().unwrap_or(JsVal::Undef);
                let rest_args: Vec<JsVal> = args.iter().skip(1).cloned().collect();
                return self.call_fn_idx(fn_idx, this, &rest_args);
            }
        }
        match func {
            JsVal::Fn { fn_idx, .. } => self.call_fn_idx(*fn_idx, this_arg, args),
            JsVal::Builtin(Builtin::ObjectDefineProperty) => {
                if args.len() < 3 {
                    return Err(());
                }
                let key = match &args[1] {
                    JsVal::Str(s) => s.clone(),
                    JsVal::Num(n) => format!("{}", *n as i64),
                    _ => return Err(()),
                };
                let value = self.descriptor_value(&args[2])?;
                self.object_set(&args[0], &key, value)?;
                Ok(args[0].clone())
            }
            JsVal::Builtin(Builtin::ObjectGetOwnPropertyDescriptor) => {
                if args.len() < 2 {
                    return Err(());
                }
                let key = match &args[1] {
                    JsVal::Str(s) => s.clone(),
                    JsVal::Num(n) => format!("{}", *n as i64),
                    _ => return Err(()),
                };
                let val = match &args[0] {
                    JsVal::Object(i) | JsVal::Fn { obj_idx: i, .. } => {
                        self.objects.get(*i).ok_or(())?.props.get(&key).cloned()
                    }
                    _ => None,
                };
                match val {
                    Some(v) => Ok(self.make_data_descriptor(v)),
                    None => Ok(JsVal::Undef),
                }
            }
            JsVal::Builtin(Builtin::ObjectIsExtensible) => {
                let obj = args.first().ok_or(())?;
                let ext = match obj {
                    JsVal::Object(i) | JsVal::Fn { obj_idx: i, .. } => {
                        self.objects.get(*i).ok_or(())?.extensible
                    }
                    _ => true,
                };
                Ok(JsVal::Bool(ext))
            }
            JsVal::Builtin(Builtin::ObjectSetPrototypeOf) => {
                if args.len() < 2 {
                    return Err(());
                }
                let idx = match &args[0] {
                    JsVal::Object(i) | JsVal::Fn { obj_idx: i, .. } => *i,
                    _ => return Err(()),
                };
                self.objects.get_mut(idx).ok_or(())?.proto = args[1].clone();
                Ok(args[0].clone())
            }
            JsVal::Builtin(Builtin::ReflectConstruct) => {
                if args.len() < 2 {
                    return Err(());
                }
                let argv = self.array_to_vec(&args[1])?;
                let nt = if args.len() >= 3 {
                    args[2].clone()
                } else {
                    args[0].clone()
                };
                self.construct(&args[0], &argv, &nt)
            }
            JsVal::Builtin(Builtin::ReflectGet) => {
                if args.len() < 2 {
                    return Err(());
                }
                let key = match &args[1] {
                    JsVal::Str(s) => s.clone(),
                    _ => return Err(()),
                };
                self.object_get(&args[0], &key)
            }
            JsVal::Builtin(Builtin::TypeError) | JsVal::Builtin(Builtin::ReferenceError) => {
                // called without new — still make error object
                self.construct(func, args, func)
            }
            _ => Err(()),
        }
    }

    pub(super) fn weak_map_method(
        &mut self,
        this_arg: &JsVal,
        method: &str,
        args: &[JsVal],
    ) -> Result<JsVal, ()> {
        let JsVal::WeakMap(idx) = this_arg else {
            return Err(());
        };
        let idx = *idx;
        match method {
            "set" => {
                let k = args.first().ok_or(())?;
                let kid = Self::object_id(k).ok_or(())?;
                let v = args.get(1).cloned().unwrap_or(JsVal::Undef);
                let wm = self.weak_maps.get_mut(idx).ok_or(())?;
                if let Some((_, slot)) = wm.entries.iter_mut().find(|(id, _)| *id == kid) {
                    *slot = v;
                } else {
                    wm.entries.push((kid, v));
                }
                Ok(this_arg.clone())
            }
            "get" => {
                let k = args.first().ok_or(())?;
                let Some(kid) = Self::object_id(k) else {
                    return Ok(JsVal::Undef);
                };
                let wm = self.weak_maps.get(idx).ok_or(())?;
                Ok(wm
                    .entries
                    .iter()
                    .find(|(id, _)| *id == kid)
                    .map(|(_, v)| v.clone())
                    .unwrap_or(JsVal::Undef))
            }
            "has" => {
                let k = args.first().unwrap_or(&JsVal::Undef);
                let Some(kid) = Self::object_id(k) else {
                    return Ok(JsVal::Bool(false));
                };
                let wm = self.weak_maps.get(idx).ok_or(())?;
                Ok(JsVal::Bool(wm.entries.iter().any(|(id, _)| *id == kid)))
            }
            "delete" => {
                let k = args.first().unwrap_or(&JsVal::Undef);
                let Some(kid) = Self::object_id(k) else {
                    return Ok(JsVal::Bool(false));
                };
                let wm = self.weak_maps.get_mut(idx).ok_or(())?;
                let before = wm.entries.len();
                wm.entries.retain(|(id, _)| *id != kid);
                Ok(JsVal::Bool(wm.entries.len() < before))
            }
            _ => Err(()),
        }
    }

    pub(super) fn weak_set_method(
        &mut self,
        this_arg: &JsVal,
        method: &str,
        args: &[JsVal],
    ) -> Result<JsVal, ()> {
        let JsVal::WeakSet(idx) = this_arg else {
            return Err(());
        };
        let idx = *idx;
        match method {
            "add" => {
                let k = args.first().ok_or(())?;
                let kid = Self::object_id(k).ok_or(())?;
                let ws = self.weak_sets.get_mut(idx).ok_or(())?;
                if !ws.keys.contains(&kid) {
                    ws.keys.push(kid);
                }
                Ok(this_arg.clone())
            }
            "has" => {
                let k = args.first().unwrap_or(&JsVal::Undef);
                let Some(kid) = Self::object_id(k) else {
                    return Ok(JsVal::Bool(false));
                };
                let ws = self.weak_sets.get(idx).ok_or(())?;
                Ok(JsVal::Bool(ws.keys.contains(&kid)))
            }
            "delete" => {
                let k = args.first().unwrap_or(&JsVal::Undef);
                let Some(kid) = Self::object_id(k) else {
                    return Ok(JsVal::Bool(false));
                };
                let ws = self.weak_sets.get_mut(idx).ok_or(())?;
                let before = ws.keys.len();
                ws.keys.retain(|id| *id != kid);
                Ok(JsVal::Bool(ws.keys.len() < before))
            }
            _ => Err(()),
        }
    }

    pub(super) fn call_fn_idx(
        &mut self,
        fn_idx: usize,
        this_arg: JsVal,
        args: &[JsVal],
    ) -> Result<JsVal, ()> {
        let is_arrow = self.fns.get(fn_idx).map(|f| f.is_arrow).unwrap_or(false);
        // Arrows inherit outer `new.target`; non-arrows clear it on ordinary call.
        let nt = if is_arrow {
            CURRENT_NEW_TARGET.with(|c| c.borrow().clone())
        } else {
            JsVal::Undef
        };
        self.call_fn_idx_new(fn_idx, this_arg, args, nt)
    }

    pub(super) fn call_fn_idx_new(
        &mut self,
        fn_idx: usize,
        this_arg: JsVal,
        args: &[JsVal],
        new_target: JsVal,
    ) -> Result<JsVal, ()> {
        let rec = self.fns.get(fn_idx).ok_or(())?.clone();
        let this_for_body = if rec.is_arrow {
            CURRENT_THIS.with(|c| c.borrow().clone())
        } else {
            this_arg
        };
        let mut saved_local: Vec<(LocalId, Option<JsVal>)> = Vec::new();
        let mut saved_name: Vec<(String, Option<JsVal>)> = Vec::new();
        let mut ai = 0usize;
        for p in &rec.params {
            let val = if p.rest {
                let rest: Vec<JsVal> = args.get(ai..).unwrap_or(&[]).to_vec();
                let mut arr = empty_object();
                for (i, v) in rest.iter().enumerate() {
                    object_set_prop(&mut arr, i.to_string(), v.clone());
                }
                object_set_prop(&mut arr, "length".into(), JsVal::Num(rest.len() as f64));
                let idx = self.objects.len();
                self.objects.push(arr);
                ai = args.len();
                JsVal::Object(idx)
            } else {
                let v = args.get(ai).cloned().unwrap_or(JsVal::Undef);
                ai += 1;
                v
            };
            match &p.bind {
                ParamBind::Local(id) => {
                    saved_local.push((*id, self.env.get(id).cloned()));
                    self.env.insert(*id, val);
                }
                ParamBind::Name(n) => {
                    saved_name.push((n.clone(), self.name_env.get(n).cloned()));
                    self.name_env.insert(n.clone(), val);
                }
            }
        }
        let flow = with_this_new(this_for_body, new_target, || self.eval_body(&rec.body))?;
        for (id, prev) in saved_local {
            match prev {
                Some(v) => {
                    self.env.insert(id, v);
                }
                None => {
                    self.env.remove(&id);
                }
            }
        }
        for (n, prev) in saved_name {
            match prev {
                Some(v) => {
                    self.name_env.insert(n, v);
                }
                None => {
                    self.name_env.remove(&n);
                }
            }
        }
        match flow {
            Flow::Normal => Ok(JsVal::Undef),
            Flow::Return(v) => Ok(v),
            Flow::Throw(_) => Err(()),
        }
    }

    pub(super) fn descriptor_value(&self, desc: &JsVal) -> Result<JsVal, ()> {
        match desc {
            JsVal::Object(idx) => Ok(self
                .objects
                .get(*idx)
                .ok_or(())?
                .props
                .get("value")
                .cloned()
                .unwrap_or(JsVal::Undef)),
            JsVal::Fn { .. } => Ok(desc.clone()),
            _ => Ok(desc.clone()),
        }
    }

    pub(super) fn make_data_descriptor(&mut self, value: JsVal) -> JsVal {
        let mut rec = empty_object();
        object_set_prop(&mut rec, "value".into(), value);
        object_set_prop(&mut rec, "writable".into(), JsVal::Bool(true));
        object_set_prop(&mut rec, "enumerable".into(), JsVal::Bool(true));
        object_set_prop(&mut rec, "configurable".into(), JsVal::Bool(true));
        let idx = self.objects.len();
        self.objects.push(rec);
        JsVal::Object(idx)
    }
}
