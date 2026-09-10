use std::fmt::Write as _;

use super::ok::*;
use super::*;

impl<'a> super::Emitter<'a> {
    pub(super) fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => format_number_const(raw),
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_arrays: number local unknown"))?;
                if kind != LocalSlot::Number {
                    return Err(diag("es_arrays: expected number local"));
                }
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
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
                    return Err(diag("es_arrays: optional member not supported"));
                }
                // Object property number read: mem.y / mem["y"]
                if self.expr_is_object_slot(object) {
                    let obj = self.emit_object_expr(object)?;
                    let key = if *computed {
                        self.emit_string_expr(property)?
                    } else {
                        let s = member_key_string(property)
                            .ok_or_else(|| diag("es_arrays: object number prop key"))?;
                        self.string_const(&s)?
                    };
                    let raw = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        OBJECT_GET.call_to(&raw, &format!("ptr {obj}, ptr {key}"))
                    )
                    .ok();
                    let i = self.fresh();
                    writeln!(self.body, "  {i} = ptrtoint ptr {raw} to i64").ok();
                    let d = self.fresh();
                    writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                    return Ok(d);
                }
                if *computed {
                    let arr = self.emit_array_expr(object)?;
                    let idx_d = self.emit_number_expr(property)?;
                    let idx_i = self.fresh();
                    writeln!(self.body, "  {idx_i} = fptosi double {idx_d} to i64").ok();
                    let raw = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_GET.call_to(&raw, &format!("ptr {arr}, i64 {idx_i}"))
                    )
                    .ok();
                    let i = self.fresh();
                    writeln!(self.body, "  {i} = ptrtoint ptr {raw} to i64").ok();
                    let d = self.fresh();
                    writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                    Ok(d)
                } else if member_key_is_length(property) {
                    let arr = self.emit_array_expr(object)?;
                    let n = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_LEN.call_to(&n, &format!("ptr {arr}"))
                    )
                    .ok();
                    let d = self.fresh();
                    writeln!(self.body, "  {d} = sitofp i64 {n} to double").ok();
                    Ok(d)
                } else {
                    Err(diag("es_arrays: only .length or computed index on arrays"))
                }
            }
            Expr::Assign {
                target: AssignTarget::Member { .. },
                ..
            } => self.emit_member_assign(expr, true),
            Expr::Binary {
                left, op, right, ..
            } if matches!(
                op,
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
            ) =>
            {
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let inst = match op {
                    BinaryOp::Add => "fadd",
                    BinaryOp::Sub => "fsub",
                    BinaryOp::Mul => "fmul",
                    BinaryOp::Div => "fdiv",
                    BinaryOp::Rem => "frem",
                    _ => unreachable!(),
                };
                let t = self.fresh();
                writeln!(self.body, "  {t} = {inst} double {l}, {r}").ok();
                Ok(t)
            }
            _ => Err(diag("es_arrays: unsupported number expr")),
        }
    }

    pub(super) fn emit_array_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Array { elements, .. } => self.emit_array_lit(elements),
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_arrays: array local unknown"))?;
                if kind != LocalSlot::Array {
                    return Err(diag("es_arrays: expected array local"));
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
                    return Err(diag("es_arrays: optional member not supported"));
                }
                // Object property array: restMem.arr
                if self.expr_is_object_slot(object) {
                    let obj = self.emit_object_expr(object)?;
                    let key = if *computed {
                        self.emit_string_expr(property)?
                    } else {
                        let s = member_key_string(property)
                            .ok_or_else(|| diag("es_arrays: object array prop key"))?;
                        self.string_const(&s)?
                    };
                    let t = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        OBJECT_GET.call_to(&t, &format!("ptr {obj}, ptr {key}"))
                    )
                    .ok();
                    return Ok(t);
                }
                if !*computed {
                    return Err(diag("es_arrays: nested array only via computed index"));
                }
                let arr = self.emit_array_expr(object)?;
                let idx_d = self.emit_number_expr(property)?;
                let idx_i = self.fresh();
                writeln!(self.body, "  {idx_i} = fptosi double {idx_d} to i64").ok();
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    ARRAY_GET.call_to(&t, &format!("ptr {arr}, i64 {idx_i}"))
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag("es_arrays: unsupported array expr")),
        }
    }

    pub(super) fn emit_array_lit(
        &mut self,
        elements: &[ArrayElement],
    ) -> Result<String, Diagnostic> {
        let has_spread = elements
            .iter()
            .any(|el| matches!(el, ArrayElement::Spread(_)));
        if !has_spread {
            let n = elements.len();
            let arr = self.fresh();
            writeln!(
                self.body,
                "  {}",
                ARRAY_NEW.call_to(&arr, &format!("i64 {n}"))
            )
            .ok();
            for (i, el) in elements.iter().enumerate() {
                match el {
                    ArrayElement::Elision => {}
                    ArrayElement::Spread(_) => unreachable!(),
                    ArrayElement::Expr(e) => {
                        let v = self.emit_value_as_ptr(e)?;
                        writeln!(
                            self.body,
                            "  {}",
                            ARRAY_SET.call(&format!("ptr {arr}, i64 {i}, ptr {v}"))
                        )
                        .ok();
                    }
                }
            }
            return Ok(arr);
        }

        // Spread path: grow from empty via ARRAY_SET / ARRAY_SPREAD_*.
        let arr = self.fresh();
        writeln!(self.body, "  {}", ARRAY_NEW.call_to(&arr, "i64 0")).ok();
        for el in elements {
            match el {
                ArrayElement::Elision => {
                    let len = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_LEN.call_to(&len, &format!("ptr {arr}"))
                    )
                    .ok();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_SET.call(&format!("ptr {arr}, i64 {len}, ptr null"))
                    )
                    .ok();
                }
                ArrayElement::Expr(e) => {
                    let v = self.emit_value_as_ptr(e)?;
                    let len = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_LEN.call_to(&len, &format!("ptr {arr}"))
                    )
                    .ok();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_SET.call(&format!("ptr {arr}, i64 {len}, ptr {v}"))
                    )
                    .ok();
                }
                ArrayElement::Spread(e) => {
                    if self.expr_is_string_slot(e) || matches!(e, Expr::String { .. }) {
                        let s = self.emit_string_expr(e)?;
                        writeln!(
                            self.body,
                            "  {}",
                            ARRAY_SPREAD_CSTR.call(&format!("ptr {arr}, ptr {s}"))
                        )
                        .ok();
                    } else {
                        let src = self.emit_array_expr(e)?;
                        writeln!(
                            self.body,
                            "  {}",
                            ARRAY_SPREAD_ARRAY.call(&format!("ptr {arr}, ptr {src}"))
                        )
                        .ok();
                    }
                }
            }
        }
        Ok(arr)
    }

    pub(super) fn emit_value_as_ptr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        if matches!(expr, Expr::Array { .. }) || self.expr_is_array_slot(expr) {
            return self.emit_array_expr(expr);
        }
        if matches!(expr, Expr::Object { .. }) || self.expr_is_object_slot(expr) {
            return self.emit_object_expr(expr);
        }
        if matches!(expr, Expr::String { .. }) || self.expr_is_string_slot(expr) {
            return self.emit_string_expr(expr);
        }
        if matches!(expr, Expr::Boolean { .. }) || self.expr_is_bool_slot(expr) {
            return self.emit_bool_as_ptr(expr);
        }
        if matches!(expr, Expr::Null { .. })
            || self.expr_is_null_slot(expr)
            || is_undefined_expr(expr)
            || matches!(
                expr,
                Expr::Local { id, .. } if self
                    .module
                    .locals
                    .iter()
                    .any(|l| l.id == *id && l.name == "undefined")
            )
        {
            return self.emit_null_as_ptr(&Expr::Null { ty: Type::Null });
        }
        let n = self.emit_number_expr(expr)?;
        let i = self.fresh();
        writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
        let p = self.fresh();
        writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
        Ok(p)
    }

    pub(super) fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_arrays: string local unknown"))?;
                if kind != LocalSlot::String {
                    return Err(diag("es_arrays: expected string local"));
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
                if *optional || !*computed {
                    return Err(diag("es_arrays: string element only via computed index"));
                }
                let arr = self.emit_array_expr(object)?;
                let idx_d = self.emit_number_expr(property)?;
                let idx_i = self.fresh();
                writeln!(self.body, "  {idx_i} = fptosi double {idx_d} to i64").ok();
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    ARRAY_GET.call_to(&t, &format!("ptr {arr}, i64 {idx_i}"))
                )
                .ok();
                Ok(t)
            }
            Expr::Binary {
                left,
                op: BinaryOp::Add,
                right,
                ..
            } => {
                let l = self.emit_string_expr(left)?;
                let r = self.emit_string_expr(right)?;
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    CSTR_CONCAT.call_to(&t, &format!("ptr {l}, ptr {r}"))
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag("es_arrays: unsupported string expr")),
        }
    }

    pub(super) fn emit_bool_as_ptr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Boolean { value, .. } => {
                let n = if *value { 1i64 } else { 0i64 };
                let t = self.fresh();
                writeln!(self.body, "  {t} = inttoptr i64 {n} to ptr").ok();
                Ok(t)
            }
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_arrays: bool local unknown"))?;
                if kind != LocalSlot::Bool {
                    return Err(diag("es_arrays: expected bool local"));
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
                if *optional || !*computed {
                    return Err(diag("es_arrays: bool element only via computed index"));
                }
                let arr = self.emit_array_expr(object)?;
                let idx_d = self.emit_number_expr(property)?;
                let idx_i = self.fresh();
                writeln!(self.body, "  {idx_i} = fptosi double {idx_d} to i64").ok();
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    ARRAY_GET.call_to(&t, &format!("ptr {arr}, i64 {idx_i}"))
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag("es_arrays: unsupported bool expr")),
        }
    }

    pub(super) fn emit_null_as_ptr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        if matches!(expr, Expr::Null { .. }) || is_undefined_expr(expr) {
            let t = self.fresh();
            writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
            return Ok(t);
        }
        match expr {
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_arrays: null local unknown"))?;
                if kind != LocalSlot::Null {
                    return Err(diag("es_arrays: expected null local"));
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
                if *optional || !*computed {
                    return Err(diag("es_arrays: null element only via computed index"));
                }
                let arr = self.emit_array_expr(object)?;
                let idx_d = self.emit_number_expr(property)?;
                let idx_i = self.fresh();
                writeln!(self.body, "  {idx_i} = fptosi double {idx_d} to i64").ok();
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    ARRAY_GET.call_to(&t, &format!("ptr {arr}, i64 {idx_i}"))
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag("es_arrays: unsupported null expr")),
        }
    }

    pub(super) fn expr_is_array_slot(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Local { id, .. } => self.slot_of.get(id) == Some(&LocalSlot::Array),
            Expr::Member {
                ty, computed: true, ..
            } => matches!(ty, Type::Object | Type::Any),
            _ => false,
        }
    }

    pub(super) fn expr_is_object_slot(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Local { id, .. } => self.slot_of.get(id) == Some(&LocalSlot::Object),
            Expr::Object { .. } => true,
            _ => false,
        }
    }

    pub(super) fn expr_is_string_slot(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Local { id, .. } => self.slot_of.get(id) == Some(&LocalSlot::String),
            Expr::Member { ty, .. } => matches!(ty, Type::String),
            _ => false,
        }
    }

    pub(super) fn expr_is_bool_slot(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Local { id, .. } => self.slot_of.get(id) == Some(&LocalSlot::Bool),
            Expr::Member { ty, .. } => matches!(ty, Type::Boolean),
            _ => false,
        }
    }

    pub(super) fn expr_is_null_slot(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Local { id, .. } => self.slot_of.get(id) == Some(&LocalSlot::Null),
            Expr::Member { ty, .. } => matches!(ty, Type::Null | Type::Any),
            _ => false,
        }
    }

    pub(super) fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        self.allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("es_arrays: slot missing"))
    }

    pub(super) fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let gname = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".es_arr_str.{}", self.str_n);
            self.str_n += 1;
            self.str_globals.push((s.to_string(), g.clone()));
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
}
