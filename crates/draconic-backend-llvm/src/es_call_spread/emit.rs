use std::fmt::Write as _;

use super::*;
use crate::emitter::escape_llvm_string;

impl<'a> super::Emitter<'a> {
    pub(super) fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            allocas: HashMap::new(),
            slot_of: HashMap::new(),
            param_allocas: HashMap::new(),
            this_ssa: None,
            str_globals: Vec::new(),
            tmp: 0,
            str_n: 0,
            in_string_fn: false,
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

    pub(super) fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for (id, ty) in &info.slots {
            self.slot_of.insert(*id, *ty);
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.06.04 call/new spread via Runtime ABI)"
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
                ALLOC_OBJECT,
                OBJECT_SET,
                OBJECT_GET,
                OBJECT_SET_PROTO,
                CSTR_CONCAT,
                PRINT_F64,
                PRINT_STR,
            ])
        )
        .ok();
        writeln!(self.out).ok();

        for (id, kind) in &info.slots {
            match kind {
                LocalSlot::Number => {
                    let g = format!("es_cs_n{}", id.0);
                    writeln!(
                        self.out,
                        "@{g} = internal global double 0.00000000000000000e+00, align 8"
                    )
                    .ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
                LocalSlot::String | LocalSlot::Array | LocalSlot::Object => {
                    let tag = match kind {
                        LocalSlot::String => "s",
                        LocalSlot::Array => "a",
                        LocalSlot::Object => "o",
                        LocalSlot::Number => "n",
                    };
                    let g = format!("es_cs_{tag}{}", id.0);
                    writeln!(self.out, "@{g} = internal global ptr null, align 8").ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
            }
        }
        if !info.slots.is_empty() {
            writeln!(self.out).ok();
        }

        for f in &info.functions.clone() {
            self.emit_fn(f)?;
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for (id, kind) in &info.print_locals {
            let ptr = self.slot_ptr(*id)?;
            match kind {
                LocalSlot::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                }
                LocalSlot::String => {
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
        let name = format!("cs_fn_{}", f.idx);
        let saved_body = std::mem::take(&mut self.body);
        let saved_this = self.this_ssa.take();
        let saved_params = std::mem::take(&mut self.param_allocas);
        let saved_in_str = self.in_string_fn;

        match f.kind {
            FnKind::Number => {
                self.in_string_fn = false;
                let mut params_s = String::new();
                for i in 0..f.params.len() {
                    if i > 0 {
                        params_s.push_str(", ");
                    }
                    write!(params_s, "double %a{i}").ok();
                }
                writeln!(self.out, "define double @{name}({params_s}) {{").ok();
                writeln!(self.out, "entry:").ok();
                for (i, pid) in f.params.iter().enumerate() {
                    let ptr = format!("%p{}", pid.0);
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                    writeln!(self.body, "  store double %a{i}, ptr {ptr}").ok();
                    self.param_allocas.insert(*pid, ptr);
                }
                let mut saw_ret = false;
                for stmt in &f.body {
                    if matches!(stmt, Stmt::Return { .. }) {
                        saw_ret = true;
                    }
                    self.emit_fn_stmt(stmt, f.kind)?;
                }
                if !saw_ret {
                    writeln!(self.body, "  ret double 0.00000000000000000e+00").ok();
                }
            }
            FnKind::String => {
                self.in_string_fn = true;
                let mut params_s = String::new();
                for i in 0..f.params.len() {
                    if i > 0 {
                        params_s.push_str(", ");
                    }
                    write!(params_s, "ptr %a{i}").ok();
                }
                writeln!(self.out, "define ptr @{name}({params_s}) {{").ok();
                writeln!(self.out, "entry:").ok();
                for (i, pid) in f.params.iter().enumerate() {
                    let ptr = format!("%p{}", pid.0);
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  store ptr %a{i}, ptr {ptr}").ok();
                    self.param_allocas.insert(*pid, ptr);
                }
                let mut saw_ret = false;
                for stmt in &f.body {
                    if matches!(stmt, Stmt::Return { .. }) {
                        saw_ret = true;
                    }
                    self.emit_fn_stmt(stmt, f.kind)?;
                }
                if !saw_ret {
                    writeln!(self.body, "  ret ptr null").ok();
                }
            }
            FnKind::Ctor => {
                self.in_string_fn = false;
                let mut params_s = String::from("ptr %this");
                for i in 0..f.params.len() {
                    write!(params_s, ", double %a{i}").ok();
                }
                writeln!(self.out, "define double @{name}({params_s}) {{").ok();
                writeln!(self.out, "entry:").ok();
                self.this_ssa = Some("%this".to_string());
                for (i, pid) in f.params.iter().enumerate() {
                    let ptr = format!("%p{}", pid.0);
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                    writeln!(self.body, "  store double %a{i}, ptr {ptr}").ok();
                    self.param_allocas.insert(*pid, ptr);
                }
                for stmt in &f.body {
                    self.emit_fn_stmt(stmt, f.kind)?;
                }
                writeln!(self.body, "  ret double 0.00000000000000000e+00").ok();
            }
        }

        self.out.push_str(&self.body);
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();

        self.body = saved_body;
        self.this_ssa = saved_this;
        self.param_allocas = saved_params;
        self.in_string_fn = saved_in_str;
        Ok(())
    }

    fn emit_fn_stmt(&mut self, stmt: &Stmt, kind: FnKind) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Return { value: Some(e) } => match kind {
                FnKind::Number => {
                    let v = self.emit_number_expr(e)?;
                    writeln!(self.body, "  ret double {v}").ok();
                    Ok(())
                }
                FnKind::String => {
                    let v = self.emit_string_expr(e)?;
                    writeln!(self.body, "  ret ptr {v}").ok();
                    Ok(())
                }
                FnKind::Ctor => Err(diag("es_call_spread: ctor must not return value")),
            },
            Stmt::Return { value: None } => {
                match kind {
                    FnKind::Number => {
                        writeln!(self.body, "  ret double 0.00000000000000000e+00").ok()
                    }
                    FnKind::String => writeln!(self.body, "  ret ptr null").ok(),
                    FnKind::Ctor => {
                        return Err(diag("es_call_spread: bare return in ctor"));
                    }
                };
                Ok(())
            }
            Stmt::Expr {
                expr:
                    Expr::Assign {
                        target:
                            AssignTarget::Member {
                                object,
                                property,
                                computed: false,
                                ..
                            },
                        op: AssignOp::Eq,
                        value,
                        ..
                    },
            } => {
                if !matches!(object.as_ref(), Expr::This { .. }) {
                    return Err(diag("es_call_spread: only this.prop assign in ctor"));
                }
                let this = self
                    .this_ssa
                    .clone()
                    .ok_or_else(|| diag("es_call_spread: this outside ctor"))?;
                let key = match property.as_ref() {
                    Expr::String { value, .. } => self.string_const(&value.to_string_lossy())?,
                    _ => return Err(diag("es_call_spread: prop key must be string")),
                };
                let n = self.emit_number_expr(value)?;
                let i = self.fresh();
                writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
                let p = self.fresh();
                writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {this}, ptr {key}, ptr {p}"))
                )
                .ok();
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_fn_stmt(s, kind)?;
                }
                Ok(())
            }
            _ => Err(diag("es_call_spread: unsupported fn stmt")),
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Function { local, .. } => {
                let idx = *self
                    .info
                    .fn_binding
                    .get(local)
                    .ok_or_else(|| diag("es_call_spread: unknown fn"))?;
                if self.info.functions[idx].kind != FnKind::Ctor {
                    return Ok(());
                }
                // Ctor object with empty .prototype (same as es_objects).
                let ctor = self.fresh();
                writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&ctor, "")).ok();
                let proto = self.fresh();
                writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&proto, "")).ok();
                let key = self.string_const("prototype")?;
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {ctor}, ptr {key}, ptr {proto}"))
                )
                .ok();
                let ptr = self.slot_ptr(*local)?;
                writeln!(self.body, "  store ptr {ctor}, ptr {ptr}").ok();
                Ok(())
            }
            Stmt::Declare { local, init, .. } => {
                let Some(init) = init else {
                    return Ok(());
                };
                let kind = *self
                    .slot_of
                    .get(local)
                    .ok_or_else(|| diag("es_call_spread: declare unknown slot"))?;
                let ptr = self.slot_ptr(*local)?;
                match kind {
                    LocalSlot::Number => {
                        let v = self.emit_number_expr(init)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    LocalSlot::String => {
                        let v = self.emit_string_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    LocalSlot::Array => {
                        let v = self.emit_array_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    LocalSlot::Object => {
                        let v = self.emit_object_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            _ => Err(diag("es_call_spread: unsupported top-level stmt")),
        }
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => format_number_const(raw),
            Expr::Local { id, .. } => {
                if let Some(ptr) = self.param_allocas.get(id).cloned() {
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                    return Ok(t);
                }
                if self.slot_of.get(id) != Some(&LocalSlot::Number) {
                    return Err(diag("es_call_spread: expected number local"));
                }
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
                    BinaryOp::Rem => "frem",
                    _ => return Err(diag("es_call_spread: unsupported binary")),
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
                    return Err(diag("es_call_spread: optional call"));
                }
                self.emit_number_call(callee, args)
            }
            Expr::Member {
                object,
                property,
                optional,
                computed,
                ..
            } => {
                if *optional || *computed {
                    return Err(diag("es_call_spread: only static prop get"));
                }
                let obj = self.emit_object_expr(object)?;
                let key = match property.as_ref() {
                    Expr::String { value, .. } => self.string_const(&value.to_string_lossy())?,
                    _ => return Err(diag("es_call_spread: prop key string")),
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
                Ok(d)
            }
            _ => Err(diag("es_call_spread: unsupported number expr")),
        }
    }

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            Expr::Local { id, .. } => {
                if let Some(ptr) = self.param_allocas.get(id).cloned() {
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                    return Ok(t);
                }
                if self.slot_of.get(id) != Some(&LocalSlot::String) {
                    return Err(diag("es_call_spread: expected string local"));
                }
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Binary {
                op: BinaryOp::Add,
                left,
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
            Expr::Call {
                callee,
                args,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_call_spread: optional call"));
                }
                self.emit_string_call(callee, args)
            }
            _ => Err(diag("es_call_spread: unsupported string expr")),
        }
    }

    fn emit_number_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = callee else {
            return Err(diag("es_call_spread: call callee must be local"));
        };
        let idx = *self
            .info
            .fn_binding
            .get(id)
            .ok_or_else(|| diag("es_call_spread: unbound callee"))?;
        let f = &self.info.functions[idx];
        if f.kind != FnKind::Number {
            return Err(diag("es_call_spread: expected number fn"));
        }
        let expanded = expand_args_static(args, &self.info.arr_inits, &self.slot_of)
            .ok_or_else(|| diag("es_call_spread: cannot expand spread args"))?;
        if expanded.len() > f.params.len() {
            return Err(diag("es_call_spread: too many args"));
        }
        let mut arg_vals = Vec::new();
        for e in &expanded {
            arg_vals.push(self.emit_number_expr(e)?);
        }
        while arg_vals.len() < f.params.len() {
            arg_vals.push("0.00000000000000000e+00".to_string());
        }
        let parts: Vec<_> = arg_vals.iter().map(|v| format!("double {v}")).collect();
        let t = self.fresh();
        if parts.is_empty() {
            writeln!(self.body, "  {t} = call double @cs_fn_{idx}()").ok();
        } else {
            writeln!(
                self.body,
                "  {t} = call double @cs_fn_{idx}({})",
                parts.join(", ")
            )
            .ok();
        }
        Ok(t)
    }

    fn emit_string_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = callee else {
            return Err(diag("es_call_spread: call callee must be local"));
        };
        let idx = *self
            .info
            .fn_binding
            .get(id)
            .ok_or_else(|| diag("es_call_spread: unbound callee"))?;
        let f = &self.info.functions[idx];
        if f.kind != FnKind::String {
            return Err(diag("es_call_spread: expected string fn"));
        }
        let expanded = expand_args_static(args, &self.info.arr_inits, &self.slot_of)
            .ok_or_else(|| diag("es_call_spread: cannot expand spread args"))?;
        if expanded.len() > f.params.len() {
            return Err(diag("es_call_spread: too many args"));
        }
        let mut arg_vals = Vec::new();
        for e in &expanded {
            arg_vals.push(self.emit_string_expr(e)?);
        }
        while arg_vals.len() < f.params.len() {
            arg_vals.push("null".to_string());
        }
        let parts: Vec<_> = arg_vals.iter().map(|v| format!("ptr {v}")).collect();
        let t = self.fresh();
        if parts.is_empty() {
            writeln!(self.body, "  {t} = call ptr @cs_fn_{idx}()").ok();
        } else {
            writeln!(
                self.body,
                "  {t} = call ptr @cs_fn_{idx}({})",
                parts.join(", ")
            )
            .ok();
        }
        Ok(t)
    }

    fn emit_object_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Local { id, .. } => {
                if self.slot_of.get(id) != Some(&LocalSlot::Object) {
                    return Err(diag("es_call_spread: expected object local"));
                }
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::New { callee, args, .. } => self.emit_new(callee, args),
            _ => Err(diag("es_call_spread: unsupported object expr")),
        }
    }

    fn emit_new(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = callee else {
            return Err(diag("es_call_spread: new callee must be local"));
        };
        let idx = *self
            .info
            .fn_binding
            .get(id)
            .ok_or_else(|| diag("es_call_spread: unknown ctor"))?;
        if self.info.functions[idx].kind != FnKind::Ctor {
            return Err(diag("es_call_spread: not a ctor"));
        }
        let ctor_ptr = self.slot_ptr(*id)?;
        let ctor = self.fresh();
        writeln!(self.body, "  {ctor} = load ptr, ptr {ctor_ptr}").ok();
        let proto_key = self.string_const("prototype")?;
        let proto = self.fresh();
        writeln!(
            self.body,
            "  {}",
            OBJECT_GET.call_to(&proto, &format!("ptr {ctor}, ptr {proto_key}"))
        )
        .ok();
        let obj = self.fresh();
        writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&obj, "")).ok();
        writeln!(
            self.body,
            "  {}",
            OBJECT_SET_PROTO.call(&format!("ptr {obj}, ptr {proto}"))
        )
        .ok();

        let expanded = expand_args_static(args, &self.info.arr_inits, &self.slot_of)
            .ok_or_else(|| diag("es_call_spread: cannot expand new spread"))?;
        let n_params = self.info.functions[idx].params.len();
        if expanded.len() > n_params {
            return Err(diag("es_call_spread: too many new args"));
        }
        let mut arg_vals = Vec::new();
        for e in &expanded {
            arg_vals.push(self.emit_number_expr(e)?);
        }
        while arg_vals.len() < n_params {
            arg_vals.push("0.00000000000000000e+00".to_string());
        }
        let mut call_args = format!("ptr {obj}");
        for v in &arg_vals {
            write!(call_args, ", double {v}").ok();
        }
        let ret = self.fresh();
        writeln!(self.body, "  {ret} = call double @cs_fn_{idx}({call_args})").ok();
        let _ = ret;
        Ok(obj)
    }

    fn emit_array_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Array { elements, .. } => self.emit_array_lit(elements),
            Expr::Local { id, .. } => {
                if self.slot_of.get(id) != Some(&LocalSlot::Array) {
                    return Err(diag("es_call_spread: expected array local"));
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
                    return Err(diag("es_call_spread: nested array via index only"));
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
            _ => Err(diag("es_call_spread: unsupported array expr")),
        }
    }

    fn emit_array_lit(&mut self, elements: &[ArrayElement]) -> Result<String, Diagnostic> {
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
                ArrayElement::Spread(_) => {
                    return Err(diag("es_call_spread: array lit spread not in this path"));
                }
                ArrayElement::Expr(e) => {
                    let v = if matches!(e, Expr::Array { .. })
                        || matches!(e, Expr::Local { id, .. } if self.slot_of.get(id) == Some(&LocalSlot::Array))
                    {
                        self.emit_array_expr(e)?
                    } else if matches!(e, Expr::String { .. })
                        || matches!(e, Expr::Local { id, .. } if self.slot_of.get(id) == Some(&LocalSlot::String))
                    {
                        self.emit_string_expr(e)?
                    } else {
                        let n = self.emit_number_expr(e)?;
                        let i64v = self.fresh();
                        writeln!(self.body, "  {i64v} = fptosi double {n} to i64").ok();
                        let p = self.fresh();
                        writeln!(self.body, "  {p} = inttoptr i64 {i64v} to ptr").ok();
                        p
                    };
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_SET.call(&format!("ptr {arr}, i64 {i}, ptr {v}"))
                    )
                    .ok();
                }
            }
        }
        Ok(arr)
    }

    fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        self.allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("es_call_spread: slot missing"))
    }

    fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let gname = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".es_cs_str.{}", self.str_n);
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
