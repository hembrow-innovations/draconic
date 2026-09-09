use std::fmt::Write as _;

use super::*;

impl<'a> super::Emitter<'a> {
    /// Emit an async function/arrow: returns a Promise (N06.10–N06.11).
    /// Supports simple ident params (no rest/default); body may use `await` / `return` / `throw`.
    pub(super) fn emit_async_fn(&mut self, params: &[Param], body: &[Stmt]) -> Result<String, Diagnostic> {
        let mut param_ids = Vec::with_capacity(params.len());
        for p in params {
            if p.rest || p.default.is_some() {
                return Err(diag("rest/default params not supported on async"));
            }
            let Pattern::Local(id) = &p.pattern else {
                return Err(diag("only simple async params supported"));
            };
            param_ids.push(*id);
        }
        let fn_name = self.fresh_fn("async");

        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_exec = std::mem::take(&mut self.executor_params);
        let saved_react = std::mem::take(&mut self.reaction_params);
        let saved_caps = std::mem::take(&mut self.reaction_captures);

        self.tmp = 0;
        self.body.clear();
        self.executor_params.clear();
        self.reaction_params.clear();
        self.reaction_captures.clear();
        for (i, id) in param_ids.iter().enumerate() {
            self.reaction_params.insert(*id, format!("%arg{i}"));
        }

        let ret_promise = self.emit_async_body(body)?;

        let mut sig_params = String::new();
        for i in 0..param_ids.len() {
            if i > 0 {
                sig_params.push_str(", ");
            }
            write!(sig_params, "ptr %arg{i}").ok();
        }
        let mut fn_ir = String::new();
        writeln!(fn_ir, "define ptr @{fn_name}({sig_params}) {{").ok();
        writeln!(fn_ir, "entry:").ok();
        fn_ir.push_str(&self.body);
        writeln!(fn_ir, "  ret ptr {ret_promise}").ok();
        writeln!(fn_ir, "}}").ok();
        self.helpers.push_str(&fn_ir);
        self.helpers.push('\n');

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.executor_params = saved_exec;
        self.reaction_params = saved_react;
        self.reaction_captures = saved_caps;
        Ok(fn_name)
    }

