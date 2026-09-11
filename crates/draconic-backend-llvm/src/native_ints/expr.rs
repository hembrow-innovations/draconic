use std::fmt::Write as _;

use super::*;

impl<'a> super::Emitter<'a> {
    pub(super) fn emit_expr(
        &mut self,
        expr: &Expr,
        expect: Option<Scalar>,
    ) -> Result<String, Diagnostic> {
        let v = self.emit_expr_uncast(expr, expect)?;
        let Some(want) = expect else {
            return Ok(v);
        };
        // Number literals are formatted directly in `want` width (contextual typing).
        if matches!(expr, Expr::Number { .. }) {
            return Ok(v);
        }
        // Dual-worlds: when the IR expression type is a different unboxed scalar
        // than the destination, insert an explicit cast (`as` is erased at IR).
        let Some(got) = scalar_of_type(expr.ty()) else {
            return Ok(v);
        };
        if got == want || got.is_bool() || want.is_bool() {
            return Ok(v);
        }
        self.coerce_scalar(&v, got, want)
    }

    pub(super) fn emit_expr_uncast(
        &mut self,
        expr: &Expr,
        expect: Option<Scalar>,
    ) -> Result<String, Diagnostic> {
        match expr {
            Expr::Local { id, ty } => {
                if let Some((pname, _)) = self.params.get(id) {
                    return Ok(pname.clone());
                }
                if matches!(ty, Type::Ptr(_)) {
                    return self.emit_ptr_expr(expr);
                }
                if let Some(ptr) = self.allocas.get(id).cloned() {
                    let sty = scalar_of_type(*ty).unwrap_or(self.local_scalar(*id)?);
                    let t = self.fresh_tmp();
                    writeln!(self.body, "  {t} = load {}, ptr {ptr}", sty.llvm_ty()).ok();
                    return Ok(t);
                }
                // Function reference used as value — not supported except as callee.
                Err(diag("native scalars: bare function value not supported"))
            }
            Expr::Boolean { value, .. } => Ok(if *value { "1".into() } else { "0".into() }),
            Expr::Number { raw, ty } => {
                // Prefer destination/native context so `let x: i32 = 0b1100` formats as
                // int (binary/hex literals are not valid float constants). Bare
                // `number` without expect is dual-worlds IEEE double.
                let sty = match (ty, expect) {
                    (Type::Native(n), _) if !n.is_bool() => Scalar(*n),
                    (_, Some(s)) if !s.is_bool() => s,
                    (Type::Number, _) => Scalar(NativeType::F64),
                    _ => {
                        return Err(diag(
                            "native scalars: number literal needs native numeric context",
                        ))
                    }
                };
                let nty = sty.native();
                if nty.is_float() {
                    Ok(format_float_const(raw, nty)?)
                } else {
                    Ok(format_int_const(raw, nty)?)
                }
            }
            Expr::Unary { op, arg, ty } => {
                if matches!(op, UnaryOp::Not) {
                    let a = self.emit_bool(arg)?;
                    let t = self.fresh_tmp();
                    writeln!(self.body, "  {t} = xor i1 {a}, true").ok();
                    return Ok(t);
                }
                // N03.03: `&x` → pointer value (address of local).
                if matches!(op, UnaryOp::Ref) {
                    return self.emit_ptr_expr(expr);
                }
                // N03.03: `*p` → load pointee scalar.
                if matches!(op, UnaryOp::Deref) {
                    let ptr_v = self.emit_ptr_expr(arg)?;
                    let sty = match ty {
                        Type::Native(n) => Scalar(*n),
                        Type::Boolean => Scalar(NativeType::Bool),
                        _ => {
                            return Err(diag(
                                "native pointers: dereference result must be native scalar",
                            ))
                        }
                    };
                    let t = self.fresh_tmp();
                    writeln!(self.body, "  {t} = load {}, ptr {ptr_v}", sty.llvm_ty()).ok();
                    return Ok(t);
                }
                let nty = match ty {
                    Type::Native(n) if !n.is_bool() => *n,
                    Type::Number => NativeType::F64,
                    _ => return Err(diag("native scalars: unary result must be native numeric")),
                };
                let a = self.emit_expr(arg, Some(Scalar(nty)))?;
                let t = self.fresh_tmp();
                match op {
                    UnaryOp::Minus if nty.is_float() => {
                        writeln!(self.body, "  {t} = fneg {} {a}", llvm_ty(nty)).ok();
                    }
                    UnaryOp::Minus => {
                        writeln!(self.body, "  {t} = sub {} 0, {a}", llvm_ty(nty)).ok();
                    }
                    UnaryOp::BitNot if nty.is_int() => {
                        writeln!(self.body, "  {t} = xor {} {a}, -1", llvm_ty(nty)).ok();
                    }
                    UnaryOp::Plus if nty.is_float() => {
                        writeln!(self.body, "  {t} = fadd {} {a}, 0.000000e+00", llvm_ty(nty)).ok();
                    }
                    UnaryOp::Plus => {
                        writeln!(self.body, "  {t} = add {} {a}, 0", llvm_ty(nty)).ok();
                    }
                    _ => return Err(diag(&format!("native scalars: unsupported unary {op}"))),
                }
                Ok(t)
            }
            Expr::Binary {
                left,
                op,
                right,
                ty,
            } => self.emit_binary(left, *op, right, ty, expect),
            Expr::Assign {
                target,
                op,
                value,
                ty,
            } => self.emit_assign(target, *op, value, ty),
            Expr::Update {
                op,
                target,
                prefix,
                ty,
            } => self.emit_update(*op, target, *prefix, ty),
            Expr::Call {
                callee,
                args,
                optional,
                ty,
            } => {
                if *optional {
                    return Err(diag("native scalars: optional call not supported"));
                }
                let Expr::Local { id, .. } = callee.as_ref() else {
                    return Err(diag("native scalars: only direct function calls supported"));
                };
                // F06.03: direct call to extern "C" uses linkage name + ABI types.
                if let Some(ext) = self.extern_fns.get(id).cloned() {
                    return self.emit_extern_call(&ext, args, *ty, expect);
                }
                let fn_name = self
                    .fn_names
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("native scalars: call to unknown function"))?;
                // Checker currently types non-generic calls as `Any`; prefer the
                // expression type, then expected context, then inferred signature.
                let ret_ty = match scalar_of_type(*ty) {
                    Some(s) => s,
                    None => match expect {
                        Some(n) => n,
                        None => self.function_sig(*id)?.1,
                    },
                };
                let (param_tys, _) = self.function_sig(*id)?;
                if param_tys.len() != args.len() {
                    return Err(diag("native scalars: arity mismatch"));
                }
                let mut arg_parts = Vec::new();
                for (arg, pty) in args.iter().zip(param_tys.iter()) {
                    let Arg::Expr(e) = arg else {
                        return Err(diag("native scalars: spread args not supported"));
                    };
                    let v = self.emit_expr(e, Some(*pty))?;
                    arg_parts.push(format!("{} {v}", pty.llvm_ty()));
                }
                let t = self.fresh_tmp();
                write!(self.body, "  {t} = call {} @{fn_name}(", ret_ty.llvm_ty()).ok();
                for (i, p) in arg_parts.iter().enumerate() {
                    if i > 0 {
                        self.body.push_str(", ");
                    }
                    self.body.push_str(p);
                }
                writeln!(self.body, ")").ok();
                Ok(t)
            }
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ty,
            } => {
                let sty = match scalar_of_type(*ty) {
                    Some(s) => s,
                    None => expect.ok_or_else(|| {
                        diag("native scalars: conditional needs native scalar type")
                    })?,
                };
                let cond = self.emit_bool(test)?;
                let then_l = self.fresh_label("sel_then");
                let else_l = self.fresh_label("sel_else");
                let end_l = self.fresh_label("sel_end");
                let slot = self.fresh_tmp();
                writeln!(
                    self.body,
                    "  {slot} = alloca {}, align {}",
                    sty.llvm_ty(),
                    sty.align()
                )
                .ok();
                writeln!(
                    self.body,
                    "  br i1 {cond}, label %{then_l}, label %{else_l}"
                )
                .ok();
                writeln!(self.body, "{then_l}:").ok();
                let c = self.emit_expr(consequent, Some(sty))?;
                writeln!(self.body, "  store {} {c}, ptr {slot}", sty.llvm_ty()).ok();
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{else_l}:").ok();
                let a = self.emit_expr(alternate, Some(sty))?;
                writeln!(self.body, "  store {} {a}, ptr {slot}", sty.llvm_ty()).ok();
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{end_l}:").ok();
                let t = self.fresh_tmp();
                writeln!(self.body, "  {t} = load {}, ptr {slot}", sty.llvm_ty()).ok();
                Ok(t)
            }
            Expr::Member {
                object,
                property,
                computed,
                optional,
                ty,
            } => {
                if *optional {
                    return Err(diag("native layout: optional member not supported"));
                }
                let key = if *computed {
                    // Fixed-array index: `a[0]` with constant non-neg integer (N03.02).
                    const_index_key(property).ok_or_else(|| {
                        diag("native layout: computed member needs constant integer index")
                    })?
                } else {
                    match property.as_ref() {
                        Expr::String { value, .. } => value.to_string_lossy(),
                        _ => {
                            return Err(diag(
                                "native layout: member property must be a static string key",
                            ))
                        }
                    }
                };
                let (obj_ptr, layout_ty, idx, sc) = {
                    let (obj_ptr, shape) = self.emit_layout_base(object)?;
                    let idx = shape
                        .props
                        .iter()
                        .position(|(n, _)| n == &key)
                        .ok_or_else(|| diag(&format!("native layout: unknown field `{key}`")))?;
                    let field_ty = shape.props[idx].1;
                    let sc = scalar_of_type(field_ty)
                        .or_else(|| scalar_of_type(*ty))
                        .ok_or_else(|| diag("native layout: field must be native scalar"))?;
                    (obj_ptr, llvm_layout_ty(shape), idx, sc)
                };
                let gep = self.fresh_tmp();
                writeln!(
                    self.body,
                    "  {gep} = getelementptr inbounds {layout_ty}, ptr {obj_ptr}, i32 0, i32 {idx}"
                )
                .ok();
                let t = self.fresh_tmp();
                writeln!(self.body, "  {t} = load {}, ptr {gep}", sc.llvm_ty()).ok();
                Ok(t)
            }
            Expr::Object { .. } => Err(diag(
                "native layout: object literal only supported as layout init",
            )),
            Expr::Array { .. } => Err(diag(
                "native layout: array literal only supported as layout init",
            )),
            _ => Err(diag(&format!(
                "native scalars: unsupported expression {expr:?}"
            ))),
        }
    }
}
