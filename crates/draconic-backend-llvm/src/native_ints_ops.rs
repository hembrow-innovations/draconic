use std::fmt::Write as _;

use super::*;

impl<'a> super::Emitter<'a> {
    pub(super) fn emit_binary(
        &mut self,
        left: &Expr,
        op: BinaryOp,
        right: &Expr,
        ty: &Type,
        expect: Option<Scalar>,
    ) -> Result<String, Diagnostic> {
        // Comparisons → i1
        if matches!(
            op,
            BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::EqEq
                | BinaryOp::NotEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEqEq
        ) {
            let sty =
                scalar_operand_ty(left, right, expect.filter(|s| s.is_int() || s.is_float()))?;
            if sty.is_bool() {
                return Err(diag("native scalars: compare needs numeric operands"));
            }
            let nty = sty.native();
            let l = self.emit_expr(left, Some(sty))?;
            let r = self.emit_expr(right, Some(sty))?;
            let t = self.fresh_tmp();
            if nty.is_float() {
                let pred = match op {
                    BinaryOp::EqEq | BinaryOp::EqEqEq => "oeq",
                    BinaryOp::NotEq | BinaryOp::NotEqEq => "one",
                    BinaryOp::Lt => "olt",
                    BinaryOp::LtEq => "ole",
                    BinaryOp::Gt => "ogt",
                    BinaryOp::GtEq => "oge",
                    _ => unreachable!(),
                };
                writeln!(self.body, "  {t} = fcmp {pred} {} {l}, {r}", llvm_ty(nty)).ok();
            } else {
                let pred = match op {
                    BinaryOp::EqEq | BinaryOp::EqEqEq => "eq",
                    BinaryOp::NotEq | BinaryOp::NotEqEq => "ne",
                    BinaryOp::Lt => {
                        if nty.is_signed() {
                            "slt"
                        } else {
                            "ult"
                        }
                    }
                    BinaryOp::LtEq => {
                        if nty.is_signed() {
                            "sle"
                        } else {
                            "ule"
                        }
                    }
                    BinaryOp::Gt => {
                        if nty.is_signed() {
                            "sgt"
                        } else {
                            "ugt"
                        }
                    }
                    BinaryOp::GtEq => {
                        if nty.is_signed() {
                            "sge"
                        } else {
                            "uge"
                        }
                    }
                    _ => unreachable!(),
                };
                writeln!(self.body, "  {t} = icmp {pred} {} {l}, {r}", llvm_ty(nty)).ok();
            }
            return Ok(t);
        }

        if matches!(op, BinaryOp::Comma) {
            let _ = self.emit_expr(left, expect)?;
            return self.emit_expr(right, expect);
        }

        let sty = match scalar_of_type(*ty) {
            Some(s) if !s.is_bool() => s,
            _ => scalar_operand_ty(left, right, expect)?,
        };
        if sty.is_bool() {
            return Err(diag("native scalars: arithmetic needs numeric type"));
        }
        let nty = sty.native();
        let l = self.emit_expr(left, Some(sty))?;
        let r = self.emit_expr(right, Some(sty))?;
        let t = self.fresh_tmp();
        let ll = llvm_ty(nty);
        if nty.is_float() {
            match op {
                BinaryOp::Add => writeln!(self.body, "  {t} = fadd {ll} {l}, {r}").ok(),
                BinaryOp::Sub => writeln!(self.body, "  {t} = fsub {ll} {l}, {r}").ok(),
                BinaryOp::Mul => writeln!(self.body, "  {t} = fmul {ll} {l}, {r}").ok(),
                BinaryOp::Div => writeln!(self.body, "  {t} = fdiv {ll} {l}, {r}").ok(),
                BinaryOp::Rem => writeln!(self.body, "  {t} = frem {ll} {l}, {r}").ok(),
                _ => {
                    return Err(diag(&format!(
                        "native scalars: unsupported float binary operator {op}"
                    )))
                }
            };
        } else {
            match op {
                BinaryOp::Add => writeln!(self.body, "  {t} = add {ll} {l}, {r}").ok(),
                BinaryOp::Sub => writeln!(self.body, "  {t} = sub {ll} {l}, {r}").ok(),
                BinaryOp::Mul => writeln!(self.body, "  {t} = mul {ll} {l}, {r}").ok(),
                BinaryOp::Div => {
                    if nty.is_signed() {
                        writeln!(self.body, "  {t} = sdiv {ll} {l}, {r}").ok()
                    } else {
                        writeln!(self.body, "  {t} = udiv {ll} {l}, {r}").ok()
                    }
                }
                BinaryOp::Rem => {
                    if nty.is_signed() {
                        writeln!(self.body, "  {t} = srem {ll} {l}, {r}").ok()
                    } else {
                        writeln!(self.body, "  {t} = urem {ll} {l}, {r}").ok()
                    }
                }
                BinaryOp::BitAnd => writeln!(self.body, "  {t} = and {ll} {l}, {r}").ok(),
                BinaryOp::BitOr => writeln!(self.body, "  {t} = or {ll} {l}, {r}").ok(),
                BinaryOp::BitXor => writeln!(self.body, "  {t} = xor {ll} {l}, {r}").ok(),
                BinaryOp::Shl => writeln!(self.body, "  {t} = shl {ll} {l}, {r}").ok(),
                BinaryOp::Shr => {
                    if nty.is_signed() {
                        writeln!(self.body, "  {t} = ashr {ll} {l}, {r}").ok()
                    } else {
                        writeln!(self.body, "  {t} = lshr {ll} {l}, {r}").ok()
                    }
                }
                _ => {
                    return Err(diag(&format!(
                        "native scalars: unsupported binary operator {op}"
                    )))
                }
            };
        }
        Ok(t)
    }

