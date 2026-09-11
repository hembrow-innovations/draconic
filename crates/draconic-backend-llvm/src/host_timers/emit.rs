use std::fmt::Write as _;

use super::classify::{is_clock_callee, is_named_callee, is_timer_api_name};
use super::*;
use crate::emitter::escape_llvm_string;

impl<'a> super::Emitter<'a> {
    pub(super) fn new(module: &'a Module, info: ModuleInfo) -> Self {
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            helpers: String::new(),
            tmp: 0,
            next_fn: 0,
            allocas: HashMap::new(),
            str_globals: Vec::new(),
            reaction_captures: Vec::new(),
            in_callback: false,
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

    pub(super) fn slot_kind(&self, id: LocalId) -> Option<SlotKind> {
        self.info
            .user_locals
            .iter()
            .find(|(l, _)| *l == id)
            .map(|(_, k)| *k)
    }

    pub(super) fn intern_cstr(&mut self, s: &str) -> String {
        if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            return g.clone();
        }
        let g = format!(".str.{}", self.str_globals.len());
        self.str_globals.push((s.to_string(), g.clone()));
        g
    }

    pub(super) fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM host_timers (H05 clocks + H05.03–H05.05 timers)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(HOST_TIMER_DECLARES)).ok();
        writeln!(
            self.out,
            "{}",
            llvm_declares(&[HOST_NOW_MS, HOST_MONOTONIC_MS])
        )
        .ok();
        writeln!(self.out).ok();

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
                SlotKind::Bool => {
                    writeln!(self.body, "  {ptr} = alloca i8, align 1").ok();
                    writeln!(self.body, "  store i8 0, ptr {ptr}").ok();
                }
                SlotKind::String => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        writeln!(self.body, "  {}", JOB_DRAIN.call("")).ok();

        for (id, kind) in self.info.user_locals.clone() {
            let ptr = self.allocas.get(&id).cloned().unwrap();
            match kind {
                SlotKind::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i64, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_I64.call(&format!("i64 {v}"))).ok();
                }
                SlotKind::Bool => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {v}"))).ok();
                }
                SlotKind::String => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
            }
        }

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
            Stmt::Block { body } => {
                for s in body {
                    self.emit_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Return { value } => {
                if let Some(e) = value {
                    let _ = self.emit_expr(e)?;
                }
                Ok(())
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => self.emit_if(test, consequent, alternate.as_deref()),
            _ => Err(diag("host_timer: unsupported stmt")),
        }
    }

    pub(super) fn store_local(&mut self, id: LocalId, val: &str) -> Result<(), Diagnostic> {
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("host_timer: unknown local"))?;
        match self.slot_kind(id).unwrap_or(SlotKind::Number) {
            SlotKind::Number => {
                writeln!(self.body, "  store i64 {val}, ptr {ptr}").ok();
            }
            SlotKind::Bool => {
                writeln!(self.body, "  store i8 {val}, ptr {ptr}").ok();
            }
            SlotKind::String => {
                writeln!(self.body, "  store ptr {val}, ptr {ptr}").ok();
            }
        }
        Ok(())
    }

    pub(super) fn load_local(&mut self, id: LocalId) -> Result<String, Diagnostic> {
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("host_timer: unknown local"))?;
        let v = self.fresh();
        match self.slot_kind(id).unwrap_or(SlotKind::Number) {
            SlotKind::Number => {
                writeln!(self.body, "  {v} = load i64, ptr {ptr}").ok();
            }
            SlotKind::Bool => {
                writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
            }
            SlotKind::String => {
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
            }
        }
        Ok(v)
    }

    pub(super) fn emit_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let n: i64 = raw.parse::<f64>().unwrap_or(0.0) as i64;
                Ok(format!("{n}"))
            }
            Expr::Boolean { value, .. } => Ok(if *value { "1".into() } else { "0".into() }),
            Expr::Local { id, .. } => {
                if self.in_callback {
                    if let Some(pos) = self.reaction_captures.iter().position(|c| c == id) {
                        return self.load_capture(pos);
                    }
                }
                self.load_local(*id)
            }
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } => self.emit_typeof(arg),
            Expr::Binary {
                op, left, right, ..
            } => self.emit_binary(*op, left, right),
            Expr::Call { callee, args, .. } if is_named_callee(callee, "setTimeout") => {
                if self.in_callback {
                    self.emit_timer_set_in_callback(args, false)
                } else {
                    self.emit_timer_set(args, false)
                }
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "setInterval") => {
                if self.in_callback {
                    self.emit_timer_set_in_callback(args, true)
                } else {
                    self.emit_timer_set(args, true)
                }
            }
            Expr::Call { callee, args, .. }
                if is_named_callee(callee, "clearTimeout")
                    || is_named_callee(callee, "clearInterval") =>
            {
                self.emit_clear_timeout(args)
            }
            Expr::Call { callee, args, .. } if args.is_empty() && is_clock_callee(callee) => {
                self.emit_clock_ms(callee)
            }
            Expr::Assign {
                target: AssignTarget::Local(id),
                value,
                ..
            } => {
                let v = self.emit_expr(value)?;
                if self.in_callback {
                    self.store_capture(*id, &v)?;
                } else {
                    self.store_local(*id, &v)?;
                }
                Ok(v)
            }
            Expr::IdentName { name, .. } if is_timer_api_name(name) => {
                // Only valid under typeof — return dummy.
                Ok("null".into())
            }
            _ => Err(diag("host_timer: unsupported expr")),
        }
    }

    pub(super) fn emit_if(
        &mut self,
        test: &Expr,
        consequent: &Stmt,
        alternate: Option<&Stmt>,
    ) -> Result<(), Diagnostic> {
        let cond = self.emit_expr(test)?;
        // cond may be i8 bool or i1 from icmp — normalize to i1
        let cond_i1 = self.fresh();
        writeln!(self.body, "  {cond_i1} = icmp ne i8 {cond}, 0").ok();
        let then_l = format!("if.then.{}", self.tmp);
        let else_l = format!("if.else.{}", self.tmp);
        let end_l = format!("if.end.{}", self.tmp);
        self.tmp += 1;
        if alternate.is_some() {
            writeln!(
                self.body,
                "  br i1 {cond_i1}, label %{then_l}, label %{else_l}"
            )
            .ok();
        } else {
            writeln!(
                self.body,
                "  br i1 {cond_i1}, label %{then_l}, label %{end_l}"
            )
            .ok();
        }
        writeln!(self.body, "{then_l}:").ok();
        self.emit_stmt(consequent)?;
        writeln!(self.body, "  br label %{end_l}").ok();
        if let Some(alt) = alternate {
            writeln!(self.body, "{else_l}:").ok();
            self.emit_stmt(alt)?;
            writeln!(self.body, "  br label %{end_l}").ok();
        }
        writeln!(self.body, "{end_l}:").ok();
        Ok(())
    }

    pub(super) fn load_capture(&mut self, pos: usize) -> Result<String, Diagnostic> {
        let n = self.reaction_captures.len();
        let slot = self.fresh();
        let ptr = self.fresh();
        let v = self.fresh();
        if n == 1 {
            // data is the alloca ptr directly
            let id = self.reaction_captures[0];
            match self.slot_kind(id).unwrap_or(SlotKind::Number) {
                SlotKind::Number => {
                    writeln!(self.body, "  {v} = load i64, ptr %data").ok();
                }
                SlotKind::Bool => {
                    writeln!(self.body, "  {v} = load i8, ptr %data").ok();
                }
                SlotKind::String => {
                    writeln!(self.body, "  {v} = load ptr, ptr %data").ok();
                }
            }
            return Ok(v);
        }
        writeln!(
            self.body,
            "  {slot} = getelementptr inbounds [{n} x ptr], ptr %data, i64 0, i64 {pos}"
        )
        .ok();
        writeln!(self.body, "  {ptr} = load ptr, ptr {slot}").ok();
        let id = self.reaction_captures[pos];
        match self.slot_kind(id).unwrap_or(SlotKind::Number) {
            SlotKind::Number => {
                writeln!(self.body, "  {v} = load i64, ptr {ptr}").ok();
            }
            SlotKind::Bool => {
                writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
            }
            SlotKind::String => {
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
            }
        }
        Ok(v)
    }

    pub(super) fn store_capture(&mut self, id: LocalId, val: &str) -> Result<(), Diagnostic> {
        let pos = self
            .reaction_captures
            .iter()
            .position(|c| *c == id)
            .ok_or_else(|| diag("host_timer: capture missing"))?;
        let n = self.reaction_captures.len();
        if n == 1 {
            match self.slot_kind(id).unwrap_or(SlotKind::Number) {
                SlotKind::Number => {
                    writeln!(self.body, "  store i64 {val}, ptr %data").ok();
                }
                SlotKind::Bool => {
                    writeln!(self.body, "  store i8 {val}, ptr %data").ok();
                }
                SlotKind::String => {
                    writeln!(self.body, "  store ptr {val}, ptr %data").ok();
                }
            }
            return Ok(());
        }
        let slot = self.fresh();
        let ptr = self.fresh();
        writeln!(
            self.body,
            "  {slot} = getelementptr inbounds [{n} x ptr], ptr %data, i64 0, i64 {pos}"
        )
        .ok();
        writeln!(self.body, "  {ptr} = load ptr, ptr {slot}").ok();
        match self.slot_kind(id).unwrap_or(SlotKind::Number) {
            SlotKind::Number => {
                writeln!(self.body, "  store i64 {val}, ptr {ptr}").ok();
            }
            SlotKind::Bool => {
                writeln!(self.body, "  store i8 {val}, ptr {ptr}").ok();
            }
            SlotKind::String => {
                writeln!(self.body, "  store ptr {val}, ptr {ptr}").ok();
            }
        }
        Ok(())
    }

    pub(super) fn emit_clock_ms(&mut self, callee: &Expr) -> Result<String, Diagnostic> {
        let d = self.fresh();
        let abi = if is_named_callee(callee, "monotonicMs") {
            HOST_MONOTONIC_MS
        } else {
            HOST_NOW_MS
        };
        writeln!(self.body, "  {}", abi.call_to(&d, "")).ok();
        let i = self.fresh();
        writeln!(self.body, "  {i} = fptosi double {d} to i64").ok();
        Ok(i)
    }

    pub(super) fn emit_typeof(&mut self, arg: &Expr) -> Result<String, Diagnostic> {
        let s = match arg {
            Expr::IdentName { name, .. } if is_timer_api_name(name) => "function",
            Expr::Call { callee, args, .. } if args.is_empty() && is_clock_callee(callee) => {
                "number"
            }
            _ => {
                return Err(diag(
                    "host_timer: typeof only on timer host APIs or clock calls",
                ))
            }
        };
        let g = self.intern_cstr(s);
        let n = s.len() + 1;
        let p = self.fresh();
        writeln!(
            self.body,
            "  {p} = getelementptr inbounds [{n} x i8], ptr @{g}, i64 0, i64 0"
        )
        .ok();
        Ok(p)
    }

    pub(super) fn emit_binary(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<String, Diagnostic> {
        let l = self.emit_expr(left)?;
        let r = self.emit_expr(right)?;
        let v = self.fresh();
        match op {
            BinaryOp::Gt => {
                writeln!(self.body, "  {v} = icmp sgt i64 {l}, {r}").ok();
                let b = self.fresh();
                writeln!(self.body, "  {b} = zext i1 {v} to i8").ok();
                Ok(b)
            }
            BinaryOp::GtEq => {
                writeln!(self.body, "  {v} = icmp sge i64 {l}, {r}").ok();
                let b = self.fresh();
                writeln!(self.body, "  {b} = zext i1 {v} to i8").ok();
                Ok(b)
            }
            BinaryOp::Lt => {
                writeln!(self.body, "  {v} = icmp slt i64 {l}, {r}").ok();
                let b = self.fresh();
                writeln!(self.body, "  {b} = zext i1 {v} to i8").ok();
                Ok(b)
            }
            BinaryOp::LtEq => {
                writeln!(self.body, "  {v} = icmp sle i64 {l}, {r}").ok();
                let b = self.fresh();
                writeln!(self.body, "  {b} = zext i1 {v} to i8").ok();
                Ok(b)
            }
            BinaryOp::Add => {
                writeln!(self.body, "  {v} = add i64 {l}, {r}").ok();
                Ok(v)
            }
            BinaryOp::Sub => {
                writeln!(self.body, "  {v} = sub i64 {l}, {r}").ok();
                Ok(v)
            }
            _ => Err(diag("host_timer: unsupported binary")),
        }
    }

    pub(super) fn emit_timer_set(
        &mut self,
        args: &[Arg],
        repeating: bool,
    ) -> Result<String, Diagnostic> {
        let fn_expr = match &args[0] {
            Arg::Expr(e) => e,
            _ => return Err(diag("timer set bad arg")),
        };
        let delay_expr = match &args[1] {
            Arg::Expr(e) => e,
            _ => return Err(diag("timer set bad delay")),
        };
        let Expr::Function { params, body, .. } = fn_expr else {
            return Err(diag("timer set needs function"));
        };
        let _ = params;
        let (fn_name, data_op) = self.emit_timer_callback(body)?;
        let delay_i = self.emit_expr(delay_expr)?;
        let delay_d = self.fresh();
        writeln!(self.body, "  {delay_d} = sitofp i64 {delay_i} to double").ok();
        let id = self.fresh();
        let abi = if repeating {
            TIMER_SET_INTERVAL
        } else {
            TIMER_SET
        };
        writeln!(
            self.body,
            "  {}",
            abi.call_to(
                &id,
                &format!("ptr @{fn_name}, ptr {data_op}, double {delay_d}")
            )
        )
        .ok();
        Ok(id)
    }

    pub(super) fn emit_clear_timeout(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let id_expr = match &args[0] {
            Arg::Expr(e) => e,
            _ => return Err(diag("clearTimeout bad arg")),
        };
        let id = self.emit_expr(id_expr)?;
        writeln!(self.body, "  {}", TIMER_CLEAR.call(&format!("i64 {id}"))).ok();
        Ok("0".into())
    }

    pub(super) fn emit_timer_callback(
        &mut self,
        body: &[Stmt],
    ) -> Result<(String, String), Diagnostic> {
        let fn_name = self.fresh_fn("timer");
        let mut used = HashSet::new();
        collect_used_locals(body, &mut used);
        let mut captures: Vec<LocalId> = used
            .into_iter()
            .filter(|id| {
                matches!(
                    self.slot_kind(*id),
                    Some(SlotKind::Number) | Some(SlotKind::Bool) | Some(SlotKind::String)
                )
            })
            .collect();
        captures.sort_by_key(|id| id.0);

        let data_operand = if captures.is_empty() {
            "null".to_string()
        } else if captures.len() == 1 {
            self.allocas
                .get(&captures[0])
                .cloned()
                .ok_or_else(|| diag("capture missing alloca"))?
        } else {
            let n = captures.len();
            let env = self.fresh();
            writeln!(self.body, "  {env} = alloca [{n} x ptr], align 8").ok();
            for (i, id) in captures.iter().enumerate() {
                let ptr = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("capture missing alloca"))?;
                let slot = self.fresh();
                writeln!(
                    self.body,
                    "  {slot} = getelementptr inbounds [{n} x ptr], ptr {env}, i64 0, i64 {i}"
                )
                .ok();
                writeln!(self.body, "  store ptr {ptr}, ptr {slot}").ok();
            }
            env
        };

        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_caps = std::mem::take(&mut self.reaction_captures);
        let saved_in = self.in_callback;

        self.tmp = 0;
        self.body.clear();
        self.reaction_captures = captures;
        self.in_callback = true;

        for stmt in body {
            self.emit_callback_stmt(stmt)?;
        }

        let mut fn_ir = String::new();
        writeln!(fn_ir, "define void @{fn_name}(ptr %data) {{").ok();
        writeln!(fn_ir, "entry:").ok();
        fn_ir.push_str(&self.body);
        writeln!(fn_ir, "  ret void").ok();
        writeln!(fn_ir, "}}").ok();
        self.helpers.push_str(&fn_ir);
        self.helpers.push('\n');

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.reaction_captures = saved_caps;
        self.in_callback = saved_in;
        Ok((fn_name, data_operand))
    }

    pub(super) fn emit_callback_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Expr { expr } => {
                let _ = self.emit_expr(expr)?;
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_callback_stmt(s)?;
                }
                Ok(())
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => self.emit_if(test, consequent, alternate.as_deref()),
            Stmt::Return { value } => {
                if let Some(e) = value {
                    let _ = self.emit_expr(e)?;
                }
                Ok(())
            }
            _ => Err(diag("host_timer cb: unsupported stmt")),
        }
    }

    pub(super) fn emit_timer_set_in_callback(
        &mut self,
        args: &[Arg],
        repeating: bool,
    ) -> Result<String, Diagnostic> {
        // Nested timer: build another helper; captures are top-level allocas.
        // job_drain runs before main returns, so main stack allocas stay valid.

        let fn_expr = match &args[0] {
            Arg::Expr(e) => e,
            _ => return Err(diag("nested timer bad arg")),
        };
        let delay_expr = match &args[1] {
            Arg::Expr(e) => e,
            _ => return Err(diag("nested timer bad delay")),
        };
        let Expr::Function { body, .. } = fn_expr else {
            return Err(diag("nested timer needs function"));
        };

        let mut used = HashSet::new();
        collect_used_locals(body, &mut used);
        let mut captures: Vec<LocalId> = used
            .into_iter()
            .filter(|id| {
                matches!(
                    self.slot_kind(*id),
                    Some(SlotKind::Number) | Some(SlotKind::Bool) | Some(SlotKind::String)
                )
            })
            .collect();
        captures.sort_by_key(|id| id.0);

        // Require nested captures ⊆ outer captures; pass through %data when equal.
        if captures != self.reaction_captures && !captures.is_empty() {
            for c in &captures {
                if !self.reaction_captures.contains(c) {
                    return Err(diag("nested timer capture not in outer env"));
                }
            }
        }

        let data_op = if captures.is_empty() {
            "null".to_string()
        } else if captures == self.reaction_captures {
            "%data".to_string()
        } else if self.reaction_captures.len() == 1 && captures.len() == 1 {
            "%data".to_string()
        } else {
            // Build env of pointers loaded from outer env.
            let n = captures.len();
            let env = self.fresh();
            writeln!(self.body, "  {env} = alloca [{n} x ptr], align 8").ok();
            for (i, id) in captures.iter().enumerate() {
                let outer_pos = self
                    .reaction_captures
                    .iter()
                    .position(|c| c == id)
                    .ok_or_else(|| diag("nested capture missing"))?;
                let ptr = if self.reaction_captures.len() == 1 {
                    "%data".to_string()
                } else {
                    let on = self.reaction_captures.len();
                    let slot = self.fresh();
                    let p = self.fresh();
                    writeln!(
                        self.body,
                        "  {slot} = getelementptr inbounds [{on} x ptr], ptr %data, i64 0, i64 {outer_pos}"
                    )
                    .ok();
                    writeln!(self.body, "  {p} = load ptr, ptr {slot}").ok();
                    p
                };
                let slot = self.fresh();
                writeln!(
                    self.body,
                    "  {slot} = getelementptr inbounds [{n} x ptr], ptr {env}, i64 0, i64 {i}"
                )
                .ok();
                writeln!(self.body, "  store ptr {ptr}, ptr {slot}").ok();
            }
            env
        };

        let fn_name = self.fresh_fn("timer");
        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_caps = std::mem::take(&mut self.reaction_captures);

        self.tmp = 0;
        self.body.clear();
        self.reaction_captures = captures;
        self.in_callback = true;

        for stmt in body {
            self.emit_callback_stmt(stmt)?;
        }

        let mut fn_ir = String::new();
        writeln!(fn_ir, "define void @{fn_name}(ptr %data) {{").ok();
        writeln!(fn_ir, "entry:").ok();
        fn_ir.push_str(&self.body);
        writeln!(fn_ir, "  ret void").ok();
        writeln!(fn_ir, "}}").ok();
        self.helpers.push_str(&fn_ir);
        self.helpers.push('\n');

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.reaction_captures = saved_caps;

        let delay_i = self.emit_expr(delay_expr)?;
        let delay_d = self.fresh();
        writeln!(self.body, "  {delay_d} = sitofp i64 {delay_i} to double").ok();
        let id = self.fresh();
        let abi = if repeating {
            TIMER_SET_INTERVAL
        } else {
            TIMER_SET
        };
        writeln!(
            self.body,
            "  {}",
            abi.call_to(
                &id,
                &format!("ptr @{fn_name}, ptr {data_op}, double {delay_d}")
            )
        )
        .ok();
        Ok(id)
    }
}