    /// Lower async body into `self.body`; returns SSA of the result Promise.
    pub(super) fn emit_async_body(&mut self, body: &[Stmt]) -> Result<String, Diagnostic> {
        // Linear await: optional prefix without await, then `let x = await e`, then rest.
        for (i, stmt) in body.iter().enumerate() {
            if let Some((bind, await_arg)) = match_await_declare(stmt) {
                for s in &body[..i] {
                    if stmt_contains_await(s) {
                        return Err(diag("await only supported as `let x = await expr`"));
                    }
                    self.emit_async_sync_stmt(s)?;
                }
                let v = self.emit_expr(await_arg)?;
                let p = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    PROMISE_AWAIT.call_to(&p, &format!("ptr {v}"))
                )
                .ok();
                let rest = &body[i + 1..];
                let (cont, data) = self.emit_async_continuation(bind, rest)?;
                let out = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    PROMISE_THEN.call_to(
                        &out,
                        &format!("ptr {p}, ptr @{cont}, ptr {data}, ptr null, ptr null")
                    )
                )
                .ok();
                return Ok(out);
            }
            if stmt_contains_await(stmt) {
                return Err(diag("await only supported as `let x = await expr`"));
            }
        }
        self.emit_async_sync_body(body)
    }

    pub(super) fn emit_async_sync_body(&mut self, body: &[Stmt]) -> Result<String, Diagnostic> {
        let p = self.fresh();
        writeln!(self.body, "  {}", PROMISE_NEW.call_to(&p, "")).ok();
        for stmt in body {
            match stmt {
                Stmt::Return { value } => {
                    let v = if let Some(e) = value {
                        self.emit_expr(e)?
                    } else {
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
                        t
                    };
                    writeln!(
                        self.body,
                        "  {}",
                        PROMISE_RESOLVE.call(&format!("ptr {p}, ptr {v}"))
                    )
                    .ok();
                    return Ok(p);
                }
                Stmt::Throw { value } => {
                    let v = self.emit_expr(value)?;
                    writeln!(
                        self.body,
                        "  {}",
                        PROMISE_REJECT.call(&format!("ptr {p}, ptr {v}"))
                    )
                    .ok();
                    return Ok(p);
                }
                other => self.emit_async_sync_stmt(other)?,
            }
        }
        let u = self.fresh();
        writeln!(self.body, "  {u} = inttoptr i64 0 to ptr").ok();
        writeln!(
            self.body,
            "  {}",
            PROMISE_RESOLVE.call(&format!("ptr {p}, ptr {u}"))
        )
        .ok();
        Ok(p)
    }

    pub(super) fn emit_async_sync_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Expr { expr } => {
                let _ = self.emit_expr(expr)?;
                Ok(())
            }
            Stmt::Declare { local, init, .. } => {
                // Nested locals inside async without await: store via reaction_params map as SSA.
                if let Some(e) = init {
                    let v = self.emit_expr(e)?;
                    self.reaction_params.insert(*local, v);
                }
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_async_sync_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Return { .. } | Stmt::Throw { .. } => {
                Err(diag("return/throw must be handled by async body driver"))
            }
            _ => Err(diag("unsupported statement in async body")),
        }
    }

    /// Continuation after `let bind = await …`: reaction binds `%value` to `bind`, runs `rest`.
    pub(super) fn emit_async_continuation(
        &mut self,
        bind: LocalId,
        rest: &[Stmt],
    ) -> Result<(String, String), Diagnostic> {
        // Captures: top-level number/string locals assigned in rest.
        let mut assigned = HashSet::new();
        collect_assigned_locals(rest, &mut assigned);
        let mut captures: Vec<LocalId> = assigned
            .into_iter()
            .filter(|id| {
                matches!(
                    self.slot_kind(*id),
                    Some(SlotKind::Number) | Some(SlotKind::String)
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

        let fn_name = self.fresh_fn("async_cont");

        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_exec = std::mem::take(&mut self.executor_params);
        let saved_react = std::mem::take(&mut self.reaction_params);
        let saved_caps = std::mem::take(&mut self.reaction_captures);

        self.tmp = 0;
        self.body.clear();
        self.executor_params.clear();
        self.reaction_params.clear();
        self.reaction_params.insert(bind, "%value".into());
        self.reaction_captures = captures;

        // If rest has another await, nest; else evaluate returns as reaction return value.
        let mut ret_val = "%value".to_string();
        let mut i = 0;
        while i < rest.len() {
            let stmt = &rest[i];
            if let Some((next_bind, await_arg)) = match_await_declare(stmt) {
                let v = self.emit_expr_in_reaction(await_arg)?;
                let p = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    PROMISE_AWAIT.call_to(&p, &format!("ptr {v}"))
                )
                .ok();
                // Nested await: build another continuation for remaining rest and return that promise.
                // Note: reaction resolve does not assimilate thenables; nested await deferred.
                let nested_rest = &rest[i + 1..];
                // Re-enter via emit_async_continuation needs parent body for data — not supported.
                let _ = (next_bind, nested_rest);
                return Err(diag(
                    "multiple awaits in one async function not supported yet",
                ));
            }
            match stmt {
                Stmt::Return { value } => {
                    if let Some(e) = value {
                        ret_val = self.emit_expr_in_reaction(e)?;
                    } else {
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
                        ret_val = t;
                    }
                    i += 1;
                }
                Stmt::Throw { value } => {
                    // Reject by throwing is not available in reaction; unsupported.
                    let _ = value;
                    return Err(diag("throw after await not supported yet"));
                }
                Stmt::Expr { expr } => {
                    if let Expr::Assign {
                        target: AssignTarget::Local(id),
                        value,
                        ..
                    } = expr
                    {
                        let v = self.emit_expr_in_reaction(value)?;
                        let mut buf = std::mem::take(&mut self.body);
                        self.store_local_in(&mut buf, *id, &v)?;
                        self.body = buf;
                        ret_val = v;
                    } else {
                        ret_val = self.emit_expr_in_reaction(expr)?;
                    }
                    i += 1;
                }
                Stmt::Declare { local, init, .. } => {
                    if let Some(e) = init {
                        let v = self.emit_expr_in_reaction(e)?;
                        self.reaction_params.insert(*local, v.clone());
                        ret_val = v;
                    }
                    i += 1;
                }
                Stmt::Block { body: inner } => {
                    for s in inner {
                        match s {
                            Stmt::Return { value } => {
                                if let Some(e) = value {
                                    ret_val = self.emit_expr_in_reaction(e)?;
                                }
                            }
                            Stmt::Expr { expr } => {
                                if let Expr::Assign {
                                    target: AssignTarget::Local(id),
                                    value,
                                    ..
                                } = expr
                                {
                                    let v = self.emit_expr_in_reaction(value)?;
                                    let mut buf = std::mem::take(&mut self.body);
                                    self.store_local_in(&mut buf, *id, &v)?;
                                    self.body = buf;
                                    ret_val = v;
                                } else {
                                    ret_val = self.emit_expr_in_reaction(expr)?;
                                }
                            }
                            _ => return Err(diag("unsupported stmt in async continuation block")),
                        }
                    }
                    i += 1;
                }
                _ => return Err(diag("unsupported stmt in async continuation")),
            }
        }

        let mut fn_ir = String::new();
        writeln!(fn_ir, "define ptr @{fn_name}(ptr %data, ptr %value) {{").ok();
        writeln!(fn_ir, "entry:").ok();
        fn_ir.push_str(&self.body);
        writeln!(fn_ir, "  ret ptr {ret_val}").ok();
        writeln!(fn_ir, "}}").ok();
        self.helpers.push_str(&fn_ir);
        self.helpers.push('\n');

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.executor_params = saved_exec;
        self.reaction_params = saved_react;
        self.reaction_captures = saved_caps;

        Ok((fn_name, data_operand))
    }
}
