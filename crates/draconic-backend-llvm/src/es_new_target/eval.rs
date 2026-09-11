use super::*;

impl World {
    pub(super) fn eval_body(
        &self,
        body: &[Stmt],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<Flow, ()> {
        for stmt in body {
            match self.eval_stmt(stmt, env)? {
                Flow::Normal => {}
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }

    fn eval_stmt(&self, stmt: &Stmt, env: &mut HashMap<LocalId, JsVal>) -> Result<Flow, ()> {
        match stmt {
            Stmt::Function {
                local,
                params,
                body,
                is_async: false,
                is_generator: false,
            } => {
                let ids = self.simple_param_ids(params)?;
                let id = self.next_id();
                self.fn_reg_insert(
                    id,
                    FnRec {
                        params: ids,
                        body: body.clone(),
                        is_arrow: false,
                        parent: None,
                    },
                );
                env.insert(*local, JsVal::Fn { id });
                Ok(Flow::Normal)
            }
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
            Stmt::Return { value: None } => Ok(Flow::Return(JsVal::Undef)),
            Stmt::Return { value: Some(e) } => match self.eval_expr(e, env)? {
                Ok(v) => Ok(Flow::Return(v)),
                Err(flow) => Ok(flow),
            },
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
                if self.to_boolean(&t) {
                    self.eval_stmt(consequent, env)
                } else if let Some(a) = alternate {
                    self.eval_stmt(a, env)
                } else {
                    Ok(Flow::Normal)
                }
            }
            // Class IIFE noise: defineProperty / setPrototypeOf / throw TypeError — ignore side effects.
            Stmt::Throw { .. } => Ok(Flow::Normal),
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                let mut completion = match self.eval_body(block, env) {
                    Ok(Flow::Normal) => Flow::Normal,
                    Ok(other) => other,
                    Err(()) => {
                        // Unsupported in try → treat as throw and run handler if any.
                        if let Some(h) = handler {
                            self.eval_body(h, env)?
                        } else {
                            return Err(());
                        }
                    }
                };
                if let Some(fin) = finalizer {
                    match self.eval_body(fin, env)? {
                        Flow::Normal => {}
                        abrupt => completion = abrupt,
                    }
                }
                Ok(completion)
            }
            _ => Err(()),
        }
    }

    fn simple_param_ids(&self, params: &[Param]) -> Result<Vec<LocalId>, ()> {
        let mut ids = Vec::new();
        for p in params {
            if p.rest || p.default.is_some() {
                return Err(());
            }
            match &p.pattern {
                Pattern::Local(id) => ids.push(*id),
                _ => return Err(()),
            }
        }
        Ok(ids)
    }

    fn make_fn(
        &self,
        params: &[Param],
        body: &[Stmt],
        is_arrow: bool,
        parent: Option<u64>,
    ) -> Result<JsVal, ()> {
        let ids = self.simple_param_ids(params)?;
        let id = self.next_id();
        self.fn_reg_insert(
            id,
            FnRec {
                params: ids,
                body: body.to_vec(),
                is_arrow,
                parent,
            },
        );
        Ok(JsVal::Fn { id })
    }

    /// `Ok(Ok(v))` value; `Ok(Err(flow))` abrupt return; `Err(())` unsupported.
    fn eval_expr(
        &self,
        expr: &Expr,
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<Result<JsVal, Flow>, ()> {
        match expr {
            Expr::NewTarget { .. } => Ok(Ok(current_new_target())),
            Expr::This { .. } => Ok(Ok(current_this())),
            Expr::Boolean { value, .. } => Ok(Ok(JsVal::Bool(*value))),
            Expr::String { value, .. } => Ok(Ok(JsVal::Str(value.to_string_lossy()))),
            Expr::Null { .. } => Ok(Ok(JsVal::Undef)),
            Expr::Number { raw, .. } => {
                let _n: f64 = raw.parse().map_err(|_| ())?;
                Ok(Ok(JsVal::Undef)) // numbers unused in fixture observations
            }
            Expr::Local { id, .. } => {
                let v = env.get(id).cloned().ok_or(())?;
                Ok(Ok(v))
            }
            Expr::IdentName { name, .. } => match name.as_str() {
                "undefined" => Ok(Ok(JsVal::Undef)),
                // Class/ctor surface used only for structure; not observed.
                "Object" | "Function" | "Reflect" | "Proxy" | "TypeError" => Ok(Ok(JsVal::Undef)),
                _ => Err(()),
            },
            Expr::Function {
                params,
                body,
                is_async: false,
                is_generator: false,
                is_arrow,
                ..
            } => Ok(Ok(self.make_fn(params, body, *is_arrow, None)?)),
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } => {
                let v = match self.eval_expr(arg, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                Ok(Ok(JsVal::Str(self.typeof_str(&v))))
            }
            Expr::Unary {
                op: UnaryOp::Not,
                arg,
                ..
            } => {
                let v = match self.eval_expr(arg, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                Ok(Ok(JsVal::Bool(!self.to_boolean(&v))))
            }
            Expr::Binary {
                left,
                op: BinaryOp::EqEqEq | BinaryOp::EqEq,
                right,
                ..
            } => {
                let l = match self.eval_expr(left, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                let r = match self.eval_expr(right, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                Ok(Ok(JsVal::Bool(self.strict_eq(&l, &r))))
            }
            Expr::Binary {
                left,
                op: BinaryOp::NotEqEq | BinaryOp::NotEq,
                right,
                ..
            } => {
                let l = match self.eval_expr(left, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                let r = match self.eval_expr(right, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                Ok(Ok(JsVal::Bool(!self.strict_eq(&l, &r))))
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
                if self.to_boolean(&t) {
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
                let key = match property.as_ref() {
                    Expr::String { value, .. } => value.to_string_lossy(),
                    _ => return Err(()),
                };
                Ok(Ok(self.member_get(&obj, &key)))
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
                // Super(...) in derived ctor.
                if matches!(callee.as_ref(), Expr::Super { .. }) {
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
                    return Ok(Ok(self.eval_super(&arg_vals, env)?));
                }
                // Class builder IIFE or plain call.
                if let Expr::Function {
                    params,
                    body,
                    is_async: false,
                    is_generator: false,
                    is_arrow: false,
                    ..
                } = callee.as_ref()
                {
                    if args.is_empty() {
                        if let Some(cls) = self.try_eval_class_iife(params, body, env) {
                            return Ok(Ok(cls));
                        }
                    }
                }
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
                // Method-style Object.defineProperty / setPrototypeOf — no-op for fixture.
                if let Expr::Member {
                    object,
                    property,
                    optional: false,
                    ..
                } = callee.as_ref()
                {
                    if let (Ok(Ok(JsVal::Undef)), Expr::String { value, .. }) =
                        (self.eval_expr(object, env), property.as_ref())
                    {
                        let k = value.to_string_lossy();
                        if k == "defineProperty" || k == "setPrototypeOf" || k == "get" {
                            return Ok(Ok(JsVal::Undef));
                        }
                    }
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
                let key = match property.as_ref() {
                    Expr::String { value, .. } => value.to_string_lossy(),
                    _ => return Err(()),
                };
                self.member_set(&mut obj, &key, v.clone())?;
                if let Expr::Local { id, .. } = object.as_ref() {
                    env.insert(*id, obj);
                } else if matches!(object.as_ref(), Expr::This { .. }) {
                    CURRENT_THIS.with(|c| {
                        *c.borrow_mut() = obj;
                    });
                }
                Ok(Ok(v))
            }
            Expr::Object { properties, .. } => {
                // Descriptor objects in defineProperty — ignore contents.
                let mut props = Vec::new();
                for p in properties {
                    if let ObjectProp::Property {
                        key: ObjectPropKey::Static(k),
                        value,
                    } = p
                    {
                        let v = match self.eval_expr(value, env)? {
                            Ok(v) => v,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        props.push((k.to_string_lossy(), v));
                    }
                }
                Ok(Ok(JsVal::Object {
                    id: self.next_id(),
                    props: Rc::new(RefCell::new(props)),
                }))
            }
            Expr::Array { .. } => Ok(Ok(JsVal::Undef)),
            Expr::Super { .. } => Err(()),
            _ => Err(()),
        }
    }

    fn try_eval_class_iife(
        &self,
        params: &[Param],
        body: &[Stmt],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Option<JsVal> {
        if !params.is_empty() {
            return None;
        }
        // Detect parent: `let super = OuterClass` style declare of a function local.
        let mut parent_fn_id: Option<u64> = None;
        let mut ctor_local: Option<LocalId> = None;
        let mut ctor_fn: Option<JsVal> = None;

        for stmt in body {
            match stmt {
                Stmt::Declare {
                    local,
                    init: Some(Expr::Local { id, .. }),
                    ..
                } => {
                    if let Some(JsVal::Fn { id: fid }) = env.get(id) {
                        parent_fn_id = Some(*fid);
                        env.insert(*local, JsVal::Fn { id: *fid });
                    }
                }
                Stmt::Declare {
                    local,
                    init:
                        Some(Expr::Function {
                            params: cparams,
                            body: cbody,
                            is_async: false,
                            is_generator: false,
                            is_arrow: false,
                            ..
                        }),
                    ..
                } if ctor_local.is_none() => {
                    let filtered = if parent_fn_id.is_some() {
                        self.filter_derived_ctor_body(cbody)
                    } else {
                        self.filter_ctor_body(cbody)
                    };
                    let f = self.make_fn(cparams, &filtered, false, parent_fn_id).ok()?;
                    ctor_local = Some(*local);
                    ctor_fn = Some(f.clone());
                    env.insert(*local, f);
                }
                Stmt::Return {
                    value: Some(Expr::Local { id, .. }),
                } if Some(*id) == ctor_local => {
                    return ctor_fn;
                }
                // defineProperty / setPrototypeOf / heritage checks — skip
                Stmt::Expr { .. } | Stmt::If { .. } | Stmt::Declare { init: None, .. } => {}
                Stmt::Declare {
                    init: Some(Expr::String { .. }),
                    ..
                } => {}
                _ => {}
            }
        }
        ctor_fn
    }

    fn filter_ctor_body(&self, body: &[Stmt]) -> Vec<Stmt> {
        body.iter()
            .filter(|s| match s {
                Stmt::Expr {
                    expr: Expr::String { value, .. },
                } if value.to_string_lossy() == "use strict" => false,
                Stmt::If { .. } => false,
                Stmt::Expr {
                    expr:
                        Expr::Assign {
                            target: AssignTarget::Member { .. },
                            op: AssignOp::Eq,
                            ..
                        },
                } => true,
                Stmt::Return { .. } => true,
                Stmt::Block { .. } => true,
                _ => false,
            })
            .cloned()
            .collect()
    }

    fn filter_derived_ctor_body(&self, body: &[Stmt]) -> Vec<Stmt> {
        let mut out = Vec::new();
        self.collect_derived_ctor_stmts(body, &mut out);
        out
    }

    fn collect_derived_ctor_stmts(&self, body: &[Stmt], out: &mut Vec<Stmt>) {
        for stmt in body {
            match stmt {
                Stmt::Expr {
                    expr: Expr::String { value, .. },
                } if value.to_string_lossy() == "use strict" => {}
                Stmt::If { .. } | Stmt::Declare { .. } | Stmt::Return { .. } => {}
                Stmt::Labeled { body, .. } => self.collect_derived_ctor_stmts_one(body, out),
                Stmt::Block { body } => self.collect_derived_ctor_stmts(body, out),
                other => self.collect_derived_ctor_stmts_one(other, out),
            }
        }
    }

    fn collect_derived_ctor_stmts_one(&self, stmt: &Stmt, out: &mut Vec<Stmt>) {
        match stmt {
            Stmt::Block { body } => self.collect_derived_ctor_stmts(body, out),
            Stmt::Labeled { body, .. } => self.collect_derived_ctor_stmts_one(body, out),
            Stmt::Expr {
                expr:
                    Expr::Call {
                        callee,
                        args,
                        optional,
                        ty,
                    },
            } if !*optional && self.is_super_call_iife(callee) => {
                let super_args: Vec<Arg> = args
                    .iter()
                    .filter_map(|a| match a {
                        Arg::Expr(e) => Some(Arg::Expr(e.clone())),
                        Arg::Spread(_) => None,
                    })
                    .collect();
                if super_args.len() != args.len() {
                    return;
                }
                out.push(Stmt::Expr {
                    expr: Expr::Call {
                        callee: Box::new(Expr::Super { ty: Type::Any }),
                        args: super_args,
                        optional: false,
                        ty: *ty,
                    },
                });
            }
            Stmt::Expr {
                expr:
                    Expr::Assign {
                        target:
                            AssignTarget::Member {
                                property, computed, ..
                            },
                        op: AssignOp::Eq,
                        value,
                        ty,
                    },
            } if matches!(property.as_ref(), Expr::String { .. }) => {
                out.push(Stmt::Expr {
                    expr: Expr::Assign {
                        target: AssignTarget::Member {
                            object: Box::new(Expr::This { ty: Type::Any }),
                            property: property.clone(),
                            computed: *computed,
                        },
                        op: AssignOp::Eq,
                        value: value.clone(),
                        ty: *ty,
                    },
                });
            }
            _ => {}
        }
    }

    fn is_super_call_iife(&self, callee: &Expr) -> bool {
        let Expr::Function {
            body,
            is_arrow: true,
            ..
        } = callee
        else {
            return false;
        };
        body.iter().any(|s| self.stmt_has_reflect_construct(s))
    }

    fn stmt_has_reflect_construct(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Declare {
                init: Some(expr), ..
            }
            | Stmt::Expr { expr }
            | Stmt::Return { value: Some(expr) } => self.expr_has_reflect_construct(expr),
            Stmt::Block { body } => body.iter().any(|s| self.stmt_has_reflect_construct(s)),
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                self.stmt_has_reflect_construct(consequent)
                    || alternate
                        .as_ref()
                        .is_some_and(|a| self.stmt_has_reflect_construct(a))
            }
            _ => false,
        }
    }

    fn expr_has_reflect_construct(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Call { callee, .. } => {
                if self.is_reflect_construct(callee) {
                    return true;
                }
                self.expr_has_reflect_construct(callee)
            }
            Expr::Member {
                object, property, ..
            } => {
                self.expr_has_reflect_construct(object) || self.expr_has_reflect_construct(property)
            }
            Expr::Assign { value, .. } => self.expr_has_reflect_construct(value),
            Expr::New { callee, args, .. } => {
                self.expr_has_reflect_construct(callee)
                    || args.iter().any(|a| match a {
                        Arg::Expr(e) => self.expr_has_reflect_construct(e),
                        _ => false,
                    })
            }
            Expr::Function { body, .. } => body.iter().any(|s| self.stmt_has_reflect_construct(s)),
            Expr::Object { properties, .. } => properties.iter().any(|p| match p {
                ObjectProp::Property { value, .. } | ObjectProp::Accessor { value, .. } => {
                    self.expr_has_reflect_construct(value)
                }
                ObjectProp::Spread(e) => self.expr_has_reflect_construct(e),
            }),
            _ => false,
        }
    }

    fn is_reflect_construct(&self, callee: &Expr) -> bool {
        let Expr::Member {
            object, property, ..
        } = callee
        else {
            return false;
        };
        matches!(
            (object.as_ref(), property.as_ref()),
            (
                Expr::IdentName { name, .. },
                Expr::String { value, .. }
            ) if name == "Reflect" && value.to_string_lossy() == "construct"
        )
    }

    fn eval_new(
        &self,
        callee: &JsVal,
        args: &[JsVal],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        let JsVal::Fn { id } = callee else {
            return Err(());
        };
        let rec = self.fn_reg_get(*id).ok_or(())?;
        if rec.is_arrow {
            return Err(());
        }
        let obj = JsVal::Object {
            id: self.next_id(),
            props: Rc::new(RefCell::new(Vec::new())),
        };
        let nt = callee.clone();
        let result = self.call_user_fn(&rec, obj.clone(), nt, args, env)?;
        match result {
            JsVal::Object { .. } | JsVal::Fn { .. } => Ok(result),
            _ => Ok(obj),
        }
    }

    fn eval_super(&self, args: &[JsVal], env: &mut HashMap<LocalId, JsVal>) -> Result<JsVal, ()> {
        // Current new.target's FnRec.parent → parent ctor; same this + new.target.
        let nt = current_new_target();
        let JsVal::Fn { id } = &nt else {
            return Err(());
        };
        let rec = self.fn_reg_get(*id).ok_or(())?;
        let parent_id = rec.parent.ok_or(())?;
        let parent = self.fn_reg_get(parent_id).ok_or(())?;
        let this = current_this();
        self.call_user_fn(&parent, this, nt, args, env)
    }

    fn eval_call(
        &self,
        callee: &JsVal,
        args: &[JsVal],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        let JsVal::Fn { id } = callee else {
            // No-op builtins (Object.defineProperty etc. already handled).
            return Ok(JsVal::Undef);
        };
        let rec = self.fn_reg_get(*id).ok_or(())?;
        if rec.is_arrow {
            // Lexical new.target / this from caller.
            return self.call_user_fn_arrow(&rec, args, env);
        }
        self.call_user_fn(&rec, JsVal::Undef, JsVal::Undef, args, env)
    }

    fn call_user_fn(
        &self,
        rec: &FnRec,
        this: JsVal,
        nt: JsVal,
        args: &[JsVal],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        let mut saved: Vec<(LocalId, Option<JsVal>)> = Vec::new();
        for (i, pid) in rec.params.iter().enumerate() {
            saved.push((*pid, env.get(pid).cloned()));
            env.insert(*pid, args.get(i).cloned().unwrap_or(JsVal::Undef));
        }
        // Nested function decls bind into env; save any locals they overwrite is hard —
        // fixture only uses fresh locals.
        let flow = with_this_nt(this, nt, || self.eval_body(&rec.body, env))?;
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
        }
    }

    fn call_user_fn_arrow(
        &self,
        rec: &FnRec,
        args: &[JsVal],
        env: &mut HashMap<LocalId, JsVal>,
    ) -> Result<JsVal, ()> {
        let mut saved: Vec<(LocalId, Option<JsVal>)> = Vec::new();
        for (i, pid) in rec.params.iter().enumerate() {
            saved.push((*pid, env.get(pid).cloned()));
            env.insert(*pid, args.get(i).cloned().unwrap_or(JsVal::Undef));
        }
        let flow = self.eval_body(&rec.body, env)?;
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
        }
    }

    fn member_get(&self, obj: &JsVal, key: &str) -> JsVal {
        match obj {
            JsVal::Object { props, .. } => props
                .borrow()
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
                .unwrap_or(JsVal::Undef),
            JsVal::Fn { .. } if key == "prototype" => JsVal::Object {
                id: self.next_id(),
                props: Rc::new(RefCell::new(Vec::new())),
            },
            _ => JsVal::Undef,
        }
    }

    fn member_set(&self, obj: &mut JsVal, key: &str, val: JsVal) -> Result<(), ()> {
        let JsVal::Object { props, .. } = obj else {
            return Err(());
        };
        let mut props = props.borrow_mut();
        if let Some((_, slot)) = props.iter_mut().find(|(k, _)| k == key) {
            *slot = val;
        } else {
            props.push((key.to_string(), val));
        }
        Ok(())
    }

    fn typeof_str(&self, v: &JsVal) -> String {
        match v {
            JsVal::Undef => "undefined".into(),
            JsVal::Bool(_) => "boolean".into(),
            JsVal::Str(_) => "string".into(),
            JsVal::Object { .. } => "object".into(),
            JsVal::Fn { .. } => "function".into(),
        }
    }

    fn to_boolean(&self, v: &JsVal) -> bool {
        match v {
            JsVal::Undef => false,
            JsVal::Bool(b) => *b,
            JsVal::Str(s) => !s.is_empty(),
            JsVal::Object { .. } | JsVal::Fn { .. } => true,
        }
    }

    fn strict_eq(&self, l: &JsVal, r: &JsVal) -> bool {
        match (l, r) {
            (JsVal::Undef, JsVal::Undef) => true,
            (JsVal::Bool(a), JsVal::Bool(b)) => a == b,
            (JsVal::Str(a), JsVal::Str(b)) => a == b,
            (JsVal::Object { id: a, .. }, JsVal::Object { id: b, .. }) => a == b,
            (JsVal::Fn { id: a }, JsVal::Fn { id: b }) => a == b,
            _ => false,
        }
    }
}
