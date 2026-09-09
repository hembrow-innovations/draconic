use std::fmt::Write as _;

use super::*;

impl<'a> super::Emitter<'a> {
    /// Pointer to a layout local + its shape (for field GEP).
    pub(super) fn emit_layout_base<'b>(
        &'b self,
        expr: &'b Expr,
    ) -> Result<(String, &'b ObjectShape), Diagnostic> {
        match expr {
            Expr::Local { id, ty } => {
                let shape = native_layout_of(self.module, *ty)
                    .or_else(|| self.local_layout(*id))
                    .ok_or_else(|| diag("native layout: member base is not a layout local"))?;
                let ptr = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("internal: layout local missing alloca"))?;
                Ok((ptr, shape))
            }
            _ => Err(diag(
                "native layout: only direct local field access supported",
            )),
        }
    }

    pub(super) fn emit_store_layout(
        &mut self,
        dest_ptr: &str,
        shape: &ObjectShape,
        init: &Expr,
    ) -> Result<(), Diagnostic> {
        let layout_ty = llvm_layout_ty(shape);
        let field_meta: Vec<(String, Scalar)> = shape
            .props
            .iter()
            .map(|(name, fty)| {
                let Type::Native(n) = *fty else {
                    return Err(diag("native layout: non-native field"));
                };
                Ok((name.clone(), Scalar(n)))
            })
            .collect::<Result<Vec<_>, _>>()?;
        match init {
            Expr::Object { properties, .. } => {
                let mut by_name: HashMap<String, &Expr> = HashMap::new();
                for prop in properties {
                    match prop {
                        ObjectProp::Property {
                            key: ObjectPropKey::Static(k),
                            value,
                        } => {
                            by_name.insert(k.to_string_lossy(), value);
                        }
                        _ => {
                            return Err(diag(
                                "native layout: only static data properties in object init",
                            ))
                        }
                    }
                }
                for (i, (name, sc)) in field_meta.iter().enumerate() {
                    let val_expr = by_name.get(name).ok_or_else(|| {
                        diag(&format!("native layout: missing field `{name}` in init"))
                    })?;
                    let v = self.emit_expr(val_expr, Some(*sc))?;
                    let gep = self.fresh_tmp();
                    writeln!(
                        self.body,
                        "  {gep} = getelementptr inbounds {layout_ty}, ptr {dest_ptr}, i32 0, i32 {i}"
                    )
                    .ok();
                    writeln!(self.body, "  store {} {v}, ptr {gep}", sc.llvm_ty()).ok();
                }
                Ok(())
            }
            Expr::Array { elements, .. } => {
                if elements.len() != field_meta.len() {
                    return Err(diag(
                        "native layout: array init length must match tuple layout",
                    ));
                }
                for (i, (_, sc)) in field_meta.iter().enumerate() {
                    let ArrayElement::Expr(val_expr) = &elements[i] else {
                        return Err(diag("native layout: spread not supported in array init"));
                    };
                    let v = self.emit_expr(val_expr, Some(*sc))?;
                    let gep = self.fresh_tmp();
                    writeln!(
                        self.body,
                        "  {gep} = getelementptr inbounds {layout_ty}, ptr {dest_ptr}, i32 0, i32 {i}"
                    )
                    .ok();
                    writeln!(self.body, "  store {} {v}, ptr {gep}", sc.llvm_ty()).ok();
                }
                Ok(())
            }
            Expr::Local { id, .. } => {
                let src_ptr = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("internal: layout copy missing src alloca"))?;
                for (i, (_, sc)) in field_meta.iter().enumerate() {
                    let src_gep = self.fresh_tmp();
                    writeln!(
                        self.body,
                        "  {src_gep} = getelementptr inbounds {layout_ty}, ptr {src_ptr}, i32 0, i32 {i}"
                    )
                    .ok();
                    let v = self.fresh_tmp();
                    writeln!(self.body, "  {v} = load {}, ptr {src_gep}", sc.llvm_ty()).ok();
                    let dst_gep = self.fresh_tmp();
                    writeln!(
                        self.body,
                        "  {dst_gep} = getelementptr inbounds {layout_ty}, ptr {dest_ptr}, i32 0, i32 {i}"
                    )
                    .ok();
                    writeln!(self.body, "  store {} {v}, ptr {dst_gep}", sc.llvm_ty()).ok();
                }
                Ok(())
            }
            Expr::Call {
                callee,
                args,
                optional,
                ty,
            } => {
                if *optional {
                    return Err(diag("native layout: optional call not supported"));
                }
                let Expr::Local { id, .. } = callee.as_ref() else {
                    return Err(diag("native layout: only direct extern calls supported"));
                };
                let ext = self
                    .extern_fns
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("native layout: init call must be extern \"C\""))?;
                let v = self.emit_extern_call(&ext, args, *ty, None)?;
                let abi = layout_abi_llvm(shape);
                writeln!(self.body, "  store {abi} {v}, ptr {dest_ptr}").ok();
                Ok(())
            }
            _ => Err(diag(
                "native layout: init must be object/array literal, layout local, or extern call",
            )),
        }
    }
}
