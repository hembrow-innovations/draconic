use std::collections::HashMap;
use std::fmt::Write as _;

use super::*;

impl<'a> super::Emitter<'a> {
    /// Same-name `var` redecls / uses share one primary storage slot.
    pub(super) fn resolve_var_slot(&self, id: LocalId) -> LocalId {
        self.state.info.var_primary.get(&id).copied().unwrap_or(id)
    }

    pub(super) fn fn_id_for_ident(&self, name: &str) -> Option<LocalId> {
        self.state
            .info
            .fn_binding
            .keys()
            .copied()
            .chain(self.state.info.if_fn_slots.iter().copied())
            .chain(self.state.info.if_fn_primary.keys().copied())
            .filter(|id| {
                self.module
                    .locals
                    .iter()
                    .any(|l| l.id == *id && l.name == name)
            })
            .max_by_key(|id| id.0)
    }

    pub(super) fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.03.07+N08.16.24 ES functions + defaults/rest/arguments via Runtime ABI)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(&[PRINT_F64, PRINT_STR])).ok();
        writeln!(self.out, "@es_ret_fn = private global i32 -1").ok();
        writeln!(
            self.out,
            "@es_ret_cap = private global [{MAX_CAPS} x double] zeroinitializer"
        )
        .ok();
        writeln!(self.out).ok();

        for f in &info.functions {
            self.emit_function(f)?;
        }

        self.body.clear();
        self.tmp = 0;
        self.label = 0;
        self.state.allocas.clear();
        self.state.rest_slots.clear();
        self.state.arguments_slots.clear();
        self.state.if_fn_slot_ptrs.clear();
        self.state.typeof_code_ptrs.clear();

        // String globals for typeof observations (emitted before main).
        let mut prelude = String::new();

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();

        // Hoisted script-scope `var` slots (init undefined).
        let mut top_vars: Vec<LocalId> = info.top_var_slots.iter().copied().collect();
        top_vars.sort_by_key(|id| id.0);
        for id in top_vars {
            let ptr = format!("%l{}", id.0);
            self.state.allocas.insert(id, ptr.clone());
            SlotTy::Number.write_alloca(&mut self.out, &ptr);
            writeln!(
                self.out,
                "  store double {}, ptr {ptr}",
                undef_double_const()
            )
            .ok();
        }

        for id in &info.user_locals {
            if info.string_locals.contains(id) {
                let ptr = format!("%typeof{}", id.0);
                self.state.typeof_code_ptrs.insert(*id, ptr.clone());
                writeln!(self.out, "  {ptr} = alloca i32, align 4").ok();
                writeln!(self.out, "  store i32 0, ptr {ptr}").ok();
            } else if !self.state.allocas.contains_key(id) {
                let ptr = format!("%l{}", id.0);
                self.state.allocas.insert(*id, ptr.clone());
                SlotTy::Number.write_alloca(&mut self.out, &ptr);
            }
        }
        // Annex B if-fn binding slots (top-level).
        let mut slot_ids: Vec<LocalId> = info.if_fn_slots.iter().copied().collect();
        slot_ids.sort_by_key(|id| id.0);
        for id in slot_ids {
            // Only slots whose Function is not nested inside another function body.
            if self.if_fn_slot_owned_by_top(id) {
                let ptr = format!("%iffn{}", id.0);
                self.state.if_fn_slot_ptrs.insert(id, ptr.clone());
                writeln!(self.out, "  {ptr} = alloca i32, align 4").ok();
                writeln!(self.out, "  store i32 -1, ptr {ptr}").ok();
            }
        }
        // Function-binding locals that are only used as call targets need no storage
        // when statically bound; assigns of FunctionExpr to unused-as-value slots skip.
        // Block-scoped number locals allocate on demand in emit_top_stmt.

        for stmt in &self.module.body {
            self.emit_top_stmt(stmt)?;
        }

        for id in &info.user_locals {
            if info.string_locals.contains(id) {
                let code_ptr = self
                    .state
                    .typeof_code_ptrs
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("internal: typeof print missing"))?;
                let code = self.fresh();
                writeln!(self.body, "  {code} = load i32, ptr {code_ptr}").ok();
                let is_fn = self.fresh();
                writeln!(self.body, "  {is_fn} = icmp eq i32 {code}, 1").ok();
                let then_l = self.fresh_label("ty_fn");
                let else_l = self.fresh_label("ty_und");
                let end_l = self.fresh_label("ty_end");
                writeln!(
                    self.body,
                    "  br i1 {is_fn}, label %{then_l}, label %{else_l}"
                )
                .ok();
                writeln!(self.body, "{then_l}:").ok();
                self.emit_print_str("function")?;
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{else_l}:").ok();
                self.emit_print_str("undefined")?;
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{end_l}:").ok();
            } else {
                // Number / `var` observations: print "undefined" for the undef sentinel.
                let slot = self.resolve_var_slot(*id);
                let ptr = self
                    .state
                    .allocas
                    .get(&slot)
                    .cloned()
                    .ok_or_else(|| diag("internal: print missing alloca"))?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                let bits = self.fresh();
                writeln!(self.body, "  {bits} = bitcast double {v} to i64").ok();
                let is_u = self.fresh();
                writeln!(self.body, "  {is_u} = icmp eq i64 {bits}, {UNDEF_BITS}").ok();
                let und_l = self.fresh_label("print_und");
                let num_l = self.fresh_label("print_num");
                let end_l = self.fresh_label("print_end");
                writeln!(self.body, "  br i1 {is_u}, label %{und_l}, label %{num_l}").ok();
                writeln!(self.body, "{und_l}:").ok();
                self.emit_print_str("undefined")?;
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{num_l}:").ok();
                writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{end_l}:").ok();
            }
        }

        // Emit string globals before main definition.
        for (s, gname) in &self.state.str_globals {
            let n = s.len() + 1;
            let esc = escape_llvm_bytes(s.as_bytes());
            writeln!(
                prelude,
                "@{gname} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\""
            )
            .ok();
        }
        if !prelude.is_empty() {
            // Insert globals before `define i32 @main` — rewrite out.
            let main_def = "define i32 @main()";
            if let Some(pos) = self.out.find(main_def) {
                let mut new_out = String::new();
                new_out.push_str(&self.out[..pos]);
                new_out.push_str(&prelude);
                new_out.push('\n');
                new_out.push_str(&self.out[pos..]);
                self.out = new_out;
            }
        }

        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    /// True when `id` is an if-fn primary whose Function stmts appear only at top level
    /// (not nested inside another function). Nested slots are allocated in that function.
    fn if_fn_slot_owned_by_top(&self, id: LocalId) -> bool {
        !self.if_fn_nested_in_any_function(id)
    }

    fn if_fn_nested_in_any_function(&self, id: LocalId) -> bool {
        for f in &self.state.info.functions {
            if stmt_list_mentions_if_fn(&f.body, id) {
                return true;
            }
        }
        false
    }

    fn emit_print_str(&mut self, s: &str) -> Result<(), Diagnostic> {
        let gname = if let Some(g) = self.state.str_globals.get(s) {
            g.clone()
        } else {
            let g = format!(".esfn.str.{}", self.state.str_globals.len());
            self.state.str_globals.insert(s.to_string(), g.clone());
            g
        };
        let t = self.fresh();
        let n = s.len() + 1;
        writeln!(
            self.body,
            "  {t} = getelementptr inbounds [{n} x i8], ptr @{gname}, i64 0, i64 0"
        )
        .ok();
        writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {t}"))).ok();
        Ok(())
    }

    fn emit_function(&mut self, f: &FnInfo) -> Result<(), Diagnostic> {
        let fn_name = self.state.fn_names.get(&f.idx).cloned().unwrap();

        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_label = self.label;
        let saved_allocas = std::mem::take(&mut self.state.allocas);
        let saved_rest = std::mem::take(&mut self.state.rest_slots);
        let saved_args = std::mem::take(&mut self.state.arguments_slots);
        let saved_if_slots = std::mem::take(&mut self.state.if_fn_slot_ptrs);

        self.tmp = 0;
        self.label = 0;
        self.state.allocas.clear();
        self.state.rest_slots.clear();
        self.state.arguments_slots.clear();
        self.state.if_fn_slot_ptrs.clear();

        let mut sig_parts = Vec::new();
        for (i, _) in f.params.iter().enumerate() {
            sig_parts.push(format!("double %p{i}"));
        }
        if f.rest.is_some() {
            sig_parts.push("ptr %rest_buf".into());
            sig_parts.push("i64 %rest_len".into());
        }
        if f.arguments.is_some() {
            sig_parts.push("ptr %args_buf".into());
            sig_parts.push("i64 %argc".into());
        }
        for (i, _) in f.captures.iter().enumerate() {
            sig_parts.push(format!("double %c{i}"));
        }
        let sig = sig_parts.join(", ");

        let mut entry = String::new();
        for (i, pid) in f.params.iter().enumerate() {
            let ptr = format!("%l{}", pid.0);
            self.state.allocas.insert(*pid, ptr.clone());
            SlotTy::Number.write_alloca(&mut entry, &ptr);
            writeln!(entry, "  store double %p{i}, ptr {ptr}").ok();
        }
        if let Some(rid) = f.rest {
            let buf_slot = format!("%rest_buf_slot{}", rid.0);
            let len_slot = format!("%rest_len_slot{}", rid.0);
            writeln!(entry, "  {buf_slot} = alloca ptr, align 8").ok();
            writeln!(entry, "  {len_slot} = alloca i64, align 8").ok();
            writeln!(entry, "  store ptr %rest_buf, ptr {buf_slot}").ok();
            writeln!(entry, "  store i64 %rest_len, ptr {len_slot}").ok();
            self.state.rest_slots.insert(rid, (buf_slot, len_slot));
        }
        if let Some(aid) = f.arguments {
            let buf_slot = format!("%args_buf_slot{}", aid.0);
            let len_slot = format!("%argc_slot{}", aid.0);
            writeln!(entry, "  {buf_slot} = alloca ptr, align 8").ok();
            writeln!(entry, "  {len_slot} = alloca i64, align 8").ok();
            writeln!(entry, "  store ptr %args_buf, ptr {buf_slot}").ok();
            writeln!(entry, "  store i64 %argc, ptr {len_slot}").ok();
            self.state.arguments_slots.insert(aid, (buf_slot, len_slot));
        }
        for (i, cid) in f.captures.iter().enumerate() {
            let ptr = format!("%l{}", cid.0);
            self.state.allocas.insert(*cid, ptr.clone());
            SlotTy::Number.write_alloca(&mut entry, &ptr);
            writeln!(entry, "  store double %c{i}, ptr {ptr}").ok();
        }
        // Hoisted function-scope `var` slots (init undefined).
        if let Some(slots) = self.state.info.fn_var_slots.get(&f.idx) {
            let mut ids: Vec<LocalId> = slots.iter().copied().collect();
            ids.sort_by_key(|id| id.0);
            for id in ids {
                if self.state.allocas.contains_key(&id) {
                    continue;
                }
                let ptr = format!("%l{}", id.0);
                self.state.allocas.insert(id, ptr.clone());
                SlotTy::Number.write_alloca(&mut entry, &ptr);
                writeln!(entry, "  store double {}, ptr {ptr}", undef_double_const()).ok();
            }
        }
        // Nested Annex B if-fn slots for this function body.
        let mut nested_slots: Vec<LocalId> = self
            .state
            .info
            .if_fn_slots
            .iter()
            .copied()
            .filter(|id| stmt_list_mentions_if_fn(&f.body, *id))
            .collect();
        nested_slots.sort_by_key(|id| id.0);
        for id in nested_slots {
            let ptr = format!("%iffn{}", id.0);
            self.state.if_fn_slot_ptrs.insert(id, ptr.clone());
            writeln!(entry, "  {ptr} = alloca i32, align 4").ok();
            writeln!(entry, "  store i32 -1, ptr {ptr}").ok();
        }

        writeln!(self.out, "define double @{fn_name}({sig}) {{").ok();
        writeln!(self.out, "entry:").ok();
        write!(self.out, "{entry}").ok();

        // Apply defaults left-to-right when arg is missing/undefined sentinel.
        let defaults = f.defaults.clone();
        let param_ids = f.params.clone();
        for (i, pid) in param_ids.iter().enumerate() {
            if let Some(def) = &defaults[i] {
                self.emit_param_default(*pid, def)?;
            }
        }

        for stmt in &f.body {
            self.emit_fn_stmt(stmt)?;
        }
        if !self.body_ends_with_terminator() {
            writeln!(self.body, "  ret double 0.00000000000000000e+00").ok();
        }

        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.label = saved_label;
        self.state.allocas = saved_allocas;
        self.state.rest_slots = saved_rest;
        self.state.arguments_slots = saved_args;
        self.state.if_fn_slot_ptrs = saved_if_slots;
        Ok(())
    }

    fn emit_top_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, kind } => {
                let by_id: HashMap<_, _> = self.module.locals.iter().map(|l| (l.id, l)).collect();
                if init
                    .as_ref()
                    .is_some_and(|e| crate::es_console::is_global_this_console(e, &by_id))
                {
                    return Ok(());
                }
                if self.state.info.fn_binding.contains_key(local) {
                    // Function binding — no number storage required for static calls.
                    return Ok(());
                }
                if self.state.info.obj_methods.contains_key(local) {
                    // Object holding static methods — methods resolved via obj_methods table.
                    return Ok(());
                }
                if self.state.info.string_locals.contains(local) {
                    let init = init
                        .as_ref()
                        .ok_or_else(|| diag("es_functions: typeof declare requires init"))?;
                    return self.emit_typeof_declare(*local, init);
                }
                // `var` is hoisted to entry as undefined; bare `var x` is a no-op store.
                let is_var =
                    *kind == BindingKind::Var || self.state.info.var_primary.contains_key(local);
                let slot = self.resolve_var_slot(*local);
                if is_var {
                    let ptr = self
                        .state
                        .allocas
                        .get(&slot)
                        .cloned()
                        .ok_or_else(|| diag("es_functions: var slot missing alloca"))?;
                    if let Some(init) = init.as_ref() {
                        let v = self.emit_number_expr(init)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    return Ok(());
                }
                let init = init
                    .as_ref()
                    .ok_or_else(|| diag("es_functions: declare requires init"))?;
                let ptr = if let Some(p) = self.state.allocas.get(&slot).cloned() {
                    p
                } else {
                    let p = format!("%l{}", slot.0);
                    self.state.allocas.insert(slot, p.clone());
                    SlotTy::Number.write_alloca(&mut self.body, &p);
                    p
                };
                let v = self.emit_number_expr(init)?;
                writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                Ok(())
            }
            Stmt::Function { local, .. } => self.emit_if_fn_activate(*local),
            Stmt::Labeled { body, .. } => self.emit_top_stmt(body),
            Stmt::Block { body } => {
                for s in body {
                    self.emit_top_stmt(s)?;
                }
                Ok(())
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => self.emit_if_stmt(test, consequent, alternate, true),
            Stmt::Expr { expr } => {
                let by_id: HashMap<_, _> = self.module.locals.iter().map(|l| (l.id, l)).collect();
                if let Some(value) = crate::es_console::console_log_string_arg(expr, &by_id) {
                    return self.emit_print_str(&value.to_string_lossy());
                }
                match expr {
                    Expr::Assign {
                        target: AssignTarget::Local(id),
                        op: AssignOp::Eq,
                        value,
                        ..
                    } => {
                        let slot = self.resolve_var_slot(*id);
                        let ptr = self
                            .state
                            .allocas
                            .get(&slot)
                            .cloned()
                            .ok_or_else(|| diag("es_functions: top assign missing alloca"))?;
                        let v = self.emit_number_expr(value)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                        Ok(())
                    }
                    _ => Err(diag("es_functions: unsupported top-level expr stmt")),
                }
            }
            _ => Err(diag("es_functions: unsupported top-level stmt")),
        }
    }

    fn emit_typeof_declare(&mut self, local: LocalId, init: &Expr) -> Result<(), Diagnostic> {
        let Expr::Unary {
            op: draconic_ast::UnaryOp::TypeOf,
            arg,
            ..
        } = init
        else {
            return Err(diag("es_functions: string local must be typeof"));
        };
        let Expr::Local { id, .. } = arg.as_ref() else {
            return Err(diag("es_functions: typeof arg must be local"));
        };
        let code_ptr = self
            .state
            .typeof_code_ptrs
            .get(&local)
            .cloned()
            .ok_or_else(|| diag("es_functions: typeof code slot missing"))?;
        let primary = self
            .state
            .info
            .if_fn_primary
            .get(id)
            .copied()
            .unwrap_or(*id);
        if let Some(slot) = self.state.if_fn_slot_ptrs.get(&primary).cloned() {
            let idx = self.fresh();
            writeln!(self.body, "  {idx} = load i32, ptr {slot}").ok();
            let bound = self.fresh();
            writeln!(self.body, "  {bound} = icmp ne i32 {idx}, -1").ok();
            let t = self.fresh();
            writeln!(self.body, "  {t} = zext i1 {bound} to i32").ok();
            writeln!(self.body, "  store i32 {t}, ptr {code_ptr}").ok();
        } else if self.state.info.fn_binding.contains_key(id) {
            // Always-bound function decl.
            writeln!(self.body, "  store i32 1, ptr {code_ptr}").ok();
        } else {
            // Unbound / hoisted-uninit `var` typeof → "undefined" (code 0).
            // Number typeof string obs is out of scope for this path's table.
            writeln!(self.body, "  store i32 0, ptr {code_ptr}").ok();
        }
        Ok(())
    }

    /// Activate Annex B if-clause function: store its fn idx into the primary slot.
    fn emit_if_fn_activate(&mut self, local: LocalId) -> Result<(), Diagnostic> {
        let Some(primary) = self.state.info.if_fn_primary.get(&local).copied() else {
            // Ordinary function decl (not if-clause) — always available via fn_binding.
            return Ok(());
        };
        let Some(&idx) = self.state.info.fn_binding.get(&local) else {
            return Ok(());
        };
        let Some(slot) = self.state.if_fn_slot_ptrs.get(&primary).cloned() else {
            return Err(diag(format!(
                "es_functions: if-fn slot missing for %{}",
                primary.0
            )));
        };
        writeln!(self.body, "  store i32 {idx}, ptr {slot}").ok();
        Ok(())
    }

    fn emit_if_stmt(
        &mut self,
        test: &Expr,
        consequent: &Stmt,
        alternate: &Option<Box<Stmt>>,
        top: bool,
    ) -> Result<(), Diagnostic> {
        let cond = self.emit_bool_expr(test)?;
        let then_l = self.fresh_label("then");
        let else_l = self.fresh_label("else");
        let end_l = self.fresh_label("endif");
        if alternate.is_some() {
            writeln!(
                self.body,
                "  br i1 {cond}, label %{then_l}, label %{else_l}"
            )
            .ok();
        } else {
            writeln!(self.body, "  br i1 {cond}, label %{then_l}, label %{end_l}").ok();
        }
        writeln!(self.body, "{then_l}:").ok();
        if top {
            self.emit_top_stmt(consequent)?;
        } else {
            self.emit_fn_stmt(consequent)?;
        }
        if !self.body_ends_with_terminator() {
            writeln!(self.body, "  br label %{end_l}").ok();
        }
        if let Some(alt) = alternate {
            writeln!(self.body, "{else_l}:").ok();
            if top {
                self.emit_top_stmt(alt)?;
            } else {
                self.emit_fn_stmt(alt)?;
            }
            if !self.body_ends_with_terminator() {
                writeln!(self.body, "  br label %{end_l}").ok();
            }
        }
        writeln!(self.body, "{end_l}:").ok();
        Ok(())
    }

    fn emit_fn_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Return { value: Some(v) } => {
                if let Expr::Function { .. } = v {
                    return self.emit_return_fn(v);
                }
                let n = self.emit_number_expr(v)?;
                writeln!(self.body, "  ret double {n}").ok();
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    if self.body_ends_with_terminator() {
                        break;
                    }
                    self.emit_fn_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Declare { local, init, kind } => {
                if self.state.info.fn_binding.contains_key(local) {
                    return Ok(());
                }
                // Hoisted `var`: store init into primary (bare `var` already undef at entry).
                let is_var =
                    *kind == BindingKind::Var || self.state.info.var_primary.contains_key(local);
                if is_var {
                    let slot = self.resolve_var_slot(*local);
                    let ptr = self
                        .state
                        .allocas
                        .get(&slot)
                        .cloned()
                        .ok_or_else(|| diag("es_functions: fn var slot missing alloca"))?;
                    if let Some(e) = init {
                        if !matches!(e, Expr::Function { .. }) {
                            let v = self.emit_number_expr(e)?;
                            writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                        }
                    }
                    return Ok(());
                }
                let ptr = format!("%l{}", local.0);
                self.state.allocas.insert(*local, ptr.clone());
                SlotTy::Number.write_alloca(&mut self.body, &ptr);
                if let Some(e) = init {
                    if matches!(e, Expr::Function { .. }) {
                        writeln!(
                            self.body,
                            "  store double 0.00000000000000000e+00, ptr {ptr}"
                        )
                        .ok();
                    } else {
                        let v = self.emit_number_expr(e)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                } else {
                    writeln!(
                        self.body,
                        "  store double 0.00000000000000000e+00, ptr {ptr}"
                    )
                    .ok();
                }
                Ok(())
            }
            Stmt::Function { local, .. } => self.emit_if_fn_activate(*local),
            Stmt::Labeled { body, .. } => self.emit_fn_stmt(body),
            Stmt::If {
                test,
                consequent,
                alternate,
            } => self.emit_if_stmt(test, consequent, alternate, false),
            Stmt::ForOf {
                left,
                right,
                body,
                is_await,
            } => {
                if *is_await {
                    return Err(diag("es_functions: for-await-of not supported"));
                }
                self.emit_for_of_rest(left, right, body)
            }
            Stmt::Expr { expr } => match expr {
                Expr::Assign {
                    target: AssignTarget::Local(id),
                    op: AssignOp::Eq,
                    value,
                    ..
                } => {
                    let slot = self.resolve_var_slot(*id);
                    let ptr = self
                        .state
                        .allocas
                        .get(&slot)
                        .cloned()
                        .ok_or_else(|| diag("es_functions: assign missing alloca"))?;
                    let v = self.emit_number_expr(value)?;
                    writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    Ok(())
                }
                _ => Err(diag("es_functions: unsupported expr stmt")),
            },
            _ => Err(diag("es_functions: unsupported stmt in function body")),
        }
    }

    fn emit_for_of_rest(
        &mut self,
        left: &Stmt,
        right: &Expr,
        body: &Stmt,
    ) -> Result<(), Diagnostic> {
        let Expr::Local { id: rest_id, .. } = right else {
            return Err(diag("es_functions: for-of right must be rest local"));
        };
        let (buf_slot, len_slot) = self
            .state
            .rest_slots
            .get(rest_id)
            .cloned()
            .ok_or_else(|| diag("es_functions: for-of rest slot missing"))?;
        let Stmt::Declare {
            local: bind_id,
            init: None,
            ..
        } = left
        else {
            return Err(diag("es_functions: for-of left must be bare let binding"));
        };
        let bind_ptr = format!("%l{}", bind_id.0);
        self.state.allocas.insert(*bind_id, bind_ptr.clone());
        SlotTy::Number.write_alloca(&mut self.body, &bind_ptr);

        let buf = self.fresh();
        let len = self.fresh();
        writeln!(self.body, "  {buf} = load ptr, ptr {buf_slot}").ok();
        writeln!(self.body, "  {len} = load i64, ptr {len_slot}").ok();
        let idx_ptr = self.fresh();
        writeln!(self.body, "  {idx_ptr} = alloca i64, align 8").ok();
        writeln!(self.body, "  store i64 0, ptr {idx_ptr}").ok();

        let head = self.fresh_label("forof_head");
        let bod = self.fresh_label("forof_body");
        let cont = self.fresh_label("forof_cont");
        let end = self.fresh_label("forof_end");
        writeln!(self.body, "  br label %{head}").ok();
        writeln!(self.body, "{head}:").ok();
        let idx = self.fresh();
        writeln!(self.body, "  {idx} = load i64, ptr {idx_ptr}").ok();
        let cmp = self.fresh();
        writeln!(self.body, "  {cmp} = icmp ult i64 {idx}, {len}").ok();
        writeln!(self.body, "  br i1 {cmp}, label %{bod}, label %{end}").ok();
        writeln!(self.body, "{bod}:").ok();
        let gep = self.fresh();
        writeln!(
            self.body,
            "  {gep} = getelementptr inbounds double, ptr {buf}, i64 {idx}"
        )
        .ok();
        let elem = self.fresh();
        writeln!(self.body, "  {elem} = load double, ptr {gep}").ok();
        writeln!(self.body, "  store double {elem}, ptr {bind_ptr}").ok();
        self.emit_fn_stmt(body)?;
        if !self.body_ends_with_terminator() {
            writeln!(self.body, "  br label %{cont}").ok();
        }
        writeln!(self.body, "{cont}:").ok();
        let idx2 = self.fresh();
        writeln!(self.body, "  {idx2} = load i64, ptr {idx_ptr}").ok();
        let next = self.fresh();
        writeln!(self.body, "  {next} = add i64 {idx2}, 1").ok();
        writeln!(self.body, "  store i64 {next}, ptr {idx_ptr}").ok();
        writeln!(self.body, "  br label %{head}").ok();
        writeln!(self.body, "{end}:").ok();
        Ok(())
    }

    fn emit_return_fn(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        let Expr::Function { params, .. } = expr else {
            return Err(diag("internal: emit_return_fn"));
        };
        let idx = find_fn_idx_by_param_patterns(params, &self.state.info.functions)
            .ok_or_else(|| diag("es_functions: return unknown FunctionExpr"))?;
        let f = &self.state.info.functions[idx];
        writeln!(self.body, "  store i32 {idx}, ptr @es_ret_fn").ok();
        for (i, cid) in f.captures.iter().enumerate() {
            let ptr = self.state.allocas.get(cid).cloned().ok_or_else(|| {
                diag(format!(
                    "es_functions: return capture %{} not in frame",
                    cid.0
                ))
            })?;
            let v = self.fresh();
            writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
            let gep = self.fresh();
            writeln!(
                self.body,
                "  {gep} = getelementptr inbounds [{MAX_CAPS} x double], ptr @es_ret_cap, i64 0, i64 {i}"
            )
            .ok();
            writeln!(self.body, "  store double {v}, ptr {gep}").ok();
        }
        // Return fn idx as double for chaining.
        let d = self.fresh();
        writeln!(self.body, "  {d} = sitofp i32 {idx} to double").ok();
        writeln!(self.body, "  ret double {d}").ok();
        Ok(())
    }

    fn emit_param_default(&mut self, pid: LocalId, def: &Expr) -> Result<(), Diagnostic> {
        let ptr = self
            .state
            .allocas
            .get(&pid)
            .cloned()
            .ok_or_else(|| diag("es_functions: default param missing alloca"))?;
        let cur = self.fresh();
        writeln!(self.body, "  {cur} = load double, ptr {ptr}").ok();
        let bits = self.fresh();
        writeln!(self.body, "  {bits} = bitcast double {cur} to i64").ok();
        let is_u = self.fresh();
        writeln!(self.body, "  {is_u} = icmp eq i64 {bits}, {UNDEF_BITS}").ok();
        let then_l = self.fresh_label("def");
        let end_l = self.fresh_label("defend");
        writeln!(self.body, "  br i1 {is_u}, label %{then_l}, label %{end_l}").ok();
        writeln!(self.body, "{then_l}:").ok();
        let v = self.emit_number_expr(def)?;
        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
        writeln!(self.body, "  br label %{end_l}").ok();
        writeln!(self.body, "{end_l}:").ok();
        Ok(())
    }

    fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Boolean { value, .. } => {
                let t = self.fresh();
                let bit = if *value { 1 } else { 0 };
                writeln!(self.body, "  {t} = add i1 0, {bit}").ok();
                Ok(t)
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                use draconic_ast::BinaryOp::*;
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let pred = match op {
                    Lt => "olt",
                    LtEq => "ole",
                    Gt => "ogt",
                    GtEq => "oge",
                    EqEq | EqEqEq => "oeq",
                    NotEq | NotEqEq => "one",
                    _ => return Err(diag("es_functions: unsupported compare")),
                };
                let t = self.fresh();
                writeln!(self.body, "  {t} = fcmp {pred} double {l}, {r}").ok();
                Ok(t)
            }
            _ => {
                // ToBoolean on number: != 0
                let n = self.emit_number_expr(expr)?;
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {t} = fcmp one double {n}, 0.00000000000000000e+00"
                )
                .ok();
                Ok(t)
            }
        }
    }
}
