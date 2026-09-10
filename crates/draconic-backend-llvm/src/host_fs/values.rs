use std::fmt::Write as _;

use super::*;

impl<'a> super::Emitter<'a> {
    pub(super) fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => Ok(self.emit_cstr_ptr(&value.to_string_lossy())),
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "readFileText") => {
                if args.len() != 1 {
                    return Err(diag("host_fs: readFileText expects 1 arg"));
                }
                let path = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: readFileText path"))?,
                )?;
                let out = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out} = alloca ptr, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {out}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {path}, ptr {out})",
                    HOST_FS_READ_TEXT.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {out}").ok();
                Ok(v)
            }
            Expr::Member {
                object,
                property,
                computed: true,
                ..
            } => {
                let arr = self.emit_array_expr(object)?;
                let idx_f = self.emit_number_expr(property)?;
                let idx = self.fresh();
                let el = self.fresh();
                writeln!(self.body, "  {idx} = fptosi double {idx_f} to i64").ok();
                writeln!(
                    self.body,
                    "  {el} = call ptr @{}(ptr {arr}, i64 {idx})",
                    ARRAY_GET.symbol
                )
                .ok();
                Ok(el)
            }
            _ => Err(diag("host_fs: unsupported string expr")),
        }
    }

    pub(super) fn emit_read_bytes_into(
        &mut self,
        local: LocalId,
        expr: &Expr,
    ) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. } if is_named_callee(callee, "readFileBytes") => {
                if args.len() != 1 {
                    return Err(diag("host_fs: readFileBytes expects 1 arg"));
                }
                let path = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: readFileBytes path"))?,
                )?;
                let data_slot = self.slot_ptr(local)?;
                let len_slot = self.slot_len_ptr(local)?;
                let out_data = self.fresh();
                let out_len = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_data} = alloca ptr, align 8").ok();
                writeln!(self.body, "  {out_len} = alloca i64, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {out_data}").ok();
                writeln!(self.body, "  store i64 0, ptr {out_len}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {path}, ptr {out_data}, ptr {out_len})",
                    HOST_FS_READ_FILE.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let d = self.fresh();
                let n = self.fresh();
                writeln!(self.body, "  {d} = load ptr, ptr {out_data}").ok();
                writeln!(self.body, "  {n} = load i64, ptr {out_len}").ok();
                writeln!(self.body, "  store ptr {d}, ptr {data_slot}").ok();
                writeln!(self.body, "  store i64 {n}, ptr {len_slot}").ok();
                Ok(())
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "fileRead") => {
                if args.len() != 2 {
                    return Err(diag("host_fs: fileRead expects 2 args"));
                }
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: fileRead handle"))?,
                )?;
                let max_f = self.emit_number_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_fs: fileRead maxLen"))?,
                )?;
                let max_i = self.fresh();
                writeln!(self.body, "  {max_i} = fptosi double {max_f} to i64").ok();
                let data_slot = self.slot_ptr(local)?;
                let len_slot = self.slot_len_ptr(local)?;
                let out_data = self.fresh();
                let out_len = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_data} = alloca ptr, align 8").ok();
                writeln!(self.body, "  {out_len} = alloca i64, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {out_data}").ok();
                writeln!(self.body, "  store i64 0, ptr {out_len}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i64 {h}, i64 {max_i}, ptr {out_data}, ptr {out_len})",
                    HOST_FS_HANDLE_READ.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let d = self.fresh();
                let n = self.fresh();
                writeln!(self.body, "  {d} = load ptr, ptr {out_data}").ok();
                writeln!(self.body, "  {n} = load i64, ptr {out_len}").ok();
                writeln!(self.body, "  store ptr {d}, ptr {data_slot}").ok();
                writeln!(self.body, "  store i64 {n}, ptr {len_slot}").ok();
                Ok(())
            }
            _ => Err(diag("host_fs: expected readFileBytes or fileRead")),
        }
    }

    pub(super) fn emit_handle_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. } if is_named_callee(callee, "openFile") => {
                if args.len() != 2 {
                    return Err(diag("host_fs: openFile expects 2 args"));
                }
                let path = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: openFile path"))?,
                )?;
                let mode = self.emit_string_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_fs: openFile mode"))?,
                )?;
                let out_h = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_h} = alloca i64, align 8").ok();
                writeln!(self.body, "  store i64 -1, ptr {out_h}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {path}, ptr {mode}, ptr {out_h})",
                    HOST_FS_OPEN.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let iv = self.fresh();
                let fv = self.fresh();
                writeln!(self.body, "  {iv} = load i64, ptr {out_h}").ok();
                writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                Ok(fv)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_fs: expected openFile handle")),
        }
    }

    pub(super) fn emit_handle_i64(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        let f = self.emit_handle_expr(expr)?;
        let i = self.fresh();
        writeln!(self.body, "  {i} = fptosi double {f} to i64").ok();
        Ok(i)
    }

    pub(super) fn emit_file_write(&mut self, handle: &Expr, data: &Expr) -> Result<(), Diagnostic> {
        let h = self.emit_handle_i64(handle)?;
        let (d, n) = self.emit_bytes_ptr_len(data)?;
        let rc = self.fresh();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(i64 {h}, ptr {d}, i64 {n})",
            HOST_FS_HANDLE_WRITE.symbol
        )
        .ok();
        self.emit_check_rc(&rc)
    }

    pub(super) fn emit_file_seek(
        &mut self,
        handle: &Expr,
        offset: &Expr,
        whence: Option<&Expr>,
    ) -> Result<String, Diagnostic> {
        let h = self.emit_handle_i64(handle)?;
        let off_f = self.emit_number_expr(offset)?;
        let off_i = self.fresh();
        writeln!(self.body, "  {off_i} = fptosi double {off_f} to i64").ok();
        let wh_i = if let Some(w) = whence {
            let wf = self.emit_number_expr(w)?;
            let wi = self.fresh();
            writeln!(self.body, "  {wi} = fptosi double {wf} to i32").ok();
            wi
        } else {
            "0".to_string()
        };
        let out_pos = self.fresh();
        let rc = self.fresh();
        writeln!(self.body, "  {out_pos} = alloca i64, align 8").ok();
        writeln!(self.body, "  store i64 0, ptr {out_pos}").ok();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(i64 {h}, i64 {off_i}, i32 {wh_i}, ptr {out_pos})",
            HOST_FS_HANDLE_SEEK.symbol
        )
        .ok();
        self.emit_check_rc(&rc)?;
        let iv = self.fresh();
        let fv = self.fresh();
        writeln!(self.body, "  {iv} = load i64, ptr {out_pos}").ok();
        writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
        Ok(fv)
    }

    pub(super) fn emit_close_file(&mut self, handle: &Expr) -> Result<(), Diagnostic> {
        let h = self.emit_handle_i64(handle)?;
        let rc = self.fresh();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(i64 {h})",
            HOST_HANDLE_CLOSE.symbol
        )
        .ok();
        self.emit_check_rc(&rc)
    }

    pub(super) fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                // LLVM double constants need a decimal form (`0` alone is rejected).
                if raw.contains('.') || raw.contains('e') || raw.contains('E') {
                    Ok(raw.clone())
                } else {
                    Ok(format!("{raw}.0"))
                }
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let prop = string_lit(property).ok_or_else(|| diag("host_fs: member prop"))?;
                let id = match object.as_ref() {
                    Expr::Local { id, .. } => *id,
                    _ => return Err(diag("host_fs: member object must be local")),
                };
                match (self.slot_of.get(&id), prop.as_str()) {
                    (Some(LocalSlot::DynBytes), "length") => {
                        let lp = self.slot_len_ptr(id)?;
                        let iv = self.fresh();
                        let fv = self.fresh();
                        writeln!(self.body, "  {iv} = load i64, ptr {lp}").ok();
                        writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                        Ok(fv)
                    }
                    (Some(LocalSlot::Array), "length") => {
                        let ap = self.slot_ptr(id)?;
                        let arr = self.fresh();
                        let iv = self.fresh();
                        let fv = self.fresh();
                        writeln!(self.body, "  {arr} = load ptr, ptr {ap}").ok();
                        writeln!(
                            self.body,
                            "  {iv} = call i64 @{}(ptr {arr})",
                            ARRAY_LEN.symbol
                        )
                        .ok();
                        writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                        Ok(fv)
                    }
                    (Some(LocalSlot::Stat), "size") => {
                        let sp = self.slot_stat_field(id, "size")?;
                        let iv = self.fresh();
                        let fv = self.fresh();
                        writeln!(self.body, "  {iv} = load i64, ptr {sp}").ok();
                        writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                        Ok(fv)
                    }
                    (Some(LocalSlot::Stat), "mtime") => {
                        let mp = self.slot_stat_field(id, "mtime")?;
                        let v = self.fresh();
                        writeln!(self.body, "  {v} = load double, ptr {mp}").ok();
                        Ok(v)
                    }
                    _ => Err(diag("host_fs: unsupported number member")),
                }
            }
            _ => Err(diag("host_fs: unsupported number expr")),
        }
    }

    pub(super) fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. } if is_named_callee(callee, "exists") => {
                if args.len() != 1 {
                    return Err(diag("host_fs: exists expects 1 arg"));
                }
                let path = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: exists path"))?,
                )?;
                let rc = self.fresh();
                let b = self.fresh();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {path})",
                    HOST_FS_EXISTS.symbol
                )
                .ok();
                // exists returns 0/1 i32 → i8
                writeln!(self.body, "  {b} = trunc i32 {rc} to i8").ok();
                Ok(b)
            }
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let prop = string_lit(property).ok_or_else(|| diag("host_fs: bool member"))?;
                let id = match object.as_ref() {
                    Expr::Local { id, .. } => *id,
                    _ => return Err(diag("host_fs: bool member object must be local")),
                };
                match (self.slot_of.get(&id), prop.as_str()) {
                    (Some(LocalSlot::Stat), "isFile") => {
                        let p = self.slot_stat_field(id, "is_file")?;
                        let iv = self.fresh();
                        let b = self.fresh();
                        writeln!(self.body, "  {iv} = load i32, ptr {p}").ok();
                        writeln!(self.body, "  {b} = trunc i32 {iv} to i8").ok();
                        Ok(b)
                    }
                    (Some(LocalSlot::Stat), "isDir") => {
                        let p = self.slot_stat_field(id, "is_dir")?;
                        let iv = self.fresh();
                        let b = self.fresh();
                        writeln!(self.body, "  {iv} = load i32, ptr {p}").ok();
                        writeln!(self.body, "  {b} = trunc i32 {iv} to i8").ok();
                        Ok(b)
                    }
                    _ => Err(diag("host_fs: unsupported bool member")),
                }
            }
            Expr::Binary {
                op: BinaryOp::Gt,
                left,
                right,
                ..
            } => {
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let cmp = self.fresh();
                let b = self.fresh();
                writeln!(self.body, "  {cmp} = fcmp ogt double {l}, {r}").ok();
                writeln!(self.body, "  {b} = zext i1 {cmp} to i8").ok();
                Ok(b)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_fs: unsupported bool expr")),
        }
    }

    pub(super) fn emit_stat_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. } if is_named_callee(callee, "stat") => {
                if args.len() != 1 {
                    return Err(diag("host_fs: stat expects 1 arg"));
                }
                let path = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: stat path"))?,
                )?;
                let size = self.slot_stat_field(local, "size")?;
                let is_file = self.slot_stat_field(local, "is_file")?;
                let is_dir = self.slot_stat_field(local, "is_dir")?;
                let mtime = self.slot_stat_field(local, "mtime")?;
                let rc = self.fresh();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {path}, ptr {size}, ptr {is_file}, ptr {is_dir}, ptr {mtime})",
                    HOST_FS_STAT.symbol
                )
                .ok();
                self.emit_check_rc(&rc)
            }
            _ => Err(diag("host_fs: expected stat")),
        }
    }
}