    pub(super) fn emit_assign(
        &mut self,
        target: &AssignTarget,
        op: AssignOp,
        value: &Expr,
        ty: &Type,
    ) -> Result<String, Diagnostic> {
        // N03.03: `*p = v` store through pointer (simple `=` only).
        if let AssignTarget::Deref(ptr_expr) = target {
            if !matches!(op, AssignOp::Eq) {
                return Err(diag(
                    "native pointers: only simple `=` store through pointer supported",
                ));
            }
            let sty = match scalar_of_type(*ty) {
                Some(s) => s,
                None => match ptr_expr.ty() {
                    Type::Ptr(n) => Scalar(n),
                    _ => return Err(diag("native pointers: store value must be a native scalar")),
                },
            };
            let dest = self.emit_ptr_expr(ptr_expr)?;
            let rhs = self.emit_expr(value, Some(sty))?;
            writeln!(self.body, "  store {} {rhs}, ptr {dest}", sty.llvm_ty()).ok();
            return Ok(rhs);
        }

        let AssignTarget::Local(id) = target else {
            return Err(diag("native scalars: only local assignment supported"));
        };
        // Pointer local assign: `p = &x` / `p = q`.
        if matches!(self.locals.get(id).map(|l| l.ty), Some(Type::Ptr(_))) {
            if !matches!(op, AssignOp::Eq) {
                return Err(diag("native pointers: only simple `=` to pointer local"));
            }
            let slot = self
                .allocas
                .get(id)
                .cloned()
                .ok_or_else(|| diag("internal: pointer assign missing alloca"))?;
            let rhs = self.emit_ptr_expr(value)?;
            writeln!(self.body, "  store ptr {rhs}, ptr {slot}").ok();
            return Ok(rhs);
        }
        // Prefer the local's storage type so `let c: i32; c = 1` stores i32 even
        // when the RHS expression is still typed `number` in IR (contextual lit).
        let sty = match self.local_scalar(*id) {
            Ok(s) => s,
            Err(_) => scalar_of_type(*ty)
                .ok_or_else(|| diag("native scalars: assignment needs a native scalar local"))?,
        };
        let ptr = self
            .allocas
            .get(id)
            .cloned()
            .ok_or_else(|| diag("internal: assign missing alloca"))?;

        let rhs = if matches!(op, AssignOp::Eq) {
            self.emit_expr(value, Some(sty))?
        } else {
            if sty.is_bool() {
                return Err(diag("native scalars: compound assign needs numeric local"));
            }
            let nty = sty.native();
            let cur = self.fresh_tmp();
            writeln!(self.body, "  {cur} = load {}, ptr {ptr}", llvm_ty(nty)).ok();
            let rhs_v = self.emit_expr(value, Some(sty))?;
            let t = self.fresh_tmp();
            let ll = llvm_ty(nty);
            if nty.is_float() {
                match op {
                    AssignOp::AddEq => writeln!(self.body, "  {t} = fadd {ll} {cur}, {rhs_v}").ok(),
                    AssignOp::SubEq => writeln!(self.body, "  {t} = fsub {ll} {cur}, {rhs_v}").ok(),
                    AssignOp::MulEq => writeln!(self.body, "  {t} = fmul {ll} {cur}, {rhs_v}").ok(),
                    AssignOp::DivEq => writeln!(self.body, "  {t} = fdiv {ll} {cur}, {rhs_v}").ok(),
                    AssignOp::RemEq => writeln!(self.body, "  {t} = frem {ll} {cur}, {rhs_v}").ok(),
                    _ => {
                        return Err(diag(&format!(
                            "native scalars: unsupported float compound assign {op:?}"
                        )))
                    }
                };
            } else {
                match op {
                    AssignOp::AddEq => writeln!(self.body, "  {t} = add {ll} {cur}, {rhs_v}").ok(),
                    AssignOp::SubEq => writeln!(self.body, "  {t} = sub {ll} {cur}, {rhs_v}").ok(),
                    AssignOp::MulEq => writeln!(self.body, "  {t} = mul {ll} {cur}, {rhs_v}").ok(),
                    AssignOp::DivEq => {
                        if nty.is_signed() {
                            writeln!(self.body, "  {t} = sdiv {ll} {cur}, {rhs_v}").ok()
                        } else {
                            writeln!(self.body, "  {t} = udiv {ll} {cur}, {rhs_v}").ok()
                        }
                    }
                    AssignOp::RemEq => {
                        if nty.is_signed() {
                            writeln!(self.body, "  {t} = srem {ll} {cur}, {rhs_v}").ok()
                        } else {
                            writeln!(self.body, "  {t} = urem {ll} {cur}, {rhs_v}").ok()
                        }
                    }
                    AssignOp::BitAndEq => {
                        writeln!(self.body, "  {t} = and {ll} {cur}, {rhs_v}").ok()
                    }
                    AssignOp::BitOrEq => writeln!(self.body, "  {t} = or {ll} {cur}, {rhs_v}").ok(),
                    AssignOp::BitXorEq => {
                        writeln!(self.body, "  {t} = xor {ll} {cur}, {rhs_v}").ok()
                    }
                    AssignOp::ShlEq => writeln!(self.body, "  {t} = shl {ll} {cur}, {rhs_v}").ok(),
                    AssignOp::ShrEq => {
                        if nty.is_signed() {
                            writeln!(self.body, "  {t} = ashr {ll} {cur}, {rhs_v}").ok()
                        } else {
                            writeln!(self.body, "  {t} = lshr {ll} {cur}, {rhs_v}").ok()
                        }
                    }
                    _ => {
                        return Err(diag(&format!(
                            "native scalars: unsupported compound assign {op:?}"
                        )))
                    }
                };
            }
            t
        };
        writeln!(self.body, "  store {} {rhs}, ptr {ptr}", sty.llvm_ty()).ok();
        Ok(rhs)
    }

