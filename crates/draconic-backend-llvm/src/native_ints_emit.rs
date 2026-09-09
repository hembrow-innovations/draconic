use std::fmt::Write as _;

use super::*;

impl<'a> super::Emitter<'a> {
    pub(super) fn new(module: &'a Module, debug: Option<&'a SourceDebug>) -> Self {
        let locals: HashMap<LocalId, &'a Local> = module.locals.iter().map(|l| (l.id, l)).collect();
        let mut fn_names = HashMap::new();
        let mut extern_fns = HashMap::new();
        for stmt in &module.body {
            match stmt {
                Stmt::Function { local, .. } => {
                    let name = locals.get(local).map(|l| l.name.as_str()).unwrap_or("fn");
                    let safe: String = name
                        .chars()
                        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                        .collect();
                    fn_names.insert(*local, format!("d_{safe}_{}", local.0));
                }
                Stmt::ExternFunction {
                    local,
                    name,
                    params,
                    ret,
                    ..
                } => {
                    // C linkage uses the source name as the symbol (F06.03).
                    fn_names.insert(*local, name.clone());
                    extern_fns.insert(
                        *local,
                        ExternAbi {
                            name: name.clone(),
                            params: params.clone(),
                            ret: *ret,
                        },
                    );
                }
                _ => {}
            }
        }
        Self {
            module,
            debug,
            locals,
            allocas: HashMap::new(),
            fn_names,
            extern_fns,
            params: HashMap::new(),
            out: String::new(),
            body: String::new(),
            tmp: 0,
            label: 0,
            print_order: Vec::new(),
            body_idx: 0,
        }
    }

    pub(super) fn emit_dbg_for_body_index(&mut self, idx: usize) {
        let Some(debug) = self.debug else {
            return;
        };
        let span = self
            .module
            .body_spans
            .get(idx)
            .copied()
            .unwrap_or_else(Span::dummy);
        let (line, col) = if span.is_dummy() {
            (1, 1)
        } else {
            let file = SourceFile::new("src", &debug.source);
            let loc = file.lookup(span.start);
            (loc.line.max(1), loc.column.max(1))
        };
        writeln!(self.body, "{}", dbg_marker(line, col)).ok();
    }

    pub(super) fn finish(self) -> String {
        self.out
    }

