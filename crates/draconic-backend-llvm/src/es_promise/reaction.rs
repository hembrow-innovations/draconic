use std::fmt::Write as _;

use super::*;

impl<'a> super::Emitter<'a> {
    pub(super) fn emit_executor_fn(
        &mut self,
        params: &[Param],
        body: &[Stmt],
    ) -> Result<String, Diagnostic> {
        let fn_name = self.fresh_fn("exec");
        let mut resolve_param = None;
        let mut reject_param = None;
        for (i, p) in params.iter().enumerate() {
            let Pattern::Local(id) = &p.pattern else {
                return Err(diag("bad param"));
            };
            if i == 0 {
                resolve_param = Some(*id);
            } else if i == 1 {
                reject_param = Some(*id);
            }
        }

        // Save main emission state
        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_exec = std::mem::take(&mut self.executor_params);
        let saved_react = std::mem::take(&mut self.reaction_params);
        let saved_caps = std::mem::take(&mut self.reaction_captures);

        self.tmp = 0;
        self.body.clear();
        self.executor_params.clear();
        if let Some(id) = resolve_param {
            self.executor_params
                .insert(id, ("%resolve".into(), "%resolve_cap".into()));
        }
        if let Some(id) = reject_param {
            self.executor_params
                .insert(id, ("%reject".into(), "%reject_cap".into()));
        }

        for stmt in body {
            match stmt {
                Stmt::Expr { expr } => {
                    let _ = self.emit_expr(expr)?;
                }
                Stmt::Return { value } => {
                    if let Some(e) = value {
                        let _ = self.emit_expr(e)?;
                    }
                }
                Stmt::Block { body } => {
                    for s in body {
                        if let Stmt::Expr { expr } = s {
                            let _ = self.emit_expr(expr)?;
                        } else if let Stmt::Return { value } = s {
                            if let Some(e) = value {
                                let _ = self.emit_expr(e)?;
                            }
                        } else {
                            return Err(diag("unsupported stmt in executor"));
                        }
                    }
                }
                _ => return Err(diag("unsupported stmt in executor")),
            }
        }

        let mut fn_ir = String::new();
        writeln!(
            fn_ir,
            "define void @{fn_name}(ptr %data, ptr %resolve, ptr %resolve_cap, ptr %reject, ptr %reject_cap) {{"
        )
        .ok();
        writeln!(fn_ir, "entry:").ok();
        writeln!(fn_ir, "  ; data unused").ok();
        fn_ir.push_str(&self.body);
        writeln!(fn_ir, "  ret void").ok();
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

    pub(super) fn emit_reaction_fn(
        &mut self,
        params: &[Param],
        body: &[Stmt],
    ) -> Result<(String, String), Diagnostic> {
        let fn_name = self.fresh_fn("react");
        let mut param_id = None;
        if let Some(p) = params.first() {
            let Pattern::Local(id) = &p.pattern else {
                return Err(diag("bad param"));
            };
            param_id = Some(*id);
        }

        // Find assigned top-level number/string locals (captures), stable order by id.
        let mut assigned = HashSet::new();
        collect_assigned_locals(body, &mut assigned);
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
            // Env: [N x ptr] of capture allocas, allocated in main.
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
        let saved_exec = std::mem::take(&mut self.executor_params);
        let saved_react = std::mem::take(&mut self.reaction_params);
        let saved_caps = std::mem::take(&mut self.reaction_captures);

        self.tmp = 0;
        self.body.clear();
        self.executor_params.clear();
        self.reaction_params.clear();
        if let Some(id) = param_id {
            self.reaction_params.insert(id, "%value".into());
        }
        self.reaction_captures = captures;

        let mut ret_val = "%value".to_string();
        for stmt in body {
            match stmt {
                Stmt::Expr { expr } => {
                    if let Expr::Assign {
                        target: AssignTarget::Local(id),
                        value,
                        ..
                    } = expr
                    {
                        let v = self.emit_expr_in_reaction(value)?;
                        // store into capture
                        let mut buf = std::mem::take(&mut self.body);
                        self.store_local_in(&mut buf, *id, &v)?;
                        self.body = buf;
                        ret_val = v;
                    } else {
                        let v = self.emit_expr_in_reaction(expr)?;
                        ret_val = v;
                    }
                }
                Stmt::Return { value } => {
                    if let Some(e) = value {
                        ret_val = self.emit_expr_in_reaction(e)?;
                    } else {
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
                        ret_val = t;
                    }
                }
                Stmt::Block { body: inner } => {
                    for s in inner {
                        match s {
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
                            Stmt::Return { value } => {
                                if let Some(e) = value {
                                    ret_val = self.emit_expr_in_reaction(e)?;
                                }
                            }
                            _ => return Err(diag("unsupported stmt in reaction")),
                        }
                    }
                }
                _ => return Err(diag("unsupported stmt in reaction")),
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

    /// Emit expression while building a reaction (writes into self.body).
    pub(super) fn emit_expr_in_reaction(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let n: i64 = parse_number(raw)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = inttoptr i64 {n} to ptr").ok();
                Ok(t)
            }
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            Expr::Local { id, .. } => {
                if let Some(v) = self.reaction_params.get(id).cloned() {
                    return Ok(v);
                }
                if self.reaction_captures.contains(id) {
                    let mut buf = String::new();
                    let slot = self.reaction_capture_slot(*id, &mut buf)?;
                    self.body.push_str(&buf);
                    match self.slot_kind(*id) {
                        Some(SlotKind::String) | Some(SlotKind::Object) => {
                            let t = self.fresh();
                            writeln!(self.body, "  {t} = load ptr, ptr {slot}").ok();
                            return Ok(t);
                        }
                        _ => {
                            let n = self.fresh();
                            let t = self.fresh();
                            writeln!(self.body, "  {n} = load i64, ptr {slot}").ok();
                            writeln!(self.body, "  {t} = inttoptr i64 {n} to ptr").ok();
                            return Ok(t);
                        }
                    }
                }
                let t = self.fresh();
                writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
                Ok(t)
            }
            Expr::Unary {
                op: UnaryOp::Minus,
                arg,
                ..
            } => {
                let a = self.emit_expr_in_reaction(arg)?;
                let n = self.fresh();
                let m = self.fresh();
                let r = self.fresh();
                writeln!(self.body, "  {n} = ptrtoint ptr {a} to i64").ok();
                writeln!(self.body, "  {m} = sub i64 0, {n}").ok();
                writeln!(self.body, "  {r} = inttoptr i64 {m} to ptr").ok();
                Ok(r)
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                let l = self.emit_expr_in_reaction(left)?;
                let r = self.emit_expr_in_reaction(right)?;
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
                    _ => return Err(diag("unsupported binary in reaction")),
                };
                writeln!(self.body, "  {out} = {inst} i64 {ln}, {rn}").ok();
                writeln!(self.body, "  {res} = inttoptr i64 {out} to ptr").ok();
                Ok(res)
            }
            Expr::Member {
                object,
                property,
                computed,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("optional member not supported in reaction"));
                }
                if *computed {
                    let obj = self.emit_expr_in_reaction(object)?;
                    let idx_ptr = self.emit_expr_in_reaction(property)?;
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
                let Expr::String { value, .. } = property.as_ref() else {
                    return Err(diag("only string property keys supported in reaction"));
                };
                let prop = value.to_string_lossy();
                if prop == "length" {
                    let obj = self.emit_expr_in_reaction(object)?;
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
                    let obj = self.emit_expr_in_reaction(object)?;
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
                Err(diag(format!("unsupported member `{}` in reaction", prop)))
            }
            _ => Err(diag("unsupported expr in reaction")),
        }
    }
}