    pub(super) fn emit_update(
        &mut self,
        op: UpdateOp,
        target: &UpdateTarget,
        prefix: bool,
        ty: &Type,
    ) -> Result<String, Diagnostic> {
        let UpdateTarget::Local(id) = target else {
            return Err(diag("native scalars: only local ++/-- supported"));
        };
        let sty = match scalar_of_type(*ty) {
            Some(s) => s,
            None => self.local_scalar(*id)?,
        };
        if sty.is_bool() || sty.is_float() {
            return Err(diag("native scalars: ++/-- needs integer local"));
        }
        let nty = sty.native();
        let ptr = self
            .allocas
            .get(id)
            .cloned()
            .ok_or_else(|| diag("internal: update missing alloca"))?;
        let cur = self.fresh_tmp();
        writeln!(self.body, "  {cur} = load {}, ptr {ptr}", llvm_ty(nty)).ok();
        let next = self.fresh_tmp();
        match op {
            UpdateOp::Inc => writeln!(self.body, "  {next} = add {} {cur}, 1", llvm_ty(nty)).ok(),
            UpdateOp::Dec => writeln!(self.body, "  {next} = sub {} {cur}, 1", llvm_ty(nty)).ok(),
        };
        writeln!(self.body, "  store {} {next}, ptr {ptr}", llvm_ty(nty)).ok();
        if prefix {
            Ok(next)
        } else {
            Ok(cur)
        }
    }

    pub(super) fn function_sig(&self, id: LocalId) -> Result<(Vec<Scalar>, Scalar), Diagnostic> {
        for stmt in &self.module.body {
            if let Stmt::Function {
                local,
                params,
                body,
                ..
            } = stmt
            {
                if *local == id {
                    let mut ptys = Vec::new();
                    for p in params {
                        let Pattern::Local(pid) = &p.pattern else {
                            return Err(diag("native scalars: only simple params"));
                        };
                        ptys.push(self.local_scalar(*pid)?);
                    }
                    let ret = infer_return_scalar(body).unwrap_or(Scalar(NativeType::I32));
                    return Ok((ptys, ret));
                }
            }
        }
        Err(diag("native scalars: function not found for signature"))
    }

