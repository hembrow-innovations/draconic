use std::fmt::Write as _;

use super::*;
use crate::emitter::escape_llvm_string;

impl<'a> super::Emitter<'a> {
    pub(super) fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        Self {
            module,
            info,
            slot_of: HashMap::new(),
            allocas: HashMap::new(),
            param_allocas: HashMap::new(),
            this_ssa: None,
            str_globals: Vec::new(),
            out: String::new(),
            body: String::new(),
            tmp: 0,
            str_n: 0,
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
            "; Draconic LLVM backend (N08.04 ES objects + new/prototype via Runtime ABI)"
        )
        .ok();
        writeln!(
            self.out,
            "{}",
            llvm_declares(&[
                GC_INIT,
                ALLOC_OBJECT,
                OBJECT_SET,
                OBJECT_GET,
                OBJECT_SET_PROTO,
                OBJECT_SPREAD,
                PRINT_F64,
            ])
        )
        .ok();
        writeln!(self.out).ok();

        // Number/string slots as module globals so method bodies can load free vars.
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
                SlotTy::String => {
                    let g = string_global_name(*id);
                    writeln!(self.out, "@{g} = internal global ptr null, align 8").ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
                SlotTy::Object => {}
            }
        }
        if info
            .slots
            .iter()
            .any(|(_, k)| matches!(k, SlotTy::Number | SlotTy::String))
        {
            writeln!(self.out).ok();
        }

        // Emit method/ctor functions first (collect string globals into self.str_globals).
        for f in &info.functions.clone() {
            self.emit_method_fn(f)?;
        }

        // Main body into self.body — object slots stay stack allocas.
        for (id, kind) in &info.slots {
            if *kind != SlotTy::Object {
                continue;
            }
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(*id, ptr.clone());
            writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
            writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for id in &info.number_locals {
            let ptr = self.number_slot_ptr(*id)?;
            let v = self.fresh();
            writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
            writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
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

    pub(super) fn emit_method_fn(&mut self, f: &FnInfo) -> Result<(), Diagnostic> {
        let name = format!("m_fn_{}", f.idx);
        // Uniform signature: double (ptr %this, double %a0, ... %a{MAX-1})
        let mut params_s = String::from("ptr %this");
        for i in 0..MAX_METHOD_ARGS {
            write!(params_s, ", double %a{i}").ok();
        }
        writeln!(self.out, "define double @{name}({params_s}) {{").ok();
        writeln!(self.out, "entry:").ok();

        // Save outer body/this/params and emit into a fresh buffer that becomes the fn body.
        let saved_body = std::mem::take(&mut self.body);
        let saved_this = self.this_ssa.take();
        let saved_params = std::mem::take(&mut self.param_allocas);
        let saved_allocas = std::mem::take(&mut self.allocas);

        self.this_ssa = Some("%this".to_string());

        for (i, pid) in f.params.iter().enumerate() {
            let ptr = format!("%p{}", pid.0);
            writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
            writeln!(self.body, "  store double %a{i}, ptr {ptr}").ok();
            self.param_allocas.insert(*pid, ptr);
        }

        // Default return 0 if fall-off.
        let mut saw_return = false;
        for stmt in &f.body {
            if matches!(stmt, Stmt::Return { .. }) {
                saw_return = true;
            }
            self.emit_method_stmt(stmt)?;
        }
        if !saw_return {
            writeln!(self.body, "  ret double 0.00000000000000000e+00").ok();
        }

        self.out.push_str(&self.body);
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();

        self.body = saved_body;
        self.this_ssa = saved_this;
        self.param_allocas = saved_params;
        self.allocas = saved_allocas;
        Ok(())
    }

    pub(super) fn emit_method_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Return { value: Some(e) } => {
                let v = self.emit_number_expr(e)?;
                writeln!(self.body, "  ret double {v}").ok();
                Ok(())
            }
            Stmt::Return { value: None } => {
                writeln!(self.body, "  ret double 0.00000000000000000e+00").ok();
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_method_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Expr { expr } => self.emit_side_effect_expr(expr),
            _ => Err(diag("es_objects: unsupported method stmt")),
        }
    }

    pub(super) fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Function { local, .. } => {
                // Ctor object with empty `.prototype` (N08.04.05).
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
                let ptr = self
                    .allocas
                    .get(local)
                    .cloned()
                    .ok_or_else(|| diag("es_objects: function binding missing alloca"))?;
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
                    .ok_or_else(|| diag("es_objects: declare unknown slot"))?;
                match kind {
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        let ptr = self.number_slot_ptr(*local)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::String => {
                        let v = self.emit_string_expr(init)?;
                        let ptr = self.string_slot_ptr(*local)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Object => {
                        let v = self.emit_object_expr(init)?;
                        let ptr = self.allocas.get(local).cloned().unwrap();
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Stmt::Expr { expr } => self.emit_side_effect_expr(expr),
            _ => Err(diag("es_objects: unsupported stmt")),
        }
    }

    /// Top-level / method statement expressions (discarded values).
    pub(super) fn emit_side_effect_expr(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Assign {
                target:
                    AssignTarget::Member {
                        object, property, ..
                    },
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let obj = self.emit_object_expr(object)?;
                let key = self.member_key_cstr(property)?;
                let val_ptr = if let Expr::Function { params, body, .. } = value.as_ref() {
                    let idx = find_fn_idx(params, body, &self.info.functions)
                        .ok_or_else(|| diag("es_objects: unknown method FunctionExpr"))?;
                    format!("@m_fn_{idx}")
                } else if object_value_is_object(value)
                    || matches!(value.as_ref(), Expr::New { .. })
                {
                    self.emit_object_expr(value)?
                } else {
                    let n = self.emit_number_expr(value)?;
                    let i = self.fresh();
                    writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
                    let p = self.fresh();
                    writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
                    p
                };
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {val_ptr}"))
                )
                .ok();
                Ok(())
            }
            _ => {
                let _ = self.emit_number_expr(expr)?;
                Ok(())
            }
        }
    }

    pub(super) fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => format_number_const(raw),
            Expr::Local { id, .. } => {
                if let Some(ptr) = self.param_allocas.get(id).cloned() {
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                    return Ok(t);
                }
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_objects: number local unknown"))?;
                if kind != SlotTy::Number {
                    return Err(diag("es_objects: expected number local"));
                }
                let ptr = self.number_slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Member {
                object,
                property,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_objects: optional member not supported"));
                }
                let obj = self.emit_object_expr(object)?;
                let key = self.member_key_cstr(property)?;
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
            Expr::Assign {
                target:
                    AssignTarget::Member {
                        object, property, ..
                    },
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let obj = self.emit_object_expr(object)?;
                let key = self.member_key_cstr(property)?;
                let n = self.emit_number_expr(value)?;
                let i = self.fresh();
                writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
                let p = self.fresh();
                writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {p}"))
                )
                .ok();
                Ok(n)
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
                    _ => return Err(diag("es_objects: unsupported binary")),
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
                    return Err(diag("es_objects: optional call not supported"));
                }
                self.emit_method_call(callee, args)
            }
            _ => Err(diag("es_objects: unsupported number expr")),
        }
    }

    pub(super) fn emit_method_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Member {
            object,
            property,
            optional,
            ..
        } = callee
        else {
            return Err(diag("es_objects: method call requires member callee"));
        };
        if *optional {
            return Err(diag("es_objects: optional member call not supported"));
        }
        let recv = self.emit_object_expr(object)?;
        let key = self.member_key_cstr(property)?;
        let fn_ptr = self.fresh();
        writeln!(
            self.body,
            "  {}",
            OBJECT_GET.call_to(&fn_ptr, &format!("ptr {recv}, ptr {key}"))
        )
        .ok();

        let mut arg_vals = Vec::new();
        for a in args {
            match a {
                Arg::Expr(e) => arg_vals.push(self.emit_number_expr(e)?),
                Arg::Spread(_) => {
                    return Err(diag("es_objects: spread args not supported"));
                }
            }
        }
        while arg_vals.len() < MAX_METHOD_ARGS {
            arg_vals.push("0.00000000000000000e+00".to_string());
        }

        let mut call_args = format!("ptr {recv}");
        for v in &arg_vals {
            write!(call_args, ", double {v}").ok();
        }
        // Typed call through opaque ptr.
        let mut ty_params = String::from("ptr");
        for _ in 0..MAX_METHOD_ARGS {
            ty_params.push_str(", double");
        }
        let ret = self.fresh();
        writeln!(
            self.body,
            "  {ret} = call double ({ty_params}) {fn_ptr}({call_args})"
        )
        .ok();
        Ok(ret)
    }

    pub(super) fn emit_new(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = callee else {
            return Err(diag("es_objects: new callee must be local ctor"));
        };
        let idx = *self
            .info
            .fn_binding
            .get(id)
            .ok_or_else(|| diag("es_objects: unknown constructor"))?;
        // N08.04.05: instance.[[Prototype]] = C.prototype
        let ctor = {
            let ptr = self
                .allocas
                .get(id)
                .cloned()
                .ok_or_else(|| diag("es_objects: ctor binding missing alloca"))?;
            let t = self.fresh();
            writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
            t
        };
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

        let mut arg_vals = Vec::new();
        for a in args {
            match a {
                Arg::Expr(e) => arg_vals.push(self.emit_number_expr(e)?),
                Arg::Spread(_) => {
                    return Err(diag("es_objects: spread args not supported"));
                }
            }
        }
        while arg_vals.len() < MAX_METHOD_ARGS {
            arg_vals.push("0.00000000000000000e+00".to_string());
        }

        let mut call_args = format!("ptr {obj}");
        for v in &arg_vals {
            write!(call_args, ", double {v}").ok();
        }
        let mut ty_params = String::from("ptr");
        for _ in 0..MAX_METHOD_ARGS {
            ty_params.push_str(", double");
        }
        let ret = self.fresh();
        writeln!(
            self.body,
            "  {ret} = call double ({ty_params}) @m_fn_{idx}({call_args})"
        )
        .ok();
        // Ignore ctor return value; instance is the allocated object.
        let _ = ret;
        Ok(obj)
    }

    pub(super) fn emit_object_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::This { .. } => self
                .this_ssa
                .clone()
                .ok_or_else(|| diag("es_objects: This outside method")),
            Expr::New { callee, args, .. } => self.emit_new(callee, args),
            Expr::Object { properties, .. } => {
                let obj = self.fresh();
                writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&obj, "")).ok();
                for p in properties {
                    match p {
                        ObjectProp::Property { key, value } => {
                            let key_ptr = self.emit_prop_key(key)?;
                            let val_ptr = if let Expr::Function { params, body, .. } = value {
                                let idx = find_fn_idx(params, body, &self.info.functions)
                                    .ok_or_else(|| {
                                        diag("es_objects: unknown method FunctionExpr")
                                    })?;
                                // Function address as ptr (opaque pointers).
                                format!("@m_fn_{idx}")
                            } else if object_value_is_object(value) {
                                self.emit_object_expr(value)?
                            } else {
                                let n = self.emit_number_expr(value)?;
                                let i = self.fresh();
                                writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
                                let p = self.fresh();
                                writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
                                p
                            };
                            writeln!(
                                self.body,
                                "  {}",
                                OBJECT_SET
                                    .call(&format!("ptr {obj}, ptr {key_ptr}, ptr {val_ptr}"))
                            )
                            .ok();
                        }
                        ObjectProp::Spread(e) => {
                            let src = self.emit_spread_source(e)?;
                            writeln!(
                                self.body,
                                "  {}",
                                OBJECT_SPREAD.call(&format!("ptr {obj}, ptr {src}"))
                            )
                            .ok();
                        }
                        _ => return Err(diag("es_objects: only plain properties / spread")),
                    }
                }
                Ok(obj)
            }
            Expr::Local { id, .. } => {
                if let Some(kind) = self.slot_of.get(id).copied() {
                    if kind != SlotTy::Object {
                        return Err(diag("es_objects: expected object local"));
                    }
                    let ptr = self.allocas.get(id).cloned().unwrap();
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                    return Ok(t);
                }
                if self.info.fn_binding.contains_key(id) {
                    let ptr = self
                        .allocas
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("es_objects: fn object local unknown"))?;
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                    return Ok(t);
                }
                Err(diag("es_objects: object local unknown"))
            }
            Expr::Member {
                object,
                property,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_objects: optional member not supported"));
                }
                let obj = self.emit_object_expr(object)?;
                let key = self.member_key_cstr(property)?;
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_GET.call_to(&t, &format!("ptr {obj}, ptr {key}"))
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag("es_objects: unsupported object expr")),
        }
    }

    pub(super) fn member_key_cstr(&mut self, property: &Expr) -> Result<String, Diagnostic> {
        match property {
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            _ => Err(diag("es_objects: member key must be string")),
        }
    }

    pub(super) fn emit_prop_key(&mut self, key: &ObjectPropKey) -> Result<String, Diagnostic> {
        match key {
            ObjectPropKey::Static(s) => self.string_const(&s.to_string_lossy()),
            ObjectPropKey::Computed(e) => self.emit_string_expr(e),
        }
    }

    /// N08.16.28: object spread source — null/undefined → null ptr (runtime no-op).
    pub(super) fn emit_spread_source(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Null { .. } => Ok("null".to_string()),
            Expr::Unary {
                op: UnaryOp::Void, ..
            } => Ok("null".to_string()),
            Expr::Local { id, .. }
                if !self.slot_of.contains_key(id) && !self.info.fn_binding.contains_key(id) =>
            {
                // Builtin `undefined` (or other non-slot Any) — no-op spread.
                Ok("null".to_string())
            }
            _ => self.emit_object_expr(expr),
        }
    }

    pub(super) fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_objects: string local unknown"))?;
                if kind != SlotTy::String {
                    return Err(diag("es_objects: expected string local"));
                }
                let ptr = self.string_slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            _ => Err(diag("es_objects: unsupported string expr")),
        }
    }

    pub(super) fn number_slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        if let Some(ptr) = self.allocas.get(&id) {
            return Ok(ptr.clone());
        }
        // Methods emit before main fills object allocas; number slots are globals.
        if self.slot_of.get(&id) == Some(&SlotTy::Number) {
            return Ok(format!("@{}", number_global_name(id)));
        }
        Err(diag("es_objects: number slot missing"))
    }

    pub(super) fn string_slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        if let Some(ptr) = self.allocas.get(&id) {
            return Ok(ptr.clone());
        }
        if self.slot_of.get(&id) == Some(&SlotTy::String) {
            return Ok(format!("@{}", string_global_name(id)));
        }
        Err(diag("es_objects: string slot missing"))
    }

    pub(super) fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let gname = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".es_obj_str.{}", self.str_n);
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
