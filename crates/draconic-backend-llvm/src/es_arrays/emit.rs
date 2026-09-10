use std::fmt::Write as _;

use super::ok::*;
use super::*;
use crate::emitter::escape_llvm_string;

impl<'a> super::Emitter<'a> {
    pub(super) fn new(module: &'a Module, _info: &'a ModuleInfo) -> Self {
        Self {
            module,
            out: String::new(),
            body: String::new(),
            allocas: HashMap::new(),
            slot_of: HashMap::new(),
            str_globals: Vec::new(),
            tmp: 0,
            str_n: 0,
            ctrls: Vec::new(),
        }
    }

    pub(super) fn finish(self) -> String {
        self.out
    }

    pub(super) fn fresh(&mut self) -> String {
        let t = self.tmp;
        self.tmp += 1;
        format!("%t{t}")
    }

    pub(super) fn fresh_label(&mut self, prefix: &str) -> String {
        let t = self.tmp;
        self.tmp += 1;
        format!("{prefix}{t}")
    }

    pub(super) fn body_ends_with_terminator(&self) -> bool {
        self.body
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .is_some_and(|l| {
                let t = l.trim_start();
                t.starts_with("br ")
                    || t.starts_with("ret ")
                    || t.starts_with("unreachable")
                    || t.starts_with("switch ")
                    || t.starts_with("indirectbr ")
            })
    }