    /// F06.03: call an `extern "C"` symbol with ABI param/return types.
    pub(super) fn emit_extern_call(
        &mut self,
        ext: &ExternAbi,
        args: &[Arg],
        call_ty: Type,
        expect: Option<Scalar>,
    ) -> Result<String, Diagnostic> {
        if ext.params.len() != args.len() {
            return Err(diag("native FFI: extern call arity mismatch"));
        }
        let mut arg_parts = Vec::new();
        for (arg, pty) in args.iter().zip(ext.params.iter()) {
            let Arg::Expr(e) = arg else {
                return Err(diag("native FFI: spread args not supported on extern call"));
            };
            match *pty {
                Type::Native(n) => {
                    let v = self.emit_expr(e, Some(Scalar(n)))?;
                    arg_parts.push(format!("{} {v}", llvm_ty(n)));
                }
                Type::Boolean => {
                    let v = self.emit_expr(e, Some(Scalar(NativeType::Bool)))?;
                    arg_parts.push(format!("i1 {v}"));
                }
                Type::Ptr(_) => {
                    let v = self.emit_ptr_expr(e)?;
                    arg_parts.push(format!("ptr {v}"));
                }
                Type::Function => {
                    let v = self.emit_fnptr_expr(e)?;
                    arg_parts.push(format!("ptr {v}"));
                }
                Type::Shape(_) => {
                    let ptr = self.emit_layout_arg_ptr(e)?;
                    let shape = native_layout_of(self.module, *pty).ok_or_else(|| {
                        diag("native FFI: extern layout param is not a native layout")
                    })?;
                    let abi = layout_abi_llvm(shape);
                    let v = self.fresh_tmp();
                    writeln!(self.body, "  {v} = load {abi}, ptr {ptr}").ok();
                    arg_parts.push(format!("{abi} {v}"));
                }
                _ => return Err(diag(
                    "native FFI: extern param must be native scalar, pointer, function, or layout",
                )),
            }
        }
        let ret_llvm = match ext.ret {
            None => None,
            Some(t) => Some(self.llvm_abi_spelling(t)?),
        };
        match ret_llvm {
            None => {
                write!(self.body, "  call void @{}(", ext.name).ok();
                for (i, p) in arg_parts.iter().enumerate() {
                    if i > 0 {
                        self.body.push_str(", ");
                    }
                    self.body.push_str(p);
                }
                writeln!(self.body, ")").ok();
                // Void call used as value → zero of expected/context type when needed.
                if let Some(s) = expect.or_else(|| scalar_of_type(call_ty)) {
                    Ok(s.zero_const().to_string())
                } else {
                    Ok("0".into())
                }
            }
            Some(retty) => {
                let t = self.fresh_tmp();
                write!(self.body, "  {t} = call {retty} @{}(", ext.name).ok();
                for (i, p) in arg_parts.iter().enumerate() {
                    if i > 0 {
                        self.body.push_str(", ");
                    }
                    self.body.push_str(p);
                }
                writeln!(self.body, ")").ok();
                Ok(t)
            }
        }
    }

    pub(super) fn local_scalar(&self, id: LocalId) -> Result<Scalar, Diagnostic> {
        let local = self
            .locals
            .get(&id)
            .ok_or_else(|| diag("internal: unknown local"))?;
        match local.ty {
            Type::Native(n) => Ok(Scalar(n)),
            Type::Boolean => Ok(Scalar(NativeType::Bool)),
            Type::Number => Ok(Scalar(NativeType::F64)),
            _ => Err(diag(&format!(
                "native scalars: local `{}` is not a native scalar",
                local.name
            ))),
        }
    }

