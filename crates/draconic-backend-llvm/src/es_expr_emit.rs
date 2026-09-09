use std::collections::HashMap;
use std::fmt::Write as _;

use crate::emitter::{escape_llvm_bytes, SlotTy};
use draconic_ast::UnaryOp;
use draconic_diagnostics::Diagnostic;
use draconic_ir::{Expr, IrType as Type, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, CSTR_FROM_CODE_UNIT_N, CSTR_FROM_U64, CSTR_LEN, ES_EXPR_DECLARES, PRINT_BOOL,
    PRINT_BYTES, PRINT_F64, PRINT_I64, UTF16_LEN,
};

use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn emit_module(
        &mut self,
        alloc: &[(LocalId, SlotTy)],
        user: &[(LocalId, SlotTy)],
    ) -> Result<(), Diagnostic> {
        // Body first so string globals are collected, then header + globals + main.
        for (id, slot) in alloc {
            let ptr = format!("%l{}", id.0);
            self.state.allocas.insert(*id, (ptr.clone(), *slot));
            slot.write_alloca(&mut self.body, &ptr);
            if *slot == SlotTy::String {
                let len_ptr = format!("%l{}_len", id.0);
                writeln!(self.body, "  {len_ptr} = alloca i64, align 8").ok();
                self.state.string_lens.insert(*id, len_ptr);
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        // Print top-level user locals in declaration order (not for-init bindings).
        for (id, slot) in user {
            match slot {
                SlotTy::Number => {
                    let (ptr, _) = self
                        .state
                        .allocas
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("internal: print missing alloca"))?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                }
                SlotTy::BigInt => {
                    let (ptr, _) = self
                        .state
                        .allocas
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("internal: print missing alloca"))?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i64, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_I64.call(&format!("i64 {v}"))).ok();
                }
                SlotTy::Boolean => {
                    let (ptr, _) = self
                        .state
                        .allocas
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("internal: print missing alloca"))?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i1, ptr {ptr}").ok();
                    let ext = self.fresh();
                    writeln!(self.body, "  {ext} = zext i1 {v} to i8").ok();
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {ext}"))).ok();
                }
                SlotTy::String => {
                    let (ptr, _) = self
                        .state
                        .allocas
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("internal: print missing alloca"))?;
                    let len_ptr = self
                        .state
                        .string_lens
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("internal: print missing string len"))?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    let n = self.fresh();
                    writeln!(self.body, "  {n} = load i64, ptr {len_ptr}").ok();
                    writeln!(
                        self.body,
                        "  {}",
                        PRINT_BYTES.call(&format!("ptr {v}, i64 {n}"))
                    )
                    .ok();
                }
                SlotTy::Undefined => {
                    let p = self.string_const("undefined")?;
                    writeln!(
                        self.body,
                        "  {}",
                        PRINT_BYTES.call(&format!("ptr {}, i64 {}", p.data, p.len))
                    )
                    .ok();
                }
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.01/N08.02/N08.07.01–N08.07.05 ES expressions + control + strings via Runtime ABI)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        // JS `**` / Math.pow + Math.* methods (IEEE f64 intrinsics; no extra libm flags).
        writeln!(self.out, "declare double @llvm.pow.f64(double, double)").ok();
        writeln!(self.out, "declare double @llvm.fabs.f64(double)").ok();
        writeln!(self.out, "declare double @llvm.floor.f64(double)").ok();
        writeln!(self.out, "declare double @llvm.ceil.f64(double)").ok();
        writeln!(self.out, "declare double @llvm.round.f64(double)").ok();
        writeln!(self.out, "declare double @llvm.sqrt.f64(double)").ok();
        writeln!(self.out).ok();

        for (content, gname) in &self.state.str_globals {
            let n = content.len() + 1;
            let esc = escape_llvm_bytes(content);
            writeln!(
                self.out,
                "@{gname} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
            )
            .ok();
        }
        if !self.state.str_globals.is_empty() {
            writeln!(self.out).ok();
        }

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    pub(super) fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let (ptr, slot) = self
                    .state
                    .allocas
                    .get(local)
                    .cloned()
                    .ok_or_else(|| diag("internal: missing alloca"))?;
                match (slot, init) {
                    (SlotTy::Number, Some(init)) => {
                        let v = self.emit_number_expr(init)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    (SlotTy::BigInt, Some(init)) => {
                        let v = self.emit_bigint_expr(init)?;
                        writeln!(self.body, "  store i64 {v}, ptr {ptr}").ok();
                    }
                    (SlotTy::Boolean, Some(init)) => {
                        let v = self.emit_bool_expr(init)?;
                        writeln!(self.body, "  store i1 {v}, ptr {ptr}").ok();
                    }
                    (SlotTy::String, Some(init)) => {
                        let v = self.emit_string_expr(init)?;
                        self.store_string_local(*local, &v)?;
                    }
                    (SlotTy::Undefined, Some(init)) => {
                        self.emit_undefined_expr(init)?;
                    }
                    // Uninitialized string `let` (incl. any) — empty until assigned.
                    (SlotTy::String, None) => {
                        let v = self.string_const("")?;
                        self.store_string_local(*local, &v)?;
                    }
                    // Uninitialized number/bigint/bool/undefined — leave alloca undef until assigned.
                    (_, None) => {}
                }
                Ok(())
            }
            Stmt::Expr { expr } => match expr.ty() {
                Type::Number => {
                    let _ = self.emit_number_expr(expr)?;
                    Ok(())
                }
                Type::BigInt => {
                    let _ = self.emit_bigint_expr(expr)?;
                    Ok(())
                }
                Type::Boolean => {
                    let _ = self.emit_bool_expr(expr)?;
                    Ok(())
                }
                Type::String => {
                    let _ = self.emit_string_expr(expr)?;
                    Ok(())
                }
                Type::Null => self.emit_undefined_expr(expr),
                _ => Err(diag("internal: unsupported expr stmt ty in es_expr module")),
            },
            Stmt::Block { body } => {
                for s in body {
                    if self.body_ends_with_terminator() {
                        break;
                    }
                    self.emit_stmt(s)?;
                }
                Ok(())
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                let cond = self.emit_to_boolean(test)?;
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
                self.emit_stmt(consequent)?;
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{end_l}").ok();
                }
                if let Some(alt) = alternate {
                    writeln!(self.body, "{else_l}:").ok();
                    self.emit_stmt(alt)?;
                    if !self.body_ends_with_terminator() {
                        writeln!(self.body, "  br label %{end_l}").ok();
                    }
                }
                writeln!(self.body, "{end_l}:").ok();
                Ok(())
            }
            Stmt::While { test, body } => {
                let names = std::mem::take(&mut self.state.pending_names);
                let head = self.fresh_label("while_head");
                let bod = self.fresh_label("while_body");
                let end = self.fresh_label("while_end");
                writeln!(self.body, "  br label %{head}").ok();
                writeln!(self.body, "{head}:").ok();
                let cond = self.emit_to_boolean(test)?;
                writeln!(self.body, "  br i1 {cond}, label %{bod}, label %{end}").ok();
                writeln!(self.body, "{bod}:").ok();
                self.state.ctrls.push(CtrlFrame {
                    names,
                    break_label: end.clone(),
                    continue_label: Some(head.clone()),
                });
                self.emit_stmt(body)?;
                self.state.ctrls.pop();
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{head}").ok();
                }
                writeln!(self.body, "{end}:").ok();
                Ok(())
            }
            Stmt::DoWhile { body, test } => {
                let names = std::mem::take(&mut self.state.pending_names);
                let bod = self.fresh_label("do_body");
                let head = self.fresh_label("do_test");
                let end = self.fresh_label("do_end");
                writeln!(self.body, "  br label %{bod}").ok();
                writeln!(self.body, "{bod}:").ok();
                self.state.ctrls.push(CtrlFrame {
                    names,
                    break_label: end.clone(),
                    continue_label: Some(head.clone()),
                });
                self.emit_stmt(body)?;
                self.state.ctrls.pop();
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{head}").ok();
                }
                writeln!(self.body, "{head}:").ok();
                let cond = self.emit_to_boolean(test)?;
                writeln!(self.body, "  br i1 {cond}, label %{bod}, label %{end}").ok();
                writeln!(self.body, "{end}:").ok();
                Ok(())
            }
            Stmt::For {
                init,
                test,
                update,
                body,
            } => {
                let names = std::mem::take(&mut self.state.pending_names);
                if let Some(i) = init {
                    self.emit_stmt(i)?;
                }
                let head = self.fresh_label("for_head");
                let bod = self.fresh_label("for_body");
                let upd = self.fresh_label("for_update");
                let end = self.fresh_label("for_end");
                writeln!(self.body, "  br label %{head}").ok();
                writeln!(self.body, "{head}:").ok();
                if let Some(t) = test {
                    let cond = self.emit_to_boolean(t)?;
                    writeln!(self.body, "  br i1 {cond}, label %{bod}, label %{end}").ok();
                } else {
                    writeln!(self.body, "  br label %{bod}").ok();
                }
                writeln!(self.body, "{bod}:").ok();
                self.state.ctrls.push(CtrlFrame {
                    names,
                    break_label: end.clone(),
                    continue_label: Some(upd.clone()),
                });
                self.emit_stmt(body)?;
                self.state.ctrls.pop();
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{upd}").ok();
                }
                writeln!(self.body, "{upd}:").ok();
                if let Some(u) = update {
                    match u.ty() {
                        Type::Number => {
                            let _ = self.emit_number_expr(u)?;
                        }
                        Type::Boolean => {
                            let _ = self.emit_bool_expr(u)?;
                        }
                        Type::String => {
                            let _ = self.emit_string_expr(u)?;
                        }
                        Type::Null => {
                            self.emit_undefined_expr(u)?;
                        }
                        _ => {
                            return Err(diag("internal: unsupported for update ty in es_expr"));
                        }
                    }
                }
                writeln!(self.body, "  br label %{head}").ok();
                writeln!(self.body, "{end}:").ok();
                Ok(())
            }
            Stmt::Switch {
                discriminant,
                cases,
            } => {
                let names = std::mem::take(&mut self.state.pending_names);
                let disc = self.emit_number_expr(discriminant)?;
                let end = self.fresh_label("switch_end");
                let case_labels: Vec<String> = (0..cases.len())
                    .map(|i| self.fresh_label(&format!("case{i}_")))
                    .collect();
                let default_idx = cases.iter().position(|c| c.test.is_none());
                let default_target = default_idx
                    .map(|i| case_labels[i].clone())
                    .unwrap_or_else(|| end.clone());

                // Match chain: first case with Strict Equality (===) on numbers.
                for (i, c) in cases.iter().enumerate() {
                    if let Some(test) = &c.test {
                        let try_l = self.fresh_label("sw_try");
                        writeln!(self.body, "  br label %{try_l}").ok();
                        writeln!(self.body, "{try_l}:").ok();
                        let tv = self.emit_number_expr(test)?;
                        let eq = self.fresh();
                        writeln!(self.body, "  {eq} = fcmp oeq double {disc}, {tv}").ok();
                        let next = self.fresh_label("sw_next");
                        writeln!(
                            self.body,
                            "  br i1 {eq}, label %{}, label %{next}",
                            case_labels[i]
                        )
                        .ok();
                        writeln!(self.body, "{next}:").ok();
                    }
                }
                writeln!(self.body, "  br label %{default_target}").ok();

                self.state.ctrls.push(CtrlFrame {
                    names,
                    break_label: end.clone(),
                    continue_label: None,
                });
                for (i, c) in cases.iter().enumerate() {
                    writeln!(self.body, "{}:", case_labels[i]).ok();
                    for s in &c.body {
                        if self.body_ends_with_terminator() {
                            break;
                        }
                        self.emit_stmt(s)?;
                    }
                    if !self.body_ends_with_terminator() {
                        if i + 1 < cases.len() {
                            writeln!(self.body, "  br label %{}", case_labels[i + 1]).ok();
                        } else {
                            writeln!(self.body, "  br label %{end}").ok();
                        }
                    }
                }
                self.state.ctrls.pop();
                writeln!(self.body, "{end}:").ok();
                Ok(())
            }
            Stmt::ForIn { left, right, body } => {
                self.emit_for_in_of(left, right, body, /* is_of */ false)
            }
            Stmt::ForOf {
                left,
                right,
                body,
                is_await,
            } => {
                if *is_await {
                    return Err(diag("internal: for-await-of not in es_expr subset"));
                }
                self.emit_for_in_of(left, right, body, /* is_of */ true)
            }
            Stmt::Labeled { label, body } => match body.as_ref() {
                Stmt::While { .. }
                | Stmt::DoWhile { .. }
                | Stmt::For { .. }
                | Stmt::ForIn { .. }
                | Stmt::ForOf { .. }
                | Stmt::Switch { .. }
                | Stmt::Labeled { .. } => {
                    self.state.pending_names.push(label.clone());
                    self.emit_stmt(body)
                }
                _ => {
                    let end = self.fresh_label("lbl_end");
                    let mut names = std::mem::take(&mut self.state.pending_names);
                    names.push(label.clone());
                    self.state.ctrls.push(CtrlFrame {
                        names,
                        break_label: end.clone(),
                        continue_label: None,
                    });
                    self.emit_stmt(body)?;
                    self.state.ctrls.pop();
                    if !self.body_ends_with_terminator() {
                        writeln!(self.body, "  br label %{end}").ok();
                    }
                    writeln!(self.body, "{end}:").ok();
                    Ok(())
                }
            },
            Stmt::Break { label: None } => {
                let frame = self
                    .state
                    .ctrls
                    .last()
                    .ok_or_else(|| diag("internal: break outside loop/switch in es_expr"))?;
                let end = frame.break_label.clone();
                writeln!(self.body, "  br label %{end}").ok();
                Ok(())
            }
            Stmt::Break { label: Some(name) } => {
                let end = self
                    .state
                    .ctrls
                    .iter()
                    .rev()
                    .find(|f| f.names.iter().any(|n| n == name))
                    .map(|f| f.break_label.clone())
                    .ok_or_else(|| diag("internal: labeled break target missing in es_expr"))?;
                writeln!(self.body, "  br label %{end}").ok();
                Ok(())
            }
            Stmt::Continue { label: None } => {
                let cont = self
                    .state
                    .ctrls
                    .iter()
                    .rev()
                    .find_map(|f| f.continue_label.clone())
                    .ok_or_else(|| diag("internal: continue outside loop in es_expr"))?;
                writeln!(self.body, "  br label %{cont}").ok();
                Ok(())
            }
            Stmt::Continue { label: Some(name) } => {
                let cont = self
                    .state
                    .ctrls
                    .iter()
                    .rev()
                    .find(|f| f.names.iter().any(|n| n == name))
                    .and_then(|f| f.continue_label.clone())
                    .ok_or_else(|| {
                        diag("internal: labeled continue target missing or not iteration")
                    })?;
                writeln!(self.body, "  br label %{cont}").ok();
                Ok(())
            }
            _ => Err(diag("internal: unsupported stmt in es_expr module")),
        }
    }

    /// `for (let k in s)` / `for (let c of s)` over strings (N08.02.08).
    pub(super) fn emit_for_in_of(
        &mut self,
        left: &Stmt,
        right: &Expr,
        body: &Stmt,
        is_of: bool,
    ) -> Result<(), Diagnostic> {
        let names = std::mem::take(&mut self.state.pending_names);
        // Ensure for-in/of `let` binding has an alloca (also collected in classify).
        if let Stmt::Declare { local, init, .. } = left {
            if init.is_some() {
                return Err(diag("internal: for-in/of left declare must not have init"));
            }
            if !self.state.allocas.contains_key(local) {
                let ptr = format!("%l{}", local.0);
                writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                self.state.allocas.insert(*local, (ptr, SlotTy::String));
                let len_ptr = format!("%l{}_len", local.0);
                writeln!(self.body, "  {len_ptr} = alloca i64, align 8").ok();
                self.state.string_lens.insert(*local, len_ptr);
            }
        }
        let s = self.emit_string_expr(right)?;
        let idx_ptr = self.fresh();
        writeln!(self.body, "  {idx_ptr} = alloca i64, align 8").ok();
        writeln!(self.body, "  store i64 0, ptr {idx_ptr}").ok();
        let head = self.fresh_label(if is_of { "forof_head" } else { "forin_head" });
        let bod = self.fresh_label(if is_of { "forof_body" } else { "forin_body" });
        let cont = self.fresh_label(if is_of { "forof_cont" } else { "forin_cont" });
        let end = self.fresh_label(if is_of { "forof_end" } else { "forin_end" });
        writeln!(self.body, "  br label %{head}").ok();
        writeln!(self.body, "{head}:").ok();
        let idx = self.fresh();
        writeln!(self.body, "  {idx} = load i64, ptr {idx_ptr}").ok();
        // N08.07.05: for-in/of over strings iterates UTF-16 code units.
        let units = self.fresh();
        writeln!(
            self.body,
            "  {}",
            UTF16_LEN.call_to(&units, &format!("ptr {}, i64 {}", s.data, s.len))
        )
        .ok();
        let cmp = self.fresh();
        writeln!(self.body, "  {cmp} = icmp ult i64 {idx}, {units}").ok();
        writeln!(self.body, "  br i1 {cmp}, label %{bod}, label %{end}").ok();
        writeln!(self.body, "{bod}:").ok();
        let bound = if is_of {
            let out_len_ptr = self.fresh();
            writeln!(self.body, "  {out_len_ptr} = alloca i64, align 8").ok();
            let ch = self.fresh();
            writeln!(
                self.body,
                "  {}",
                CSTR_FROM_CODE_UNIT_N.call_to(
                    &ch,
                    &format!(
                        "ptr {}, i64 {}, i64 {idx}, ptr {out_len_ptr}",
                        s.data, s.len
                    )
                )
            )
            .ok();
            let ch_len = self.fresh();
            writeln!(self.body, "  {ch_len} = load i64, ptr {out_len_ptr}").ok();
            StrVal {
                data: ch,
                len: ch_len,
            }
        } else {
            let key = self.fresh();
            writeln!(
                self.body,
                "  {}",
                CSTR_FROM_U64.call_to(&key, &format!("i64 {idx}"))
            )
            .ok();
            let key_len = self.fresh();
            writeln!(
                self.body,
                "  {}",
                CSTR_LEN.call_to(&key_len, &format!("ptr {key}"))
            )
            .ok();
            StrVal {
                data: key,
                len: key_len,
            }
        };
        self.store_for_in_of_left(left, &bound)?;
        self.state.ctrls.push(CtrlFrame {
            names,
            break_label: end.clone(),
            continue_label: Some(cont.clone()),
        });
        self.emit_stmt(body)?;
        self.state.ctrls.pop();
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

    pub(super) fn store_for_in_of_left(
        &mut self,
        left: &Stmt,
        value: &StrVal,
    ) -> Result<(), Diagnostic> {
        let id = match left {
            Stmt::Declare { local, .. } => *local,
            Stmt::Expr {
                expr: Expr::Local { id, .. },
            } => *id,
            _ => return Err(diag("internal: unsupported for-in/of left")),
        };
        self.store_string_local(id, value)
    }

    pub(super) fn store_string_local(
        &mut self,
        id: LocalId,
        value: &StrVal,
    ) -> Result<(), Diagnostic> {
        let (ptr, slot) = self
            .state
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag(format!("internal: unallocated string local %{}", id.0)))?;
        if slot != SlotTy::String {
            return Err(diag("internal: expected string slot"));
        }
        let len_ptr = self
            .state
            .string_lens
            .get(&id)
            .cloned()
            .ok_or_else(|| diag(format!("internal: missing string len alloca %{}", id.0)))?;
        writeln!(self.body, "  store ptr {}, ptr {ptr}", value.data).ok();
        writeln!(self.body, "  store i64 {}, ptr {len_ptr}", value.len).ok();
        Ok(())
    }

    pub(super) fn load_string_local(&mut self, id: LocalId) -> Result<StrVal, Diagnostic> {
        let (ptr, slot) = self
            .state
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag(format!("internal: unallocated local %{}", id.0)))?;
        if slot != SlotTy::String {
            return Err(diag("internal: expected string local"));
        }
        let len_ptr = self
            .state
            .string_lens
            .get(&id)
            .cloned()
            .ok_or_else(|| diag(format!("internal: missing string len %{}", id.0)))?;
        let data = self.fresh();
        writeln!(self.body, "  {data} = load ptr, ptr {ptr}").ok();
        let len = self.fresh();
        writeln!(self.body, "  {len} = load i64, ptr {len_ptr}").ok();
        Ok(StrVal { data, len })
    }

    pub(super) fn string_const(&mut self, s: &str) -> Result<StrVal, Diagnostic> {
        self.string_const_bytes(s.as_bytes())
    }

    pub(super) fn string_const_js(&mut self, value: &JsString) -> Result<StrVal, Diagnostic> {
        let bytes = jsstring_to_wtf8(value);
        self.string_const_bytes(&bytes)
    }

    pub(super) fn string_const_bytes(&mut self, bytes: &[u8]) -> Result<StrVal, Diagnostic> {
        let gname = if let Some(g) = self.state.str_globals.get(bytes) {
            g.clone()
        } else {
            let g = format!(".str.{}", self.state.str_globals.len());
            self.state.str_globals.insert(bytes.to_vec(), g.clone());
            g
        };
        let t = self.fresh();
        let n = bytes.len() + 1;
        writeln!(
            self.body,
            "  {t} = getelementptr inbounds [{n} x i8], ptr @{gname}, i64 0, i64 0"
        )
        .ok();
        Ok(StrVal {
            data: t,
            len: format!("{}", bytes.len()),
        })
    }

    /// Evaluate a typeof/void/delete operand for side effects only.
    pub(super) fn emit_discard_arg(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            // `null` literal and pure undefined locals have no side effects.
            Expr::Null { .. } => Ok(()),
            Expr::Number { .. } => {
                let _ = self.emit_number_expr(expr)?;
                Ok(())
            }
            Expr::BigInt { .. } => {
                let _ = self.emit_bigint_expr(expr)?;
                Ok(())
            }
            Expr::Boolean { .. } => {
                let _ = self.emit_bool_expr(expr)?;
                Ok(())
            }
            Expr::String { .. } => {
                let _ = self.emit_string_expr(expr)?;
                Ok(())
            }
            Expr::Local { id, ty } => {
                // N08.08.05–06: host globals are not stack-allocated.
                if *ty == Type::Object
                    && self
                        .module
                        .locals
                        .iter()
                        .any(|l| l.id == *id && l.name == "Math")
                {
                    return Ok(());
                }
                if *ty == Type::Function
                    && self
                        .module
                        .locals
                        .iter()
                        .any(|l| l.id == *id && l.name == "Number")
                {
                    return Ok(());
                }
                if *ty == Type::Number
                    && self
                        .module
                        .locals
                        .iter()
                        .any(|l| l.id == *id && (l.name == "NaN" || l.name == "Infinity"))
                {
                    return Ok(());
                }
                let (_, slot) = self.state.allocas.get(id).cloned().ok_or_else(|| {
                    diag(format!("internal: unallocated discard local %{}", id.0))
                })?;
                match slot {
                    SlotTy::Number => {
                        let _ = self.emit_number_expr(expr)?;
                    }
                    SlotTy::BigInt => {
                        let _ = self.emit_bigint_expr(expr)?;
                    }
                    SlotTy::Boolean => {
                        let _ = self.emit_bool_expr(expr)?;
                    }
                    SlotTy::String => {
                        let _ = self.emit_string_expr(expr)?;
                    }
                    SlotTy::Undefined => {}
                }
                Ok(())
            }
            e if matches!(e.ty(), Type::Number) => {
                let _ = self.emit_number_expr(e)?;
                Ok(())
            }
            e if matches!(e.ty(), Type::BigInt) => {
                let _ = self.emit_bigint_expr(e)?;
                Ok(())
            }
            e if matches!(e.ty(), Type::Boolean) => {
                let _ = self.emit_bool_expr(e)?;
                Ok(())
            }
            e if matches!(e.ty(), Type::String) => {
                let _ = self.emit_string_expr(e)?;
                Ok(())
            }
            e if matches!(e.ty(), Type::Null) => self.emit_undefined_expr(e),
            _ => Err(diag("internal: unsupported typeof/void/delete arg")),
        }
    }

    pub(super) fn typeof_name(
        expr: &Expr,
        slot_of: &HashMap<LocalId, SlotTy>,
        module: &Module,
    ) -> Option<&'static str> {
        match expr {
            Expr::Number { .. } => Some("number"),
            Expr::BigInt { .. } => Some("bigint"),
            Expr::String { .. } => Some("string"),
            Expr::Boolean { .. } => Some("boolean"),
            Expr::Null { .. } => Some("object"),
            Expr::Local { id, .. } => {
                if module
                    .locals
                    .iter()
                    .any(|l| l.id == *id && l.name == "Math")
                {
                    return Some("object");
                }
                if module
                    .locals
                    .iter()
                    .any(|l| l.id == *id && l.name == "Number")
                {
                    return Some("function");
                }
                if module
                    .locals
                    .iter()
                    .any(|l| l.id == *id && (l.name == "NaN" || l.name == "Infinity"))
                {
                    return Some("number");
                }
                match slot_of.get(id)? {
                    SlotTy::Number => Some("number"),
                    SlotTy::BigInt => Some("bigint"),
                    SlotTy::String => Some("string"), // includes untyped any string slots
                    SlotTy::Boolean => Some("boolean"),
                    SlotTy::Undefined => Some("undefined"),
                }
            }
            Expr::Unary {
                op: UnaryOp::Void, ..
            } => Some("undefined"),
            Expr::Unary {
                op: UnaryOp::TypeOf,
                ..
            } => Some("string"),
            // Number/bool/bigint-producing ops → type name strings
            e if matches!(e.ty(), Type::Number) => Some("number"),
            e if matches!(e.ty(), Type::BigInt) => Some("bigint"),
            e if matches!(e.ty(), Type::Boolean) => Some("boolean"),
            e if matches!(e.ty(), Type::String) => Some("string"),
            e if matches!(e.ty(), Type::Null) => Some("undefined"),
            _ => None,
        }
    }
}