    pub(super) fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for (id, ty) in &info.slots {
            self.slot_of.insert(*id, *ty);
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.06 ES arrays via Runtime ABI)"
        )
        .ok();
        writeln!(
            self.out,
            "{}",
            llvm_declares(&[
                GC_INIT,
                ARRAY_NEW,
                ARRAY_GET,
                ARRAY_SET,
                ARRAY_LEN,
                ARRAY_SPREAD_ARRAY,
                ARRAY_SPREAD_CSTR,
                CSTR_CONCAT,
                ALLOC_OBJECT,
                OBJECT_GET,
                OBJECT_SET,
                PRINT_F64,
                PRINT_STR,
            ])
        )
        .ok();
        writeln!(self.out).ok();

        for (id, kind) in &info.slots {
            match kind {
                SlotTy::Number => {
                    let g = number_global_name(*id);
                    writeln!(
                        self.out,
                        "@{g} = internal global double 0.00000000000000000e+00, align 8"
                    )
                    .ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
                SlotTy::String | SlotTy::Bool | SlotTy::Null | SlotTy::Array | SlotTy::Object => {
                    let g = ptr_global_name(*id, *kind);
                    writeln!(self.out, "@{g} = internal global ptr null, align 8").ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
            }
        }
        if !info.slots.is_empty() {
            writeln!(self.out).ok();
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for (id, kind) in &info.print_locals {
            let ptr = self.slot_ptr(*id)?;
            match kind {
                SlotTy::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                }
                SlotTy::String => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
                _ => {}
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
                let Some(init) = init else {
                    return Ok(());
                };
                let kind = *self
                    .slot_of
                    .get(local)
                    .ok_or_else(|| diag("es_arrays: declare unknown slot"))?;
                let ptr = self.slot_ptr(*local)?;
                match kind {
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Array => {
                        let v = self.emit_array_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::String => {
                        let v = self.emit_string_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Bool => {
                        let v = self.emit_bool_as_ptr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Null => {
                        let v = self.emit_null_as_ptr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Object => {
                        let v = self.emit_object_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Stmt::DeclareArrayPattern {
                elements,
                init: Some(init),
                ..
            } => {
                let arr = self.emit_array_expr(init)?;
                self.emit_array_destructure(elements, &arr)
            }
            Stmt::Expr { expr } => {
                if let Expr::Assign {
                    target: AssignTarget::ArrayPattern { elements },
                    op: AssignOp::Eq,
                    value,
                    ..
                } = expr
                {
                    let arr = self.emit_array_expr(value)?;
                    return self.emit_array_destructure(elements, &arr);
                }
                if matches!(
                    expr,
                    Expr::Assign {
                        target: AssignTarget::Local(_),
                        ..
                    }
                ) {
                    self.emit_local_assign(expr)
                } else {
                    let _ = self.emit_member_assign(expr, false)?;
                    Ok(())
                }
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_stmt(s)?;
                }
                Ok(())
            }
            Stmt::ForOf {
                left,
                right,
                body,
                is_await,
            } => {
                if *is_await {
                    return Err(diag("es_arrays: for-await-of not supported"));
                }
                self.emit_for_of(left, right, body)
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => self.emit_if(test, consequent, alternate.as_deref()),
            Stmt::Break { label: None } => {
                let end = self
                    .ctrls
                    .last()
                    .ok_or_else(|| diag("es_arrays: break outside loop"))?
                    .break_label
                    .clone();
                writeln!(self.body, "  br label %{end}").ok();
                Ok(())
            }
            Stmt::Continue { label: None } => {
                let cont = self
                    .ctrls
                    .iter()
                    .rev()
                    .find_map(|f| f.continue_label.clone())
                    .ok_or_else(|| diag("es_arrays: continue outside loop"))?;
                writeln!(self.body, "  br label %{cont}").ok();
                Ok(())
            }
            _ => Err(diag("es_arrays: unsupported stmt")),
        }
    }

    pub(super) fn emit_if(
        &mut self,
        test: &Expr,
        consequent: &Stmt,
        alternate: Option<&Stmt>,
    ) -> Result<(), Diagnostic> {
        let cond = self.emit_cmp_i1(test)?;
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

    pub(super) fn emit_cmp_i1(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        let Expr::Binary {
            left, op, right, ..
        } = expr
        else {
            return Err(diag("es_arrays: if test must be comparison"));
        };
        let l = self.emit_number_expr(left)?;
        let r = self.emit_number_expr(right)?;
        let pred = match op {
            BinaryOp::EqEq | BinaryOp::EqEqEq => "oeq",
            BinaryOp::NotEq | BinaryOp::NotEqEq => "one",
            _ => return Err(diag("es_arrays: unsupported comparison")),
        };
        let t = self.fresh();
        writeln!(self.body, "  {t} = fcmp {pred} double {l}, {r}").ok();
        Ok(t)
    }

    /// Destructure `arr` into `elements` (declare or assign pattern).
    pub(super) fn emit_array_destructure(
        &mut self,
        elements: &[ArrayPatternEl],
        arr: &str,
    ) -> Result<(), Diagnostic> {
        let idx_ptr = self.fresh();
        writeln!(self.body, "  {idx_ptr} = alloca i64, align 8").ok();
        writeln!(self.body, "  store i64 0, ptr {idx_ptr}").ok();

        for el in elements {
            match el {
                ArrayPatternEl::Elision => {
                    let i = self.fresh();
                    writeln!(self.body, "  {i} = load i64, ptr {idx_ptr}").ok();
                    let n = self.fresh();
                    writeln!(self.body, "  {n} = add i64 {i}, 1").ok();
                    writeln!(self.body, "  store i64 {n}, ptr {idx_ptr}").ok();
                }
                ArrayPatternEl::Pattern { binding, default } => {
                    let i = self.fresh();
                    writeln!(self.body, "  {i} = load i64, ptr {idx_ptr}").ok();
                    let len = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_LEN.call_to(&len, &format!("ptr {arr}"))
                    )
                    .ok();
                    let in_range = self.fresh();
                    writeln!(self.body, "  {in_range} = icmp ult i64 {i}, {len}").ok();
                    let got = self.fresh();
                    writeln!(self.body, "  {got} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  store ptr null, ptr {got}").ok();
                    let take_l = self.fresh_label("dstr_take");
                    let def_l = self.fresh_label("dstr_def");
                    let done_l = self.fresh_label("dstr_done");
                    writeln!(
                        self.body,
                        "  br i1 {in_range}, label %{take_l}, label %{def_l}"
                    )
                    .ok();
                    writeln!(self.body, "{take_l}:").ok();
                    let raw = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_GET.call_to(&raw, &format!("ptr {arr}, i64 {i}"))
                    )
                    .ok();
                    // Hole / undefined → null ptr; treat as missing for defaults.
                    let is_null = self.fresh();
                    writeln!(self.body, "  {is_null} = icmp eq ptr {raw}, null").ok();
                    let use_l = self.fresh_label("dstr_use");
                    writeln!(
                        self.body,
                        "  br i1 {is_null}, label %{def_l}, label %{use_l}"
                    )
                    .ok();
                    writeln!(self.body, "{use_l}:").ok();
                    writeln!(self.body, "  store ptr {raw}, ptr {got}").ok();
                    writeln!(self.body, "  br label %{done_l}").ok();
                    writeln!(self.body, "{def_l}:").ok();
                    if let Some(d) = default {
                        let dv = self.emit_value_as_ptr(d)?;
                        writeln!(self.body, "  store ptr {dv}, ptr {got}").ok();
                    }
                    writeln!(self.body, "  br label %{done_l}").ok();
                    writeln!(self.body, "{done_l}:").ok();
                    let val = self.fresh();
                    writeln!(self.body, "  {val} = load ptr, ptr {got}").ok();
                    self.emit_bind_pattern(binding, &val)?;
                    let i2 = self.fresh();
                    writeln!(self.body, "  {i2} = load i64, ptr {idx_ptr}").ok();
                    let n = self.fresh();
                    writeln!(self.body, "  {n} = add i64 {i2}, 1").ok();
                    writeln!(self.body, "  store i64 {n}, ptr {idx_ptr}").ok();
                }
                ArrayPatternEl::Rest(binding) => {
                    let i = self.fresh();
                    writeln!(self.body, "  {i} = load i64, ptr {idx_ptr}").ok();
                    let len = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_LEN.call_to(&len, &format!("ptr {arr}"))
                    )
                    .ok();
                    // rest_len = max(0, len - i)
                    let ge = self.fresh();
                    writeln!(self.body, "  {ge} = icmp uge i64 {len}, {i}").ok();
                    let diff = self.fresh();
                    writeln!(self.body, "  {diff} = sub i64 {len}, {i}").ok();
                    let rest_len = self.fresh();
                    writeln!(
                        self.body,
                        "  {rest_len} = select i1 {ge}, i64 {diff}, i64 0"
                    )
                    .ok();
                    let rest = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_NEW.call_to(&rest, &format!("i64 {rest_len}"))
                    )
                    .ok();
                    // Copy arr[i..] into rest[0..]
                    let j_ptr = self.fresh();
                    writeln!(self.body, "  {j_ptr} = alloca i64, align 8").ok();
                    writeln!(self.body, "  store i64 0, ptr {j_ptr}").ok();
                    let head = self.fresh_label("rest_head");
                    let body = self.fresh_label("rest_body");
                    let end = self.fresh_label("rest_end");
                    writeln!(self.body, "  br label %{head}").ok();
                    writeln!(self.body, "{head}:").ok();
                    let j = self.fresh();
                    writeln!(self.body, "  {j} = load i64, ptr {j_ptr}").ok();
                    let cmp = self.fresh();
                    writeln!(self.body, "  {cmp} = icmp ult i64 {j}, {rest_len}").ok();
                    writeln!(self.body, "  br i1 {cmp}, label %{body}, label %{end}").ok();
                    writeln!(self.body, "{body}:").ok();
                    let src_i = self.fresh();
                    writeln!(self.body, "  {src_i} = add i64 {i}, {j}").ok();
                    let elv = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_GET.call_to(&elv, &format!("ptr {arr}, i64 {src_i}"))
                    )
                    .ok();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_SET.call(&format!("ptr {rest}, i64 {j}, ptr {elv}"))
                    )
                    .ok();
                    let jn = self.fresh();
                    writeln!(self.body, "  {jn} = add i64 {j}, 1").ok();
                    writeln!(self.body, "  store i64 {jn}, ptr {j_ptr}").ok();
                    writeln!(self.body, "  br label %{head}").ok();
                    writeln!(self.body, "{end}:").ok();
                    self.emit_bind_pattern(binding, &rest)?;
                    // Rest consumes the remainder; advance idx to len.
                    writeln!(self.body, "  store i64 {len}, ptr {idx_ptr}").ok();
                }
            }
        }
        Ok(())
    }

    pub(super) fn emit_bind_pattern(&mut self, binding: &Pattern, val_ptr: &str) -> Result<(), Diagnostic> {
        match binding {
            Pattern::Local(id) => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_arrays: pattern local unknown slot"))?;
                let ptr = self.slot_ptr(*id)?;
                match kind {
                    SlotTy::Number => {
                        let i = self.fresh();
                        writeln!(self.body, "  {i} = ptrtoint ptr {val_ptr} to i64").ok();
                        let d = self.fresh();
                        writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                        writeln!(self.body, "  store double {d}, ptr {ptr}").ok();
                    }
                    SlotTy::Array
                    | SlotTy::String
                    | SlotTy::Bool
                    | SlotTy::Null
                    | SlotTy::Object => {
                        writeln!(self.body, "  store ptr {val_ptr}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Pattern::Member {
                object,
                property,
                computed,
            } => {
                let obj = self.emit_object_expr(object)?;
                let key = if *computed {
                    if matches!(property.as_ref(), Expr::String { .. })
                        || self.expr_is_string_slot(property)
                    {
                        self.emit_string_expr(property)?
                    } else {
                        return Err(diag(
                            "es_arrays: computed member pattern key must be string",
                        ));
                    }
                } else {
                    let s = member_key_string(property)
                        .ok_or_else(|| diag("es_arrays: member pattern key"))?;
                    self.string_const(&s)?
                };
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {val_ptr}"))
                )
                .ok();
                Ok(())
            }
            Pattern::Array(inner) => {
                // Nested array pattern: val_ptr is the array to destructure.
                self.emit_array_destructure(inner, val_ptr)
            }
            Pattern::Name(_) | Pattern::Object(_) => {
                Err(diag("es_arrays: unsupported pattern binding"))
            }
        }
    }

    pub(super) fn emit_object_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Object { properties, .. } => {
                if !properties.is_empty() {
                    return Err(diag("es_arrays: only empty object literals"));
                }
                let t = self.fresh();
                writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&t, "")).ok();
                Ok(t)
            }
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_arrays: object local unknown"))?;
                if kind != SlotTy::Object {
                    return Err(diag("es_arrays: expected object local"));
                }
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Member {
                object,
                property,
                optional,
                computed,
                ..
            } => {
                if *optional {
                    return Err(diag("es_arrays: optional object member"));
                }
                let obj = self.emit_object_expr(object)?;
                let key = if *computed {
                    self.emit_string_expr(property)?
                } else {
                    let s = member_key_string(property)
                        .ok_or_else(|| diag("es_arrays: object member key"))?;
                    self.string_const(&s)?
                };
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_GET.call_to(&t, &format!("ptr {obj}, ptr {key}"))
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag("es_arrays: unsupported object expr")),
        }
    }

    pub(super) fn emit_for_of(&mut self, left: &Stmt, right: &Expr, body: &Stmt) -> Result<(), Diagnostic> {
        let bind_id = match left {
            Stmt::Declare {
                local, init: None, ..
            } => *local,
            Stmt::Expr {
                expr: Expr::Local { id, .. },
            } => *id,
            _ => return Err(diag("es_arrays: unsupported for-of left")),
        };
        let bind_kind = *self
            .slot_of
            .get(&bind_id)
            .ok_or_else(|| diag("es_arrays: for-of bind unknown slot"))?;
        let bind_ptr = self.slot_ptr(bind_id)?;

        let arr = self.emit_array_expr(right)?;
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
        let len = self.fresh();
        writeln!(
            self.body,
            "  {}",
            ARRAY_LEN.call_to(&len, &format!("ptr {arr}"))
        )
        .ok();
        let cmp = self.fresh();
        writeln!(self.body, "  {cmp} = icmp ult i64 {idx}, {len}").ok();
        writeln!(self.body, "  br i1 {cmp}, label %{bod}, label %{end}").ok();
        writeln!(self.body, "{bod}:").ok();
        let elem = self.fresh();
        writeln!(
            self.body,
            "  {}",
            ARRAY_GET.call_to(&elem, &format!("ptr {arr}, i64 {idx}"))
        )
        .ok();
        match bind_kind {
            SlotTy::Number => {
                let i = self.fresh();
                writeln!(self.body, "  {i} = ptrtoint ptr {elem} to i64").ok();
                let d = self.fresh();
                writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                writeln!(self.body, "  store double {d}, ptr {bind_ptr}").ok();
            }
            SlotTy::String | SlotTy::Array | SlotTy::Bool | SlotTy::Null | SlotTy::Object => {
                writeln!(self.body, "  store ptr {elem}, ptr {bind_ptr}").ok();
            }
        }
        self.ctrls.push(CtrlFrame {
            break_label: end.clone(),
            continue_label: Some(cont.clone()),
        });
        self.emit_stmt(body)?;
        self.ctrls.pop();
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

    pub(super) fn emit_local_assign(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        let Expr::Assign {
            target: AssignTarget::Local(id),
            op: AssignOp::Eq,
            value,
            ..
        } = expr
        else {
            return Err(diag("es_arrays: expected local assign"));
        };
        let kind = *self
            .slot_of
            .get(id)
            .ok_or_else(|| diag("es_arrays: assign unknown slot"))?;
        let ptr = self.slot_ptr(*id)?;
        match kind {
            SlotTy::Number => {
                let v = self.emit_number_expr(value)?;
                writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
            }
            SlotTy::String => {
                let v = self.emit_string_expr(value)?;
                writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
            }
            SlotTy::Array => {
                let v = self.emit_array_expr(value)?;
                writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
            }
            SlotTy::Bool => {
                let v = self.emit_bool_as_ptr(value)?;
                writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
            }
            SlotTy::Null => {
                let v = self.emit_null_as_ptr(value)?;
                writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
            }
            SlotTy::Object => {
                let v = self.emit_object_expr(value)?;
                writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
            }
        }
        Ok(())
    }

    /// Emit `a[i] = v` (or nested). When `yield_number`, returns the RHS as double.
    pub(super) fn emit_member_assign(
        &mut self,
        expr: &Expr,
        yield_number: bool,
    ) -> Result<String, Diagnostic> {
        let Expr::Assign {
            target:
                AssignTarget::Member {
                    object,
                    property,
                    computed: true,
                    ..
                },
            op: AssignOp::Eq,
            value,
            ..
        } = expr
        else {
            return Err(diag("es_arrays: expected computed member assign"));
        };
        let arr = self.emit_array_expr(object)?;
        let idx_d = self.emit_number_expr(property)?;
        let idx_i = self.fresh();
        writeln!(self.body, "  {idx_i} = fptosi double {idx_d} to i64").ok();
        if yield_number {
            let n = self.emit_number_expr(value)?;
            let i = self.fresh();
            writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
            let p = self.fresh();
            writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
            writeln!(
                self.body,
                "  {}",
                ARRAY_SET.call(&format!("ptr {arr}, i64 {idx_i}, ptr {p}"))
            )
            .ok();
            Ok(n)
        } else {
            let v = self.emit_value_as_ptr(value)?;
            writeln!(
                self.body,
                "  {}",
                ARRAY_SET.call(&format!("ptr {arr}, i64 {idx_i}, ptr {v}"))
            )
            .ok();
            Ok(String::new())
        }
    }
}
