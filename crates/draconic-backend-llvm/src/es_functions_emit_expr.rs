use std::fmt::Write as _;

use super::ok::returned_fn_idx_in_body;
use super::*;

impl<'a> super::Emitter<'a> {
    pub(super) fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => Ok(format_number_const(raw)?),
            Expr::IdentName { name, .. } => {
                let id = self
                    .state
                    .allocas
                    .keys()
                    .copied()
                    .filter(|id| {
                        self.module
                            .locals
                            .iter()
                            .any(|l| l.id == *id && l.name == *name)
                    })
                    .max_by_key(|id| id.0)
                    .ok_or_else(|| diag("es_functions: unbound ident"))?;
                let slot = self.resolve_var_slot(id);
                let ptr = self
                    .state
                    .allocas
                    .get(&slot)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated local %{}", slot.0)))?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Local { id, .. } => {
                let slot = self.resolve_var_slot(*id);
                let ptr = self
                    .state
                    .allocas
                    .get(&slot)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated local %{}", slot.0)))?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Unary {
                op: draconic_ast::UnaryOp::Plus,
                arg,
                ..
            } => self.emit_number_expr(arg),
            Expr::Unary {
                op: draconic_ast::UnaryOp::Minus,
                arg,
                ..
            } => {
                let a = self.emit_number_expr(arg)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = fneg double {a}").ok();
                Ok(t)
            }
            Expr::Unary {
                op: draconic_ast::UnaryOp::Void,
                ..
            } => Ok(undef_double_const()),
            Expr::Binary {
                left, op, right, ..
            } => {
                use draconic_ast::BinaryOp::*;
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let inst = match op {
                    Add => "fadd",
                    Sub => "fsub",
                    Mul => "fmul",
                    Div => "fdiv",
                    Rem => "frem",
                    _ => return Err(diag("es_functions: unsupported binary")),
                };
                let t = self.fresh();
                writeln!(self.body, "  {t} = {inst} double {l}, {r}").ok();
                Ok(t)
            }
            Expr::Call {
                callee,
                args,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_functions: optional call not supported"));
                }
                self.emit_call(callee, args)
            }
            Expr::Member {
                object,
                property,
                computed,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_functions: optional member not supported"));
                }
                let Expr::Local { id, .. } = object.as_ref() else {
                    return Err(diag("es_functions: member object must be local"));
                };
                let (buf_slot, len_slot) = self
                    .state
                    .arguments_slots
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("es_functions: arguments member outside args fn"))?;
                if !*computed {
                    // arguments.length
                    if static_prop_name(property).as_deref() != Some("length") {
                        return Err(diag("es_functions: only arguments.length supported"));
                    }
                    let argc = self.fresh();
                    writeln!(self.body, "  {argc} = load i64, ptr {len_slot}").ok();
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = sitofp i64 {argc} to double").ok();
                    Ok(t)
                } else {
                    let Expr::Number { raw, .. } = property.as_ref() else {
                        return Err(diag("es_functions: arguments index must be number const"));
                    };
                    let idx = parse_nonneg_index(raw)
                        .ok_or_else(|| diag("es_functions: bad arguments index"))?;
                    // if idx < argc load else undef
                    let argc = self.fresh();
                    writeln!(self.body, "  {argc} = load i64, ptr {len_slot}").ok();
                    let in_range = self.fresh();
                    writeln!(self.body, "  {in_range} = icmp ult i64 {idx}, {argc}").ok();
                    let then_l = self.fresh_label("arg_ok");
                    let else_l = self.fresh_label("arg_miss");
                    let end_l = self.fresh_label("arg_end");
                    writeln!(
                        self.body,
                        "  br i1 {in_range}, label %{then_l}, label %{else_l}"
                    )
                    .ok();
                    writeln!(self.body, "{then_l}:").ok();
                    let buf = self.fresh();
                    writeln!(self.body, "  {buf} = load ptr, ptr {buf_slot}").ok();
                    let gep = self.fresh();
                    writeln!(
                        self.body,
                        "  {gep} = getelementptr inbounds double, ptr {buf}, i64 {idx}"
                    )
                    .ok();
                    let loaded = self.fresh();
                    writeln!(self.body, "  {loaded} = load double, ptr {gep}").ok();
                    writeln!(self.body, "  br label %{end_l}").ok();
                    writeln!(self.body, "{else_l}:").ok();
                    let und = undef_double_const();
                    writeln!(self.body, "  br label %{end_l}").ok();
                    writeln!(self.body, "{end_l}:").ok();
                    let phi = self.fresh();
                    writeln!(
                        self.body,
                        "  {phi} = phi double [ {loaded}, %{then_l} ], [ {und}, %{else_l} ]"
                    )
                    .ok();
                    Ok(phi)
                }
            }
            _ => Err(diag("es_functions: unsupported number expr")),
        }
    }

    fn emit_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let mut arg_vals = Vec::with_capacity(args.len());
        for a in args {
            match a {
                Arg::Expr(e) => arg_vals.push(self.emit_number_expr(e)?),
                Arg::Spread(_) => {
                    return Err(diag("es_functions: spread args not supported"));
                }
            }
        }

        match callee {
            Expr::Local { .. } | Expr::IdentName { .. } => {
                let id = match callee {
                    Expr::Local { id, .. } => *id,
                    Expr::IdentName { name, .. } => self
                        .fn_id_for_ident(name)
                        .ok_or_else(|| diag("es_functions: call to unbound function name"))?,
                    _ => unreachable!(),
                };
                let primary = self.state.info.if_fn_primary.get(&id).copied().or_else(|| {
                    if self.state.info.if_fn_slots.contains(&id) {
                        Some(id)
                    } else {
                        None
                    }
                });
                if let Some(primary) = primary {
                    if self.state.if_fn_slot_ptrs.contains_key(&primary) {
                        return self.emit_dynamic_if_fn_call(primary, &arg_vals);
                    }
                }
                let idx = *self
                    .state
                    .info
                    .fn_binding
                    .get(&id)
                    .ok_or_else(|| diag("es_functions: call to unbound function local"))?;
                self.emit_direct_call(idx, &arg_vals)
            }
            Expr::Function { params, .. } => {
                let idx = find_fn_idx_by_param_patterns(params, &self.state.info.functions)
                    .ok_or_else(|| diag("es_functions: IIFE unknown FunctionExpr"))?;
                self.emit_direct_call(idx, &arg_vals)
            }
            Expr::Member {
                object,
                property,
                computed,
                optional,
                ..
            } => {
                if *optional || *computed {
                    return Err(diag("es_functions: unsupported method call form"));
                }
                let Expr::Local { id: oid, .. } = object.as_ref() else {
                    return Err(diag("es_functions: method call object must be local"));
                };
                let name = static_prop_name(property)
                    .ok_or_else(|| diag("es_functions: method name must be static"))?;
                let idx = *self
                    .state
                    .info
                    .obj_methods
                    .get(oid)
                    .and_then(|m| m.get(&name))
                    .ok_or_else(|| diag(format!("es_functions: unknown method {name}")))?;
                self.emit_direct_call(idx, &arg_vals)
            }
            Expr::Call {
                callee: inner,
                args: inner_args,
                ..
            } => {
                // Higher-order: evaluate inner call (sets @es_ret_fn / caps if returns fn).
                let _inner_ret = self.emit_call(inner, inner_args)?;
                let idx = match inner.as_ref() {
                    Expr::Local { .. } | Expr::IdentName { .. } => {
                        let id = match inner.as_ref() {
                            Expr::Local { id, .. } => *id,
                            Expr::IdentName { name, .. } => {
                                self.fn_id_for_ident(name).ok_or_else(|| {
                                    diag("es_functions: higher-order call unbound callee")
                                })?
                            }
                            _ => unreachable!(),
                        };
                        let caller_idx = *self.state.info.fn_binding.get(&id).ok_or_else(|| {
                            diag("es_functions: higher-order call unbound callee")
                        })?;
                        returned_fn_idx_in_body(
                            &self.state.info.functions[caller_idx].body,
                            &self.state.info.functions,
                        )
                        .ok_or_else(|| diag("es_functions: callee does not return function"))?
                    }
                    _ => return Err(diag("es_functions: unsupported higher-order callee")),
                };
                // Pad defaults / pack rest, then load captures from return buffer.
                let f = &self.state.info.functions[idx];
                if !call_arity_ok(f, arg_vals.len()) {
                    return Err(diag("es_functions: higher-order call arity mismatch"));
                }
                let mut caps = Vec::new();
                for i in 0..f.captures.len() {
                    let gep = self.fresh();
                    writeln!(
                            self.body,
                            "  {gep} = getelementptr inbounds [{MAX_CAPS} x double], ptr @es_ret_cap, i64 0, i64 {i}"
                        )
                        .ok();
                    let c = self.fresh();
                    writeln!(self.body, "  {c} = load double, ptr {gep}").ok();
                    caps.push(c);
                }
                self.emit_call_args(idx, &arg_vals, &caps)
            }
            _ => Err(diag("es_functions: unsupported call callee")),
        }
    }

    fn emit_direct_call(&mut self, idx: usize, arg_vals: &[String]) -> Result<String, Diagnostic> {
        let f = &self.state.info.functions[idx];
        if !call_arity_ok(f, arg_vals.len()) {
            return Err(diag("es_functions: call arity mismatch"));
        }
        let mut caps = Vec::new();
        for cid in &f.captures.clone() {
            let ptr = self.state.allocas.get(cid).cloned().ok_or_else(|| {
                diag(format!(
                    "es_functions: capture local %{} not in caller frame",
                    cid.0
                ))
            })?;
            let t = self.fresh();
            writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
            caps.push(t);
        }
        self.emit_call_args(idx, arg_vals, &caps)
    }

    /// Call through Annex B if-fn slot (i32 idx, -1 = unbound).
    fn emit_dynamic_if_fn_call(
        &mut self,
        primary: LocalId,
        arg_vals: &[String],
    ) -> Result<String, Diagnostic> {
        let slot = self
            .state
            .if_fn_slot_ptrs
            .get(&primary)
            .cloned()
            .ok_or_else(|| diag("es_functions: dynamic call missing slot"))?;
        let candidates = self
            .state
            .info
            .if_fn_candidates
            .get(&primary)
            .cloned()
            .unwrap_or_default();
        if candidates.is_empty() {
            return Err(diag("es_functions: dynamic call has no candidates"));
        }
        // Precompute captures per candidate (same frame).
        let idx_v = self.fresh();
        writeln!(self.body, "  {idx_v} = load i32, ptr {slot}").ok();
        let end_l = self.fresh_label("dyn_end");
        let bad_l = self.fresh_label("dyn_bad");
        let mut case_labels = Vec::new();
        for &cidx in &candidates {
            case_labels.push((cidx, self.fresh_label(&format!("dyn_c{cidx}"))));
        }
        // switch
        let mut sw = format!("  switch i32 {idx_v}, label %{bad_l} [");
        for (cidx, lab) in &case_labels {
            sw.push_str(&format!(" i32 {cidx}, label %{lab}"));
        }
        sw.push_str(" ]");
        writeln!(self.body, "{sw}").ok();

        let mut phi_pairs = Vec::new();
        for (cidx, lab) in &case_labels {
            writeln!(self.body, "{lab}:").ok();
            let ret = self.emit_direct_call(*cidx, arg_vals)?;
            phi_pairs.push((ret, lab.clone()));
            writeln!(self.body, "  br label %{end_l}").ok();
        }
        writeln!(self.body, "{bad_l}:").ok();
        // Unbound / bad idx — return 0 (should not be observed in fixtures).
        let bad_ret = "0.00000000000000000e+00".to_string();
        writeln!(self.body, "  br label %{end_l}").ok();
        writeln!(self.body, "{end_l}:").ok();
        let phi = self.fresh();
        let mut phi_src = String::from(&format!("  {phi} = phi double "));
        let mut first = true;
        for (ret, lab) in &phi_pairs {
            if !first {
                phi_src.push_str(", ");
            }
            first = false;
            phi_src.push_str(&format!("[ {ret}, %{lab} ]"));
        }
        if !first {
            phi_src.push_str(", ");
        }
        phi_src.push_str(&format!("[ {bad_ret}, %{bad_l} ]"));
        writeln!(self.body, "{phi_src}").ok();
        Ok(phi)
    }

    /// Build fixed params (pad defaults), optional rest buffer, then captures.
    fn emit_call_args(
        &mut self,
        idx: usize,
        arg_vals: &[String],
        caps: &[String],
    ) -> Result<String, Diagnostic> {
        let f = &self.state.info.functions[idx];
        let n_fixed = f.params.len();
        let undef = undef_double_const();
        let mut fixed: Vec<String> = arg_vals.iter().take(n_fixed).cloned().collect();
        while fixed.len() < n_fixed {
            fixed.push(undef.clone());
        }

        let mut call_parts: Vec<String> = fixed.iter().map(|v| format!("double {v}")).collect();

        if f.rest.is_some() {
            let rest_vals: Vec<&String> = arg_vals.iter().skip(n_fixed).collect();
            let rest_len = rest_vals.len();
            if rest_len > MAX_REST {
                return Err(diag("es_functions: too many rest args"));
            }
            let buf = self.fresh();
            writeln!(self.body, "  {buf} = alloca [{MAX_REST} x double], align 8").ok();
            for (i, v) in rest_vals.iter().enumerate() {
                let gep = self.fresh();
                writeln!(
                    self.body,
                    "  {gep} = getelementptr inbounds [{MAX_REST} x double], ptr {buf}, i64 0, i64 {i}"
                )
                .ok();
                writeln!(self.body, "  store double {v}, ptr {gep}").ok();
            }
            let buf_ptr = self.fresh();
            writeln!(
                self.body,
                "  {buf_ptr} = getelementptr inbounds [{MAX_REST} x double], ptr {buf}, i64 0, i64 0"
            )
            .ok();
            call_parts.push(format!("ptr {buf_ptr}"));
            call_parts.push(format!("i64 {rest_len}"));
        }

        if f.arguments.is_some() {
            let argc = arg_vals.len();
            if argc > MAX_ARGS {
                return Err(diag("es_functions: too many arguments"));
            }
            let buf = self.fresh();
            writeln!(self.body, "  {buf} = alloca [{MAX_ARGS} x double], align 8").ok();
            for (i, v) in arg_vals.iter().enumerate() {
                let gep = self.fresh();
                writeln!(
                    self.body,
                    "  {gep} = getelementptr inbounds [{MAX_ARGS} x double], ptr {buf}, i64 0, i64 {i}"
                )
                .ok();
                writeln!(self.body, "  store double {v}, ptr {gep}").ok();
            }
            let buf_ptr = self.fresh();
            writeln!(
                self.body,
                "  {buf_ptr} = getelementptr inbounds [{MAX_ARGS} x double], ptr {buf}, i64 0, i64 0"
            )
            .ok();
            call_parts.push(format!("ptr {buf_ptr}"));
            call_parts.push(format!("i64 {argc}"));
        }

        for c in caps {
            call_parts.push(format!("double {c}"));
        }

        let fn_name = self.state.fn_names.get(&idx).cloned().unwrap();
        let t = self.fresh();
        if call_parts.is_empty() {
            writeln!(self.body, "  {t} = call double @{fn_name}()").ok();
        } else {
            writeln!(
                self.body,
                "  {t} = call double @{fn_name}({})",
                call_parts.join(", ")
            )
            .ok();
        }
        Ok(t)
    }
}
