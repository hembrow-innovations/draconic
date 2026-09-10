use std::fmt::Write as _;

use super::*;
use crate::emitter::escape_llvm_string;

impl<'a> super::Emitter<'a> {
    pub(super) fn new(module: &'a Module, info: ModuleInfo) -> Self {
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            tmp: 0,
            next_fn: 0,
            allocas: HashMap::new(),
            str_globals: HashMap::new(),
            helpers: String::new(),
            executor_params: HashMap::new(),
            reaction_params: HashMap::new(),
            reaction_captures: Vec::new(),
            async_fns: HashMap::new(),
        }
    }

    pub(super) fn finish(self) -> String {
        self.out
    }

    pub(super) fn fresh(&mut self) -> String {
        let n = self.tmp;
        self.tmp += 1;
        format!("%t{n}")
    }

    pub(super) fn fresh_fn(&mut self, prefix: &str) -> String {
        let n = self.next_fn;
        self.next_fn += 1;
        format!("d_{prefix}_{n}")
    }

    pub(super) fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM backend (N06.03–N06.11 Promise/async via Runtime ABI)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_PROMISE_DECLARES)).ok();
        writeln!(self.out).ok();

        // Pre-scan string constants from typeof etc. by emitting body into buffer first.
        self.body.clear();
        self.tmp = 0;

        for (id, kind) in self.info.user_locals.clone() {
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(id, ptr.clone());
            match kind {
                SlotKind::Number => {
                    writeln!(self.body, "  {ptr} = alloca i64, align 8").ok();
                    writeln!(self.body, "  store i64 0, ptr {ptr}").ok();
                }
                SlotKind::String | SlotKind::Object => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        // Drain microtasks before observing locals.
        writeln!(self.body, "  {}", JOB_DRAIN.call("")).ok();

        for (id, kind) in self.info.user_locals.clone() {
            let ptr = self.allocas.get(&id).cloned().unwrap();
            match kind {
                SlotKind::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i64, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_I64.call(&format!("i64 {v}"))).ok();
                }
                SlotKind::String => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
                SlotKind::Object => {
                    // Promise objects are not printed (observation is via number/string side effects).
                }
            }
        }

        // String globals
        for (content, gname) in self.str_globals.clone() {
            let n = content.len() + 1;
            let esc = escape_llvm_string(&content);
            writeln!(
                self.out,
                "@{gname} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
            )
            .ok();
        }
        if !self.str_globals.is_empty() {
            writeln!(self.out).ok();
        }

        // Helpers (executors + reactions)
        self.out.push_str(&self.helpers);
        if !self.helpers.is_empty() {
            writeln!(self.out).ok();
        }

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        writeln!(self.out, "  {}", GC_INIT.call("")).ok();
        self.out.push_str(&self.body);
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    pub(super) fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                if let Some(e) = init {
                    let v = self.emit_expr(e)?;
                    self.store_local(*local, &v)?;
                }
                Ok(())
            }
            Stmt::Expr { expr } => {
                let _ = self.emit_expr(expr)?;
                Ok(())
            }
            Stmt::Return { value } => {
                // Only valid inside reaction helpers — handled separately.
                let _ = value;
                Err(diag("return at top level not supported"))
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Function {
                local,
                params,
                body,
                is_async,
                is_generator,
            } => {
                if !*is_async || *is_generator {
                    return Err(diag("only async function declarations supported"));
                }
                let fn_name = self.emit_async_fn(params, body)?;
                self.async_fns.insert(*local, fn_name.clone());
                let t = self.fresh();
                writeln!(self.body, "  {t} = bitcast ptr @{fn_name} to ptr").ok();
                self.store_local(*local, &t)?;
                Ok(())
            }
            _ => Err(diag("unsupported statement")),
        }
    }

    pub(super) fn store_local(&mut self, id: LocalId, value_ptr_or_num: &str) -> Result<(), Diagnostic> {
        let Some(kind) = self.slot_kind(id) else {
            // Nested param / unknown — ignore stores to non-top-level when not in reaction.
            if let Some(ptr) = self.allocas.get(&id).cloned() {
                writeln!(self.body, "  store ptr {value_ptr_or_num}, ptr {ptr}").ok();
                return Ok(());
            }
            return Ok(());
        };
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("missing alloca"))?;
        match kind {
            SlotKind::Number => {
                let n = self.fresh();
                writeln!(self.body, "  {n} = ptrtoint ptr {value_ptr_or_num} to i64").ok();
                writeln!(self.body, "  store i64 {n}, ptr {ptr}").ok();
            }
            SlotKind::String | SlotKind::Object => {
                writeln!(self.body, "  store ptr {value_ptr_or_num}, ptr {ptr}").ok();
            }
        }
        Ok(())
    }

    pub(super) fn store_local_in(
        &mut self,
        buf: &mut String,
        id: LocalId,
        value_ptr: &str,
    ) -> Result<(), Diagnostic> {
        let Some(kind) = self.slot_kind(id) else {
            return Ok(());
        };
        // Capture allocas live in main; reaction env (`%data`) holds pointers to them.
        let slot = self.reaction_capture_slot(id, buf)?;
        match kind {
            SlotKind::Number => {
                let n = format!("%n{}", self.tmp);
                self.tmp += 1;
                writeln!(buf, "  {n} = ptrtoint ptr {value_ptr} to i64").ok();
                writeln!(buf, "  store i64 {n}, ptr {slot}").ok();
            }
            SlotKind::String | SlotKind::Object => {
                writeln!(buf, "  store ptr {value_ptr}, ptr {slot}").ok();
            }
        }
        Ok(())
    }

    /// Resolve the alloca ptr for a captured local inside a reaction (`%data` env).
    pub(super) fn reaction_capture_slot(
        &mut self,
        id: LocalId,
        buf: &mut String,
    ) -> Result<String, Diagnostic> {
        let Some(pos) = self.reaction_captures.iter().position(|c| *c == id) else {
            return Err(diag("capture not in reaction env"));
        };
        if self.reaction_captures.len() == 1 {
            return Ok("%data".into());
        }
        let gep = format!("%envp{}", self.tmp);
        self.tmp += 1;
        let slot = format!("%slot{}", self.tmp);
        self.tmp += 1;
        let n = self.reaction_captures.len();
        writeln!(
            buf,
            "  {gep} = getelementptr inbounds [{n} x ptr], ptr %data, i64 0, i64 {pos}"
        )
        .ok();
        writeln!(buf, "  {slot} = load ptr, ptr {gep}").ok();
        Ok(slot)
    }

    pub(super) fn slot_kind(&self, id: LocalId) -> Option<SlotKind> {
        self.info
            .user_locals
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, k)| *k)
    }

    pub(super) fn emit_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let n: i64 = parse_number(raw)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = inttoptr i64 {n} to ptr").ok();
                Ok(t)
            }
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            Expr::Local { id, .. } => self.load_local(*id),
            Expr::Unary { op, arg, .. } => match op {
                UnaryOp::TypeOf => self.emit_typeof(arg),
                UnaryOp::Minus => {
                    let a = self.emit_expr(arg)?;
                    let n = self.fresh();
                    let m = self.fresh();
                    let r = self.fresh();
                    writeln!(self.body, "  {n} = ptrtoint ptr {a} to i64").ok();
                    writeln!(self.body, "  {m} = sub i64 0, {n}").ok();
                    writeln!(self.body, "  {r} = inttoptr i64 {m} to ptr").ok();
                    Ok(r)
                }
                UnaryOp::Plus => self.emit_expr(arg),
                UnaryOp::Await => Err(diag("await only valid inside async function body")),
                _ => Err(diag("unsupported unary")),
            },
            Expr::Binary {
                left, op, right, ..
            } => {
                let l = self.emit_expr(left)?;
                let r = self.emit_expr(right)?;
                let ln = self.fresh();
                let rn = self.fresh();
                let out = self.fresh();
                let res = self.fresh();
                writeln!(self.body, "  {ln} = ptrtoint ptr {l} to i64").ok();
                writeln!(self.body, "  {rn} = ptrtoint ptr {r} to i64").ok();
                let inst = match op {
                    BinaryOp::Add => "add",
                    BinaryOp::Sub => "sub",
                    BinaryOp::Mul => "mul",
                    BinaryOp::Div => "sdiv",
                    BinaryOp::Rem => "srem",
                    _ => return Err(diag("unsupported binary")),
                };
                writeln!(self.body, "  {out} = {inst} i64 {ln}, {rn}").ok();
                writeln!(self.body, "  {res} = inttoptr i64 {out} to ptr").ok();
                Ok(res)
            }
            Expr::Assign { target, value, .. } => {
                let v = self.emit_expr(value)?;
                match target {
                    AssignTarget::Local(id) => {
                        self.store_local(*id, &v)?;
                        Ok(v)
                    }
                    _ => Err(diag("unsupported assign target")),
                }
            }
            Expr::New { callee, args, .. } => self.emit_new_promise(callee, args),
            Expr::Call { callee, args, .. } => self.emit_call(callee, args),
            Expr::Array { elements, .. } => self.emit_array(elements),
            Expr::Member {
                object,
                property,
                computed,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("optional member not supported"));
                }
                self.emit_member(object, property, *computed)
            }
            Expr::Function {
                name,
                params,
                body,
                is_async,
                is_generator,
                ..
            } => {
                if name.is_some() {
                    return Err(diag("named function expressions not supported"));
                }
                if *is_generator {
                    return Err(diag("generator functions not supported"));
                }
                if *is_async {
                    let fn_name = self.emit_async_fn(params, body)?;
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = bitcast ptr @{fn_name} to ptr").ok();
                    return Ok(t);
                }
                Err(diag("bare function expr not supported at value position"))
            }
            _ => Err(diag("unsupported expression")),
        }
    }

    pub(super) fn emit_array(&mut self, elements: &[ArrayElement]) -> Result<String, Diagnostic> {
        let n = elements.len();
        let arr = self.fresh();
        writeln!(
            self.body,
            "  {}",
            ARRAY_NEW.call_to(&arr, &format!("i64 {n}"))
        )
        .ok();
        for (i, el) in elements.iter().enumerate() {
            let ArrayElement::Expr(e) = el else {
                return Err(diag("spread array elements not supported"));
            };
            let v = self.emit_expr(e)?;
            writeln!(
                self.body,
                "  {}",
                ARRAY_SET.call(&format!("ptr {arr}, i64 {i}, ptr {v}"))
            )
            .ok();
        }
        Ok(arr)
    }

    pub(super) fn emit_member(
        &mut self,
        object: &Expr,
        property: &Expr,
        computed: bool,
    ) -> Result<String, Diagnostic> {
        if computed {
            let obj = self.emit_expr(object)?;
            let idx_ptr = self.emit_expr(property)?;
            let idx = self.fresh();
            let t = self.fresh();
            writeln!(self.body, "  {idx} = ptrtoint ptr {idx_ptr} to i64").ok();
            writeln!(
                self.body,
                "  {}",
                ARRAY_GET.call_to(&t, &format!("ptr {obj}, i64 {idx}"))
            )
            .ok();
            return Ok(t);
        }
        let Expr::String { value, .. } = property else {
            return Err(diag("only string property keys supported"));
        };
        let prop = value.to_string_lossy();
        if prop == "length" {
            let obj = self.emit_expr(object)?;
            let n = self.fresh();
            let t = self.fresh();
            writeln!(
                self.body,
                "  {}",
                ARRAY_LEN.call_to(&n, &format!("ptr {obj}"))
            )
            .ok();
            writeln!(self.body, "  {t} = inttoptr i64 {n} to ptr").ok();
            return Ok(t);
        }
        if prop == "status"
            || prop == "value"
            || prop == "reason"
            || prop == "name"
            || prop == "errors"
        {
            let obj = self.emit_expr(object)?;
            let key = self.string_const(&prop)?;
            let t = self.fresh();
            writeln!(
                self.body,
                "  {}",
                OBJECT_GET.call_to(&t, &format!("ptr {obj}, ptr {key}"))
            )
            .ok();
            return Ok(t);
        }
        Err(diag(format!("unsupported member `{}`", prop)))
    }

    pub(super) fn load_local(&mut self, id: LocalId) -> Result<String, Diagnostic> {
        if let Some((settle, cap)) = self.executor_params.get(&id).cloned() {
            // Calling resolve/reject is handled in emit_call; loading as value is unsupported.
            let _ = (settle, cap);
            return Err(diag("cannot use resolve/reject as values"));
        }
        if let Some(v) = self.reaction_params.get(&id).cloned() {
            return Ok(v);
        }
        let Some(kind) = self.slot_kind(id) else {
            // Unknown local (e.g. unused) → null
            let t = self.fresh();
            writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
            return Ok(t);
        };
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("missing alloca"))?;
        match kind {
            SlotKind::Number => {
                let n = self.fresh();
                let t = self.fresh();
                writeln!(self.body, "  {n} = load i64, ptr {ptr}").ok();
                writeln!(self.body, "  {t} = inttoptr i64 {n} to ptr").ok();
                Ok(t)
            }
            SlotKind::String | SlotKind::Object => {
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
        }
    }

    pub(super) fn emit_typeof(&mut self, arg: &Expr) -> Result<String, Diagnostic> {
        // typeof Promise → "function"
        if let Expr::Local { id, .. } = arg {
            if Some(*id) == self.info.promise_id {
                return self.string_const("function");
            }
        }
        // typeof Promise.resolve / Promise.reject / p.then|catch|finally → "function"
        if let Expr::Member {
            object,
            property,
            computed,
            optional,
            ..
        } = arg
        {
            if !*optional && !*computed {
                if let Expr::String { value, .. } = property.as_ref() {
                    let prop = value.to_string_lossy();
                    match prop.as_ref() {
                        "resolve" | "reject" | "all" | "race" | "allSettled" | "any" => {
                            if let Expr::Local { id, .. } = object.as_ref() {
                                if Some(*id) == self.info.promise_id {
                                    return self.string_const("function");
                                }
                            }
                        }
                        "then" | "catch" | "finally" => {
                            let _ = self.emit_expr(object)?;
                            return self.string_const("function");
                        }
                        _ => {}
                    }
                }
            }
        }
        // typeof promise object → "object"
        let _ = self.emit_expr(arg)?;
        self.string_const("object")
    }

    pub(super) fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let gname = if let Some(g) = self.str_globals.get(s) {
            g.clone()
        } else {
            let g = format!(".str.{}", self.str_globals.len());
            self.str_globals.insert(s.to_string(), g.clone());
            g
        };
        let t = self.fresh();
        let n = s.len() + 1;
        writeln!(
            self.body,
            "  {t} = getelementptr inbounds [{n} x i8], ptr @{gname}, i64 0, i64 0"
        )
        .ok();
        Ok(t)
    }

    pub(super) fn emit_new_promise(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = callee else {
            return Err(diag("new callee must be Promise"));
        };
        if Some(*id) != self.info.promise_id {
            return Err(diag("only new Promise supported"));
        }
        if args.len() != 1 {
            return Err(diag("Promise constructor expects 1 argument"));
        }
        let Arg::Expr(exec_expr) = &args[0] else {
            return Err(diag("spread not supported"));
        };
        let Expr::Function { params, body, .. } = exec_expr else {
            return Err(diag("Promise executor must be a function expression"));
        };
        let fn_name = self.emit_executor_fn(params, body)?;
        let t = self.fresh();
        writeln!(
            self.body,
            "  {}",
            PROMISE_CONSTRUCT.call_to(&t, &format!("ptr @{fn_name}, ptr null"))
        )
        .ok();
        Ok(t)
    }

    pub(super) fn emit_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        // resolve(value) / reject(reason) inside executor
        if let Expr::Local { id, .. } = callee {
            if let Some((settle, cap)) = self.executor_params.get(id).cloned() {
                if args.len() != 1 {
                    return Err(diag("resolve/reject expect 1 arg"));
                }
                let Arg::Expr(vexpr) = &args[0] else {
                    return Err(diag("spread not supported"));
                };
                let v = self.emit_expr(vexpr)?;
                writeln!(self.body, "  call void {settle}(ptr {cap}, ptr {v})").ok();
                let t = self.fresh();
                writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
                return Ok(t);
            }
            // User async function call (0+ simple args; N06.10–N06.11).
            let mut arg_ssas = Vec::with_capacity(args.len());
            for a in args {
                let Arg::Expr(e) = a else {
                    return Err(diag("spread args not supported"));
                };
                arg_ssas.push(self.emit_expr(e)?);
            }
            let arg_list = format_ptr_args(&arg_ssas);
            if let Some(name) = self.async_fns.get(id).cloned() {
                let t = self.fresh();
                writeln!(self.body, "  {t} = call ptr @{name}({arg_list})").ok();
                return Ok(t);
            }
            if matches!(self.slot_kind(*id), Some(SlotKind::Object)) {
                let fp = self.load_local(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = call ptr {fp}({arg_list})").ok();
                return Ok(t);
            }
        }

        // Promise.resolve(v) / Promise.reject(v) / promise.then / promise.catch
        if let Expr::Member {
            object,
            property,
            computed,
            optional,
            ..
        } = callee
        {
            if *optional || *computed {
                return Err(diag("unsupported member call"));
            }
            let Expr::String { value, .. } = property.as_ref() else {
                return Err(diag("only string property calls supported"));
            };
            let prop = value.to_string_lossy();
            match prop.as_ref() {
                "resolve" | "reject" => {
                    return self.emit_promise_static(object, prop.as_ref(), args);
                }
                "all" => {
                    return self.emit_promise_all(object, args);
                }
                "race" => {
                    return self.emit_promise_race(object, args);
                }
                "allSettled" => {
                    return self.emit_promise_all_settled(object, args);
                }
                "any" => {
                    return self.emit_promise_any(object, args);
                }
                "then" => {
                    let p = self.emit_expr(object)?;
                    let mut on_ful = "null".to_string();
                    let mut ful_data = "null".to_string();
                    let mut on_rej = "null".to_string();
                    let mut rej_data = "null".to_string();
                    if let Some(Arg::Expr(f0)) = args.first() {
                        if let Expr::Function { params, body, .. } = f0 {
                            let (name, data) = self.emit_reaction_fn(params, body)?;
                            on_ful = format!("@{name}");
                            ful_data = data;
                        } else {
                            return Err(diag("then callback must be function expression"));
                        }
                    }
                    if let Some(Arg::Expr(f1)) = args.get(1) {
                        if let Expr::Function { params, body, .. } = f1 {
                            let (name, data) = self.emit_reaction_fn(params, body)?;
                            on_rej = format!("@{name}");
                            rej_data = data;
                        } else {
                            return Err(diag("then callback must be function expression"));
                        }
                    }
                    let t = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        PROMISE_THEN.call_to(
                            &t,
                            &format!(
                                "ptr {p}, ptr {on_ful}, ptr {ful_data}, ptr {on_rej}, ptr {rej_data}"
                            )
                        )
                    )
                    .ok();
                    return Ok(t);
                }
                "catch" => {
                    // p.catch(onRejected) ≡ p.then(undefined, onRejected)
                    if args.len() != 1 {
                        return Err(diag("catch expects 1 argument"));
                    }
                    let p = self.emit_expr(object)?;
                    let Arg::Expr(f0) = &args[0] else {
                        return Err(diag("spread not supported"));
                    };
                    let Expr::Function { params, body, .. } = f0 else {
                        return Err(diag("catch callback must be function expression"));
                    };
                    let (name, data) = self.emit_reaction_fn(params, body)?;
                    let t = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        PROMISE_THEN.call_to(
                            &t,
                            &format!("ptr {p}, ptr null, ptr null, ptr @{name}, ptr {data}")
                        )
                    )
                    .ok();
                    return Ok(t);
                }
                "finally" => {
                    if args.len() != 1 {
                        return Err(diag("finally expects 1 argument"));
                    }
                    let p = self.emit_expr(object)?;
                    let Arg::Expr(f0) = &args[0] else {
                        return Err(diag("spread not supported"));
                    };
                    let Expr::Function { params, body, .. } = f0 else {
                        return Err(diag("finally callback must be function expression"));
                    };
                    let (name, data) = self.emit_reaction_fn(params, body)?;
                    let t = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        PROMISE_FINALLY.call_to(&t, &format!("ptr {p}, ptr @{name}, ptr {data}"))
                    )
                    .ok();
                    return Ok(t);
                }
                _ => return Err(diag(format!("unsupported method `{prop}`"))),
            }
        }

        Err(diag("unsupported call"))
    }

    pub(super) fn emit_promise_static(
        &mut self,
        object: &Expr,
        which: &str,
        args: &[Arg],
    ) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = object else {
            return Err(diag("static Promise methods require Promise receiver"));
        };
        if Some(*id) != self.info.promise_id {
            return Err(diag("only Promise.resolve / Promise.reject supported"));
        }
        if args.len() != 1 {
            return Err(diag(format!("Promise.{which} expects 1 argument")));
        }
        let Arg::Expr(vexpr) = &args[0] else {
            return Err(diag("spread not supported"));
        };
        let v = self.emit_expr(vexpr)?;
        let p = self.fresh();
        writeln!(self.body, "  {}", PROMISE_NEW.call_to(&p, "")).ok();
        match which {
            "resolve" => {
                writeln!(
                    self.body,
                    "  {}",
                    PROMISE_RESOLVE.call(&format!("ptr {p}, ptr {v}"))
                )
                .ok();
            }
            "reject" => {
                writeln!(
                    self.body,
                    "  {}",
                    PROMISE_REJECT.call(&format!("ptr {p}, ptr {v}"))
                )
                .ok();
            }
            _ => return Err(diag("internal: bad Promise static")),
        }
        Ok(p)
    }

    pub(super) fn emit_promise_all(&mut self, object: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = object else {
            return Err(diag("Promise.all requires Promise receiver"));
        };
        if Some(*id) != self.info.promise_id {
            return Err(diag("only Promise.all supported"));
        }
        if args.len() != 1 {
            return Err(diag("Promise.all expects 1 argument"));
        }
        let Arg::Expr(vexpr) = &args[0] else {
            return Err(diag("spread not supported"));
        };
        let arr = self.emit_expr(vexpr)?;
        let t = self.fresh();
        writeln!(
            self.body,
            "  {}",
            PROMISE_ALL.call_to(&t, &format!("ptr {arr}"))
        )
        .ok();
        Ok(t)
    }

    pub(super) fn emit_promise_race(&mut self, object: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = object else {
            return Err(diag("Promise.race requires Promise receiver"));
        };
        if Some(*id) != self.info.promise_id {
            return Err(diag("only Promise.race supported"));
        }
        if args.len() != 1 {
            return Err(diag("Promise.race expects 1 argument"));
        }
        let Arg::Expr(vexpr) = &args[0] else {
            return Err(diag("spread not supported"));
        };
        let arr = self.emit_expr(vexpr)?;
        let t = self.fresh();
        writeln!(
            self.body,
            "  {}",
            PROMISE_RACE.call_to(&t, &format!("ptr {arr}"))
        )
        .ok();
        Ok(t)
    }

    pub(super) fn emit_promise_all_settled(
        &mut self,
        object: &Expr,
        args: &[Arg],
    ) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = object else {
            return Err(diag("Promise.allSettled requires Promise receiver"));
        };
        if Some(*id) != self.info.promise_id {
            return Err(diag("only Promise.allSettled supported"));
        }
        if args.len() != 1 {
            return Err(diag("Promise.allSettled expects 1 argument"));
        }
        let Arg::Expr(vexpr) = &args[0] else {
            return Err(diag("spread not supported"));
        };
        let arr = self.emit_expr(vexpr)?;
        let t = self.fresh();
        writeln!(
            self.body,
            "  {}",
            PROMISE_ALL_SETTLED.call_to(&t, &format!("ptr {arr}"))
        )
        .ok();
        Ok(t)
    }

    pub(super) fn emit_promise_any(&mut self, object: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = object else {
            return Err(diag("Promise.any requires Promise receiver"));
        };
        if Some(*id) != self.info.promise_id {
            return Err(diag("only Promise.any supported"));
        }
        if args.len() != 1 {
            return Err(diag("Promise.any expects 1 argument"));
        }
        let Arg::Expr(vexpr) = &args[0] else {
            return Err(diag("spread not supported"));
        };
        let arr = self.emit_expr(vexpr)?;
        let t = self.fresh();
        writeln!(
            self.body,
            "  {}",
            PROMISE_ANY.call_to(&t, &format!("ptr {arr}"))
        )
        .ok();
        Ok(t)
    }
}
