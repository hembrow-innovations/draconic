use std::fmt::Write as _;

use super::*;

impl<'a> super::Emitter<'a> {
    pub(super) fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            tmp: 0,
            str_n: 0,
            str_globals: Vec::new(),
            allocas: HashMap::new(),
            slot_of: HashMap::new(),
            fn_local_allocas: HashMap::new(),
            in_fn: false,
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

    pub(super) fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for (id, ty) in &info.slots {
            self.slot_of.insert(*id, *ty);
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.16.19 object destructure via Runtime ABI)"
        )
        .ok();
        writeln!(
            self.out,
            "{}",
            llvm_declares(&[
                GC_INIT,
                ALLOC_OBJECT,
                OBJECT_GET,
                OBJECT_SET,
                OBJECT_COPY_OWN,
                OBJECT_DELETE,
                ARRAY_NEW,
                ARRAY_GET,
                ARRAY_SET,
                ARRAY_LEN,
                PRINT_F64,
                PRINT_STR,
            ])
        )
        .ok();
        writeln!(self.out).ok();

        for (id, kind) in &info.slots {
            match kind {
                SlotTy::Number => {
                    let g = format!("g_num_{}", id.0);
                    writeln!(
                        self.out,
                        "@{g} = internal global double 0.00000000000000000e+00, align 8"
                    )
                    .ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
                SlotTy::String | SlotTy::Object | SlotTy::Array | SlotTy::Function => {
                    let g = format!("g_ptr_{}", id.0);
                    writeln!(self.out, "@{g} = internal global ptr null, align 8").ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
            }
        }
        if !info.slots.is_empty() {
            writeln!(self.out).ok();
        }

        // Emit functions first.
        let fns = info.functions.clone();
        for f in &fns {
            self.emit_fn(f)?;
        }

        // Main body.
        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        // Observations.
        for (id, kind) in &info.print_locals {
            match kind {
                SlotTy::Number => {
                    let ptr = self.slot_ptr(*id)?;
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
                SlotTy::String => {
                    let ptr = self.slot_ptr(*id)?;
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

    pub(super) fn emit_fn(&mut self, f: &FnInfo) -> Result<(), Diagnostic> {
        let mut fn_body = String::new();
        std::mem::swap(&mut self.body, &mut fn_body);
        self.in_fn = true;
        self.fn_local_allocas.clear();

        // Allocas for param locals.
        for &id in &f.param_locals {
            let a = format!("%p_{}", id.0);
            writeln!(self.body, "  {a} = alloca double, align 8").ok();
            // init undefined
            let u = format!("bitcast (i64 {UNDEF_BITS} to double)");
            writeln!(self.body, "  store double {u}, ptr {a}").ok();
            self.fn_local_allocas.insert(id, a);
        }

        // Destructure arg0 into pattern.
        self.emit_object_destructure(&f.param_pattern, "%arg0", /* is_array */ false)?;

        // Body: return local.
        for stmt in &f.body {
            match stmt {
                Stmt::Return {
                    value: Some(Expr::Local { id, .. }),
                } => {
                    let ptr = self
                        .fn_local_allocas
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("es_od: return local missing"))?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    writeln!(self.body, "  ret double {v}").ok();
                }
                _ => return Err(diag("es_od: unsupported fn body stmt")),
            }
        }

        self.in_fn = false;
        let body = std::mem::take(&mut self.body);
        self.body = fn_body;

        writeln!(self.out, "define double @d_fn_{}(ptr %arg0) {{", f.idx).ok();
        writeln!(self.out, "entry:").ok();
        self.out.push_str(&body);
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();
        Ok(())
    }

    pub(super) fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Function { .. } => Ok(()), // emitted separately
            Stmt::Declare { local, init, .. } => {
                let Some(init) = init else {
                    // bare — leave as 0 / undef
                    let kind = *self.slot_of.get(local).unwrap_or(&SlotTy::Number);
                    if kind == SlotTy::Number {
                        let ptr = self.slot_ptr(*local)?;
                        let u = format!("bitcast (i64 {UNDEF_BITS} to double)");
                        writeln!(self.body, "  store double {u}, ptr {ptr}").ok();
                    }
                    return Ok(());
                };
                let kind = *self
                    .slot_of
                    .get(local)
                    .ok_or_else(|| diag("es_od: declare unknown slot"))?;
                match kind {
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::String => {
                        let v = self.emit_string_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Object => {
                        let v = self.emit_object_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Array => {
                        let v = self.emit_array_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Function => {}
                }
                Ok(())
            }
            Stmt::DeclareObjectPattern {
                properties,
                init: Some(init),
                ..
            } => {
                let (src, is_arr) = self.emit_source(init)?;
                self.emit_object_destructure(properties, &src, is_arr)
            }
            Stmt::Expr { expr } => self.emit_side_effect(expr),
            _ => Err(diag("es_od: unsupported stmt")),
        }
    }

    pub(super) fn emit_side_effect(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Assign {
                target: AssignTarget::ObjectPattern { properties },
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let (src, is_arr) = self.emit_source(value)?;
                self.emit_object_destructure(properties, &src, is_arr)
            }
            Expr::Assign {
                target: AssignTarget::Local(id),
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_od: assign local unknown"))?;
                match kind {
                    SlotTy::Number => {
                        let v = self.emit_number_expr(value)?;
                        let ptr = self.slot_ptr(*id)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Object => {
                        let v = self.emit_object_expr(value)?;
                        let ptr = self.slot_ptr(*id)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    _ => return Err(diag("es_od: unsupported local assign kind")),
                }
                Ok(())
            }
            Expr::Call { .. } => {
                let _ = self.emit_number_expr(expr)?;
                Ok(())
            }
            _ => {
                let _ = self.emit_number_expr(expr)?;
                Ok(())
            }
        }
    }

    pub(super) fn emit_source(&mut self, expr: &Expr) -> Result<(String, bool), Diagnostic> {
        match expr {
            Expr::Array { .. } => Ok((self.emit_array_expr(expr)?, true)),
            Expr::Local { id, .. } if self.slot_of.get(id) == Some(&SlotTy::Array) => {
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok((t, true))
            }
            _ => Ok((self.emit_object_expr(expr)?, false)),
        }
    }

    pub(super) fn emit_object_destructure(
        &mut self,
        properties: &[ObjectPatternEl],
        src: &str,
        is_array: bool,
    ) -> Result<(), Diagnostic> {
        let mut excluded_keys: Vec<String> = Vec::new();

        for p in properties {
            match p {
                ObjectPatternEl::Prop {
                    key,
                    binding,
                    default,
                    ..
                } => {
                    let key_s = self.static_key_string(key)?;
                    if let Some(ref s) = key_s {
                        excluded_keys.push(s.clone());
                    }
                    let raw = self.emit_prop_get(src, key, is_array)?;
                    // Apply default if missing (null) or undefined sentinel not used for ptrs.
                    let got = self.fresh();
                    writeln!(self.body, "  {got} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  store ptr {raw}, ptr {got}").ok();
                    if let Some(d) = default {
                        let is_null = self.fresh();
                        writeln!(self.body, "  {is_null} = icmp eq ptr {raw}, null").ok();
                        let def_l = self.fresh_label("dstr_def");
                        let done_l = self.fresh_label("dstr_done");
                        writeln!(
                            self.body,
                            "  br i1 {is_null}, label %{def_l}, label %{done_l}"
                        )
                        .ok();
                        writeln!(self.body, "{def_l}:").ok();
                        let dv = self.emit_number_as_ptr(d)?;
                        writeln!(self.body, "  store ptr {dv}, ptr {got}").ok();
                        writeln!(self.body, "  br label %{done_l}").ok();
                        writeln!(self.body, "{done_l}:").ok();
                    }
                    let val = self.fresh();
                    writeln!(self.body, "  {val} = load ptr, ptr {got}").ok();
                    self.emit_bind_pattern(binding, &val)?;
                }
                ObjectPatternEl::Rest(binding) => {
                    if is_array {
                        return Err(diag("es_od: rest on array source not supported"));
                    }
                    let rest = self.fresh();
                    writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&rest, "")).ok();
                    writeln!(
                        self.body,
                        "  {}",
                        OBJECT_COPY_OWN.call(&format!("ptr {rest}, ptr {src}"))
                    )
                    .ok();
                    for k in &excluded_keys {
                        let key_ptr = self.string_const(k)?;
                        writeln!(
                            self.body,
                            "  {}",
                            OBJECT_DELETE.call(&format!("ptr {rest}, ptr {key_ptr}"))
                        )
                        .ok();
                    }
                    self.emit_bind_pattern(binding, &rest)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn static_key_string(&self, key: &ObjectPropKey) -> Result<Option<String>, Diagnostic> {
        match key {
            ObjectPropKey::Static(s) => Ok(Some(s.to_string_lossy())),
            ObjectPropKey::Computed(_) => Ok(None),
        }
    }

    pub(super) fn emit_prop_get(
        &mut self,
        src: &str,
        key: &ObjectPropKey,
        is_array: bool,
    ) -> Result<String, Diagnostic> {
        if is_array {
            let key_s = match key {
                ObjectPropKey::Static(s) => s.to_string_lossy(),
                ObjectPropKey::Computed(_) => {
                    return Err(diag("es_od: computed key on array source unsupported"));
                }
            };
            if key_s == "length" {
                let len = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    ARRAY_LEN.call_to(&len, &format!("ptr {src}"))
                )
                .ok();
                let p = self.fresh();
                writeln!(self.body, "  {p} = inttoptr i64 {len} to ptr").ok();
                return Ok(p);
            }
            if let Ok(idx) = key_s.parse::<u64>() {
                let raw = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    ARRAY_GET.call_to(&raw, &format!("ptr {src}, i64 {idx}"))
                )
                .ok();
                return Ok(raw);
            }
            // Missing → null
            let p = self.fresh();
            writeln!(self.body, "  {p} = bitcast ptr null to ptr").ok();
            return Ok(p);
        }
        let key_ptr = self.emit_prop_key(key)?;
        let raw = self.fresh();
        writeln!(
            self.body,
            "  {}",
            OBJECT_GET.call_to(&raw, &format!("ptr {src}, ptr {key_ptr}"))
        )
        .ok();
        Ok(raw)
    }

    pub(super) fn emit_bind_pattern(&mut self, binding: &Pattern, val_ptr: &str) -> Result<(), Diagnostic> {
        match binding {
            Pattern::Local(id) => {
                let kind = if self.in_fn && self.fn_local_allocas.contains_key(id) {
                    SlotTy::Number
                } else {
                    *self
                        .slot_of
                        .get(id)
                        .ok_or_else(|| diag("es_od: pattern local unknown"))?
                };
                let ptr = self.slot_ptr(*id)?;
                match kind {
                    SlotTy::Number => {
                        // null → undefined sentinel
                        let is_null = self.fresh();
                        writeln!(self.body, "  {is_null} = icmp eq ptr {val_ptr}, null").ok();
                        let und_l = self.fresh_label("bind_und");
                        let num_l = self.fresh_label("bind_num");
                        let end_l = self.fresh_label("bind_end");
                        writeln!(
                            self.body,
                            "  br i1 {is_null}, label %{und_l}, label %{num_l}"
                        )
                        .ok();
                        writeln!(self.body, "{und_l}:").ok();
                        let u = format!("bitcast (i64 {UNDEF_BITS} to double)");
                        writeln!(self.body, "  store double {u}, ptr {ptr}").ok();
                        writeln!(self.body, "  br label %{end_l}").ok();
                        writeln!(self.body, "{num_l}:").ok();
                        let i = self.fresh();
                        writeln!(self.body, "  {i} = ptrtoint ptr {val_ptr} to i64").ok();
                        let d = self.fresh();
                        writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                        writeln!(self.body, "  store double {d}, ptr {ptr}").ok();
                        writeln!(self.body, "  br label %{end_l}").ok();
                        writeln!(self.body, "{end_l}:").ok();
                    }
                    SlotTy::Object | SlotTy::Array | SlotTy::String | SlotTy::Function => {
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
                    self.emit_string_expr(property)?
                } else {
                    match property.as_ref() {
                        Expr::String { value, .. } => {
                            self.string_const(&value.to_string_lossy())?
                        }
                        _ => return Err(diag("es_od: member pattern key")),
                    }
                };
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {val_ptr}"))
                )
                .ok();
                Ok(())
            }
            Pattern::Object(inner) => {
                // Nested object pattern: val_ptr is the object to destructure.
                self.emit_object_destructure(inner, val_ptr, false)
            }
            Pattern::Array(_) | Pattern::Name(_) => Err(diag("es_od: unsupported pattern binding")),
        }
    }

    pub(super) fn emit_object_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Object { properties, .. } => {
                let obj = self.fresh();
                writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&obj, "")).ok();
                for p in properties {
                    match p {
                        ObjectProp::Property { key, value } => {
                            let key_ptr = self.emit_prop_key(key)?;
                            let val_ptr = if matches!(value, Expr::Object { .. })
                                || self.expr_is_object_slot(value)
                            {
                                self.emit_object_expr(value)?
                            } else {
                                self.emit_number_as_ptr(value)?
                            };
                            writeln!(
                                self.body,
                                "  {}",
                                OBJECT_SET
                                    .call(&format!("ptr {obj}, ptr {key_ptr}, ptr {val_ptr}"))
                            )
                            .ok();
                        }
                        _ => return Err(diag("es_od: only plain properties")),
                    }
                }
                Ok(obj)
            }
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_od: object local unknown"))?;
                if kind != SlotTy::Object {
                    return Err(diag("es_od: expected object local"));
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
                ..
            } => {
                if *optional {
                    return Err(diag("es_od: optional member"));
                }
                let obj = self.emit_object_expr(object)?;
                let key = match property.as_ref() {
                    Expr::String { value, .. } => self.string_const(&value.to_string_lossy())?,
                    _ => self.emit_string_expr(property)?,
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
            _ => Err(diag("es_od: unsupported object expr")),
        }
    }

    pub(super) fn expr_is_object_slot(&self, expr: &Expr) -> bool {
        matches!(
            expr,
            Expr::Local { id, .. } if self.slot_of.get(id) == Some(&SlotTy::Object)
        )
    }

    pub(super) fn emit_array_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Array { elements, .. } => {
                let len = elements.len() as u64;
                let arr = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    ARRAY_NEW.call_to(&arr, &format!("i64 {len}"))
                )
                .ok();
                for (i, el) in elements.iter().enumerate() {
                    let ArrayElement::Expr(e) = el else {
                        return Err(diag("es_od: array hole/spread unsupported"));
                    };
                    let p = self.emit_number_as_ptr(e)?;
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_SET.call(&format!("ptr {arr}, i64 {i}, ptr {p}"))
                    )
                    .ok();
                }
                Ok(arr)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            _ => Err(diag("es_od: unsupported array expr")),
        }
    }

    pub(super) fn emit_number_as_ptr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        let n = self.emit_number_expr(expr)?;
        let i = self.fresh();
        writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
        let p = self.fresh();
        writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
        Ok(p)
    }

    pub(super) fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => format_number_const(raw),
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let inst = match op {
                    BinaryOp::Add => "fadd",
                    BinaryOp::Sub => "fsub",
                    BinaryOp::Mul => "fmul",
                    BinaryOp::Div => "fdiv",
                    _ => return Err(diag("es_od: unsupported binary")),
                };
                let t = self.fresh();
                writeln!(self.body, "  {t} = {inst} double {l}, {r}").ok();
                Ok(t)
            }
            Expr::Member {
                object,
                property,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_od: optional member number"));
                }
                // Object or array member → number (null → undefined sentinel).
                let (src, is_arr) = match object.as_ref() {
                    Expr::Local { id, .. } if self.slot_of.get(id) == Some(&SlotTy::Array) => {
                        let ptr = self.slot_ptr(*id)?;
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                        (t, true)
                    }
                    _ => (self.emit_object_expr(object)?, false),
                };
                let key = match property.as_ref() {
                    Expr::String { value, .. } => ObjectPropKey::Static(value.clone()),
                    _ => {
                        // computed not used for number member in fixture except via pattern
                        return Err(diag("es_od: member key must be string for number get"));
                    }
                };
                let raw = self.emit_prop_get(&src, &key, is_arr)?;
                let is_null = self.fresh();
                writeln!(self.body, "  {is_null} = icmp eq ptr {raw}, null").ok();
                let und_l = self.fresh_label("mem_und");
                let num_l = self.fresh_label("mem_num");
                let end_l = self.fresh_label("mem_end");
                let slot = self.fresh();
                writeln!(self.body, "  {slot} = alloca double, align 8").ok();
                writeln!(
                    self.body,
                    "  br i1 {is_null}, label %{und_l}, label %{num_l}"
                )
                .ok();
                writeln!(self.body, "{und_l}:").ok();
                let u = format!("bitcast (i64 {UNDEF_BITS} to double)");
                writeln!(self.body, "  store double {u}, ptr {slot}").ok();
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{num_l}:").ok();
                let i = self.fresh();
                writeln!(self.body, "  {i} = ptrtoint ptr {raw} to i64").ok();
                let d = self.fresh();
                writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                writeln!(self.body, "  store double {d}, ptr {slot}").ok();
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{end_l}:").ok();
                let out = self.fresh();
                writeln!(self.body, "  {out} = load double, ptr {slot}").ok();
                Ok(out)
            }
            Expr::Call { callee, args, .. } => {
                let Expr::Local { id, .. } = callee.as_ref() else {
                    return Err(diag("es_od: call callee must be local"));
                };
                let idx = self
                    .info
                    .fn_binding
                    .get(id)
                    .copied()
                    .ok_or_else(|| diag("es_od: unknown fn call"))?;
                if args.len() != 1 {
                    return Err(diag("es_od: fn arity"));
                }
                let Arg::Expr(arg_e) = &args[0] else {
                    return Err(diag("es_od: spread arg"));
                };
                let arg = self.emit_object_expr(arg_e)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = call double @d_fn_{idx}(ptr {arg})").ok();
                Ok(t)
            }
            _ => Err(diag("es_od: unsupported number expr")),
        }
    }

    pub(super) fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            _ => Err(diag("es_od: unsupported string expr")),
        }
    }

    pub(super) fn emit_prop_key(&mut self, key: &ObjectPropKey) -> Result<String, Diagnostic> {
        match key {
            ObjectPropKey::Static(s) => self.string_const(&s.to_string_lossy()),
            ObjectPropKey::Computed(e) => self.emit_string_expr(e),
        }
    }

    pub(super) fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        if let Some(p) = self.fn_local_allocas.get(&id) {
            return Ok(p.clone());
        }
        self.allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("es_od: slot missing"))
    }

    pub(super) fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let gname = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".es_od_str.{}", self.str_n);
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

    pub(super) fn emit_print_str(&mut self, s: &str) -> Result<(), Diagnostic> {
        let p = self.string_const(s)?;
        writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {p}"))).ok();
        Ok(())
    }
}