    pub(super) fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM backend (N01–N03.03 native scalars/layouts/pointers)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(NATIVE_INT_DECLARES)).ok();
        writeln!(self.out).ok();

        // F06.03: emit `declare` for each extern "C" ABI surface first.
        for stmt in &self.module.body {
            if let Stmt::ExternFunction {
                name,
                params,
                ret,
                abi,
                ..
            } = stmt
            {
                if abi != "C" {
                    return Err(diag(&format!(
                        "native FFI: unsupported extern ABI {abi:?}; only \"C\" is supported"
                    )));
                }
                self.emit_extern_declare(name, params, ret.as_ref().copied())?;
            }
        }

        // Emit nested function definitions first.
        for stmt in &self.module.body {
            if let Stmt::Function {
                local,
                params,
                body,
                is_async,
                is_generator,
            } = stmt
            {
                if *is_async || *is_generator {
                    return Err(diag(
                        "native scalars: async/generator functions not supported",
                    ));
                }
                self.emit_function(*local, params, body)?;
            }
        }

        // main
        self.body.clear();
        self.tmp = 0;
        self.label = 0;
        self.params.clear();
        self.allocas.clear();
        self.print_order.clear();

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();

        // Pre-declare allocas for all non-function, non-param top-level locals.
        for local in &self.module.locals {
            if matches!(local.ty, Type::Function) {
                continue;
            }
            if let Some(sc) = scalar_of_type(local.ty) {
                let ptr = format!("%l{}", local.id.0);
                self.allocas.insert(local.id, ptr.clone());
                writeln!(
                    self.out,
                    "  {ptr} = alloca {}, align {}",
                    sc.llvm_ty(),
                    sc.align()
                )
                .ok();
            } else if matches!(local.ty, Type::Ptr(_)) {
                let ptr = format!("%l{}", local.id.0);
                self.allocas.insert(local.id, ptr.clone());
                writeln!(self.out, "  {ptr} = alloca ptr, align 8").ok();
            } else if let Some(shape) = native_layout_of(self.module, local.ty) {
                let ptr = format!("%l{}", local.id.0);
                self.allocas.insert(local.id, ptr.clone());
                writeln!(
                    self.out,
                    "  {ptr} = alloca {}, align {}",
                    llvm_layout_ty(shape),
                    layout_align(shape)
                )
                .ok();
            }
        }

        for (idx, stmt) in self.module.body.iter().enumerate() {
            if matches!(stmt, Stmt::Function { .. } | Stmt::ExternFunction { .. }) {
                continue;
            }
            self.body_idx = idx;
            self.emit_dbg_for_body_index(idx);
            self.emit_stmt(stmt)?;
        }

        // Print top-level native int declares in source order.
        for id in self.print_order.clone() {
            self.emit_print_local(id)?;
        }

        writeln!(self.out, "{}", self.body).ok();
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    pub(super) fn llvm_abi_spelling(&self, ty: Type) -> Result<String, Diagnostic> {
        match ty {
            Type::Native(n) => Ok(llvm_ty(n).to_string()),
            Type::Boolean => Ok("i1".into()),
            Type::Number => Ok("double".into()),
            Type::Ptr(_) | Type::Function => Ok("ptr".into()),
            Type::Shape(_) => {
                let shape = native_layout_of(self.module, ty)
                    .ok_or_else(|| diag("extern ABI: type is not a native layout"))?;
                let size = layout_size(shape);
                if size == 0 || size > 16 {
                    return Err(diag(
                        "extern ABI: native layout must be 1..=16 bytes to pass/return by value",
                    ));
                }
                Ok(layout_abi_llvm(shape))
            }
            _ => Err(diag(
                "extern ABI: unsupported type (native scalar, pointer, function, or layout)",
            )),
        }
    }

    /// F06.03: `declare retty @name(paramtys…)`
    pub(super) fn emit_extern_declare(
        &mut self,
        name: &str,
        params: &[Type],
        ret: Option<Type>,
    ) -> Result<(), Diagnostic> {
        let ret_ty = match ret {
            None => "void".to_string(),
            Some(t) => self.llvm_abi_spelling(t)?,
        };
        let mut sig = format!("declare {ret_ty} @{name}(");
        for (i, p) in params.iter().enumerate() {
            if i > 0 {
                sig.push_str(", ");
            }
            sig.push_str(&self.llvm_abi_spelling(*p)?);
        }
        sig.push(')');
        writeln!(self.out, "{sig}").ok();
        Ok(())
    }

    pub(super) fn emit_function(
        &mut self,
        local: LocalId,
        params: &[Param],
        body: &[Stmt],
    ) -> Result<(), Diagnostic> {
        let fn_name = self
            .fn_names
            .get(&local)
            .cloned()
            .ok_or_else(|| diag("internal: missing function name"))?;

        let mut param_tys = Vec::new();
        let mut param_ids = Vec::new();
        for p in params {
            if p.rest || p.default.is_some() {
                return Err(diag("native scalars: rest/default params not supported"));
            }
            let Pattern::Local(id) = &p.pattern else {
                return Err(diag("native scalars: only simple ident params supported"));
            };
            let ty = self.local_scalar(*id)?;
            param_tys.push(ty);
            param_ids.push(*id);
        }

        // Infer return type from first Return with a value, else i32.
        let ret_ty = infer_return_scalar(body).unwrap_or(Scalar(NativeType::I32));

        let mut sig = String::new();
        write!(sig, "define {} @{fn_name}(", ret_ty.llvm_ty()).ok();
        for (i, (id, ty)) in param_ids.iter().zip(param_tys.iter()).enumerate() {
            if i > 0 {
                sig.push_str(", ");
            }
            write!(sig, "{} %p{}", ty.llvm_ty(), id.0).ok();
        }
        sig.push(')');

        // Save main state.
        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_label = self.label;
        let saved_params = std::mem::take(&mut self.params);
        let saved_allocas = std::mem::take(&mut self.allocas);
        let saved_print = std::mem::take(&mut self.print_order);

        self.tmp = 0;
        self.label = 0;
        self.params.clear();
        self.allocas.clear();

        for (id, ty) in param_ids.iter().zip(param_tys.iter()) {
            self.params.insert(*id, (format!("%p{}", id.0), *ty));
        }

        // Allocas for locals declared inside the function (and copy params to allocas for mutability).
        let mut pre = String::new();
        for (id, ty) in param_ids.iter().zip(param_tys.iter()) {
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(*id, ptr.clone());
            writeln!(
                pre,
                "  {ptr} = alloca {}, align {}",
                ty.llvm_ty(),
                ty.align()
            )
            .ok();
            writeln!(pre, "  store {} %p{}, ptr {ptr}", ty.llvm_ty(), id.0).ok();
            // Params are also addressable via alloca now.
            self.params.remove(id);
        }

        // Collect locals used in body that aren't params.
        let mut body_locals = Vec::new();
        collect_declared_locals(body, &mut body_locals);
        for id in body_locals {
            if self.allocas.contains_key(&id) {
                continue;
            }
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(id, ptr.clone());
            if let Ok(ty) = self.local_scalar(id) {
                writeln!(
                    pre,
                    "  {ptr} = alloca {}, align {}",
                    ty.llvm_ty(),
                    ty.align()
                )
                .ok();
            } else if self
                .locals
                .get(&id)
                .is_some_and(|l| matches!(l.ty, Type::Ptr(_)))
            {
                writeln!(pre, "  {ptr} = alloca ptr, align 8").ok();
            } else if let Some(shape) = self.local_layout(id) {
                writeln!(
                    pre,
                    "  {ptr} = alloca {}, align {}",
                    llvm_layout_ty(shape),
                    layout_align(shape)
                )
                .ok();
            } else {
                return Err(diag("native scalars: unsupported local type in function"));
            }
        }

        for stmt in body {
            self.emit_stmt(stmt)?;
        }

        // Ensure terminator.
        if !self.body_ends_with_terminator() {
            writeln!(
                self.body,
                "  ret {} {}",
                ret_ty.llvm_ty(),
                ret_ty.zero_const()
            )
            .ok();
        }

        writeln!(self.out, "{sig} {{").ok();
        writeln!(self.out, "entry:").ok();
        write!(self.out, "{pre}").ok();
        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.label = saved_label;
        self.params = saved_params;
        self.allocas = saved_allocas;
        self.print_order = saved_print;
        Ok(())
    }

    pub(super) fn body_ends_with_terminator(&self) -> bool {
        for line in self.body.lines().rev() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            return t.starts_with("ret ") || t.starts_with("br ");
        }
        false
    }

    pub(super) fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let ptr = self
                    .allocas
                    .get(local)
                    .cloned()
                    .ok_or_else(|| diag("internal: missing alloca for local"))?;
                if let Ok(ty) = self.local_scalar(*local) {
                    if let Some(init) = init {
                        let v = self.emit_expr(init, Some(ty))?;
                        writeln!(self.body, "  store {} {v}, ptr {ptr}", ty.llvm_ty()).ok();
                    } else {
                        writeln!(
                            self.body,
                            "  store {} {}, ptr {ptr}",
                            ty.llvm_ty(),
                            ty.zero_const()
                        )
                        .ok();
                    }
                } else if self
                    .locals
                    .get(local)
                    .is_some_and(|l| matches!(l.ty, Type::Ptr(_)))
                {
                    if let Some(init) = init {
                        let v = self.emit_ptr_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    } else {
                        writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
                    }
                    // Pointers are not printed at end of main.
                    return Ok(());
                } else if let Some(shape) = self.local_layout(*local).cloned() {
                    let layout_ty = llvm_layout_ty(&shape);
                    let fields: Vec<Scalar> = shape
                        .props
                        .iter()
                        .map(|(_, fty)| {
                            let Type::Native(n) = *fty else {
                                return Err(diag("native layout: non-native field"));
                            };
                            Ok(Scalar(n))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if let Some(init) = init {
                        self.emit_store_layout(&ptr, &shape, init)?;
                    } else {
                        // Zero-init each field.
                        for (i, sc) in fields.iter().enumerate() {
                            let gep = self.fresh_tmp();
                            writeln!(
                                self.body,
                                "  {gep} = getelementptr inbounds {layout_ty}, ptr {ptr}, i32 0, i32 {i}"
                            )
                            .ok();
                            writeln!(
                                self.body,
                                "  store {} {}, ptr {gep}",
                                sc.llvm_ty(),
                                sc.zero_const()
                            )
                            .ok();
                        }
                    }
                } else {
                    return Err(diag(
                        "native scalars: declare needs scalar, layout, or pointer local",
                    ));
                }
                // Main tracks declares for end-of-program print; function emit uses a
                // saved empty print_order that is discarded.
                if !self.print_order.contains(local) {
                    self.print_order.push(*local);
                }
                Ok(())
            }
            Stmt::Expr { expr } => {
                let _ = self.emit_expr(expr, None)?;
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Return { value } => {
                if let Some(v) = value {
                    let sty = scalar_of_type(v.ty()).ok_or_else(|| {
                        diag("native scalars: return value must be a native scalar")
                    })?;
                    let val = self.emit_expr(v, Some(sty))?;
                    writeln!(self.body, "  ret {} {val}", sty.llvm_ty()).ok();
                } else {
                    writeln!(self.body, "  ret i32 0").ok();
                }
                // Unreachable padding label so subsequent code is valid if any.
                let lab = self.fresh_label("after_ret");
                writeln!(self.body, "{lab}:").ok();
                Ok(())
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                let cond = self.emit_bool(test)?;
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
                let head = self.fresh_label("while_head");
                let bod = self.fresh_label("while_body");
                let end = self.fresh_label("while_end");
                writeln!(self.body, "  br label %{head}").ok();
                writeln!(self.body, "{head}:").ok();
                let cond = self.emit_bool(test)?;
                writeln!(self.body, "  br i1 {cond}, label %{bod}, label %{end}").ok();
                writeln!(self.body, "{bod}:").ok();
                self.emit_stmt(body)?;
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{head}").ok();
                }
                writeln!(self.body, "{end}:").ok();
                Ok(())
            }
            Stmt::Function { .. } | Stmt::ExternFunction { .. } => Ok(()), // emitted separately
            other => Err(diag(&format!(
                "native scalars: unsupported statement {other:?}"
            ))),
        }
    }

    pub(super) fn emit_print_local(&mut self, id: LocalId) -> Result<(), Diagnostic> {
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("internal: print missing alloca"))?;
        if let Ok(ty) = self.local_scalar(id) {
            let v = self.fresh_tmp();
            writeln!(self.body, "  {v} = load {}, ptr {ptr}", ty.llvm_ty()).ok();
            return self.emit_print_scalar_value(ty, &v);
        }
        if let Some(shape) = self.local_layout(id) {
            let layout_ty = llvm_layout_ty(shape);
            let fields: Vec<Scalar> = shape
                .props
                .iter()
                .map(|(_, fty)| {
                    let Type::Native(n) = *fty else {
                        return Err(diag("native layout: non-native field"));
                    };
                    Ok(Scalar(n))
                })
                .collect::<Result<Vec<_>, _>>()?;
            for (i, sc) in fields.iter().enumerate() {
                let gep = self.fresh_tmp();
                writeln!(
                    self.body,
                    "  {gep} = getelementptr inbounds {layout_ty}, ptr {ptr}, i32 0, i32 {i}"
                )
                .ok();
                let v = self.fresh_tmp();
                writeln!(self.body, "  {v} = load {}, ptr {gep}", sc.llvm_ty()).ok();
                self.emit_print_scalar_value(*sc, &v)?;
            }
            return Ok(());
        }
        Err(diag(
            "native scalars: print only supports native scalars/layouts",
        ))
    }

    pub(super) fn emit_print_scalar_value(&mut self, ty: Scalar, v: &str) -> Result<(), Diagnostic> {
        let n = ty.native();
        if n.is_bool() {
            let ext = self.fresh_tmp();
            writeln!(self.body, "  {ext} = zext i1 {v} to i8").ok();
            writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {ext}"))).ok();
        } else if n.is_float() {
            let d = if n == NativeType::F32 {
                let t = self.fresh_tmp();
                writeln!(self.body, "  {t} = fpext float {v} to double").ok();
                t
            } else {
                v.to_string()
            };
            writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {d}"))).ok();
        } else {
            let ext = self.fresh_tmp();
            if n.bit_width() < 64 {
                if n.is_signed() {
                    writeln!(self.body, "  {ext} = sext {} {v} to i64", llvm_ty(n)).ok();
                } else {
                    writeln!(self.body, "  {ext} = zext {} {v} to i64", llvm_ty(n)).ok();
                }
            } else {
                writeln!(self.body, "  {ext} = add i64 {v}, 0").ok();
            }
            if n.is_signed() {
                writeln!(self.body, "  {}", PRINT_I64.call(&format!("i64 {ext}"))).ok();
            } else {
                writeln!(self.body, "  {}", PRINT_U64.call(&format!("i64 {ext}"))).ok();
            }
        }
        Ok(())
    }

    pub(super) fn emit_bool(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr.ty() {
            Type::Boolean | Type::Native(NativeType::Bool) => {
                self.emit_expr(expr, Some(Scalar(NativeType::Bool)))
            }
            Type::Native(n) if n.is_int() => {
                let v = self.emit_expr(expr, Some(Scalar(n)))?;
                let t = self.fresh_tmp();
                writeln!(self.body, "  {t} = icmp ne {} {v}, 0", llvm_ty(n)).ok();
                Ok(t)
            }
            Type::Native(n) if n.is_float() => {
                let v = self.emit_expr(expr, Some(Scalar(n)))?;
                let t = self.fresh_tmp();
                writeln!(
                    self.body,
                    "  {t} = fcmp one {} {v}, 0.000000e+00",
                    llvm_ty(n)
                )
                .ok();
                Ok(t)
            }
            Type::Number => {
                let v = self.emit_expr(expr, Some(Scalar(NativeType::F64)))?;
                let t = self.fresh_tmp();
                writeln!(self.body, "  {t} = fcmp one double {v}, 0.000000e+00").ok();
                Ok(t)
            }
            _ => Err(diag(
                "native scalars: condition must be bool or native numeric",
            )),
        }
    }
}