    /// Dual-worlds / width change: convert unboxed scalar `v` from `from` to `to`.
    pub(super) fn coerce_scalar(&mut self, v: &str, from: Scalar, to: Scalar) -> Result<String, Diagnostic> {
        if from == to {
            return Ok(v.to_string());
        }
        if from.is_bool() || to.is_bool() {
            return Err(diag(
                "native scalars: cannot coerce bool across dual-worlds numeric boundary",
            ));
        }
        let from_n = from.native();
        let to_n = to.native();
        let t = self.fresh_tmp();
        match (from_n.is_float(), to_n.is_float()) {
            (true, true) => {
                if from_n.bit_width() < to_n.bit_width() {
                    writeln!(
                        self.body,
                        "  {t} = fpext {} {v} to {}",
                        llvm_ty(from_n),
                        llvm_ty(to_n)
                    )
                    .ok();
                } else if from_n.bit_width() > to_n.bit_width() {
                    writeln!(
                        self.body,
                        "  {t} = fptrunc {} {v} to {}",
                        llvm_ty(from_n),
                        llvm_ty(to_n)
                    )
                    .ok();
                } else {
                    return Ok(v.to_string());
                }
            }
            (false, true) => {
                if from_n.is_signed() {
                    writeln!(
                        self.body,
                        "  {t} = sitofp {} {v} to {}",
                        llvm_ty(from_n),
                        llvm_ty(to_n)
                    )
                    .ok();
                } else {
                    writeln!(
                        self.body,
                        "  {t} = uitofp {} {v} to {}",
                        llvm_ty(from_n),
                        llvm_ty(to_n)
                    )
                    .ok();
                }
            }
            (true, false) => {
                if to_n.is_signed() {
                    writeln!(
                        self.body,
                        "  {t} = fptosi {} {v} to {}",
                        llvm_ty(from_n),
                        llvm_ty(to_n)
                    )
                    .ok();
                } else {
                    writeln!(
                        self.body,
                        "  {t} = fptoui {} {v} to {}",
                        llvm_ty(from_n),
                        llvm_ty(to_n)
                    )
                    .ok();
                }
            }
            (false, false) => {
                let from_w = from_n.bit_width();
                let to_w = to_n.bit_width();
                if from_w == to_w {
                    return Ok(v.to_string());
                } else if from_w < to_w {
                    if from_n.is_signed() {
                        writeln!(
                            self.body,
                            "  {t} = sext {} {v} to {}",
                            llvm_ty(from_n),
                            llvm_ty(to_n)
                        )
                        .ok();
                    } else {
                        writeln!(
                            self.body,
                            "  {t} = zext {} {v} to {}",
                            llvm_ty(from_n),
                            llvm_ty(to_n)
                        )
                        .ok();
                    }
                } else {
                    writeln!(
                        self.body,
                        "  {t} = trunc {} {v} to {}",
                        llvm_ty(from_n),
                        llvm_ty(to_n)
                    )
                    .ok();
                }
            }
        }
        Ok(t)
    }

    pub(super) fn local_layout(&self, id: LocalId) -> Option<&ObjectShape> {
        let local = self.locals.get(&id)?;
        native_layout_of(self.module, local.ty)
    }

    /// Emit a pointer-typed expression (`*T` value): local load, `&local`, or copy.
    pub(super) fn emit_ptr_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Local { id, .. } => {
                if let Some((pname, _)) = self.params.get(id) {
                    return Ok(pname.clone());
                }
                let slot = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("native pointers: local missing alloca"))?;
                let t = self.fresh_tmp();
                writeln!(self.body, "  {t} = load ptr, ptr {slot}").ok();
                Ok(t)
            }
            Expr::Unary {
                op: UnaryOp::Ref,
                arg,
                ..
            } => match arg.as_ref() {
                Expr::Local { id, .. } => self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("native pointers: address-of needs stack local")),
                _ => Err(diag(
                    "native pointers: address-of only supports direct locals",
                )),
            },
            Expr::Null { .. } => Ok("null".into()),
            _ => Err(diag(&format!(
                "native pointers: unsupported pointer expression {expr:?}"
            ))),
        }
    }

    pub(super) fn emit_layout_arg_ptr(&self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Local { id, .. } => self
                .allocas
                .get(id)
                .cloned()
                .ok_or_else(|| diag("native FFI: layout arg missing alloca")),
            _ => Err(diag("native FFI: pass a layout local by value")),
        }
    }

    pub(super) fn emit_fnptr_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Local { id, .. } => self
                .fn_names
                .get(id)
                .map(|name| format!("@{name}"))
                .ok_or_else(|| diag("native FFI: function pointer arg is not a function")),
            Expr::Null { .. } => Ok("null".into()),
            _ => Err(diag("native FFI: function pointer arg must be a function")),
        }
    }

    pub(super) fn fresh_tmp(&mut self) -> String {
        let n = self.tmp;
        self.tmp += 1;
        format!("%t{n}")
    }

    pub(super) fn fresh_label(&mut self, prefix: &str) -> String {
        let n = self.label;
        self.label += 1;
        format!("{prefix}{n}")
    }
}