fn collect_used_locals(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Expr { expr } => collect_used_in_expr(expr, out),
            Stmt::Block { body } => collect_used_locals(body, out),
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                collect_used_in_expr(test, out);
                collect_used_locals(std::slice::from_ref(consequent.as_ref()), out);
                if let Some(alt) = alternate {
                    collect_used_locals(std::slice::from_ref(alt.as_ref()), out);
                }
            }
            Stmt::Return { value } => {
                if let Some(e) = value {
                    collect_used_in_expr(e, out);
                }
            }
            _ => {}
        }
    }
}

fn collect_used_in_expr(expr: &Expr, out: &mut HashSet<LocalId>) {
    match expr {
        Expr::Local { id, .. } => {
            out.insert(*id);
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            value,
            ..
        } => {
            out.insert(*id);
            collect_used_in_expr(value, out);
        }
        Expr::Call { callee, args, .. } => {
            if is_named_callee(callee, "setTimeout") || is_named_callee(callee, "setInterval") {
                if let Some(Arg::Expr(Expr::Function { body, .. })) = args.first() {
                    collect_used_locals(body, out);
                }
            }
            for a in args {
                if let Arg::Expr(e) = a {
                    collect_used_in_expr(e, out);
                }
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_used_in_expr(left, out);
            collect_used_in_expr(right, out);
        }
        Expr::Unary { arg, .. } => collect_used_in_expr(arg, out),
        _ => {}
    }
}
