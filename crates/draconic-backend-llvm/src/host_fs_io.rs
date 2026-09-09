use std::fmt::Write as _;

use super::*;

impl<'a> super::Emitter<'a> {
    pub(super) fn emit_path_void_call(&mut self, path: &Expr, symbol: &str) -> Result<(), Diagnostic> {
        let p = self.emit_string_expr(path)?;
        let rc = self.fresh();
        writeln!(self.body, "  {rc} = call i32 @{symbol}(ptr {p})").ok();
        self.emit_check_rc(&rc)
    }

    pub(super) fn emit_two_path_void_call(
        &mut self,
        from: &Expr,
        to: &Expr,
        symbol: &str,
    ) -> Result<(), Diagnostic> {
        let a = self.emit_string_expr(from)?;
        let b = self.emit_string_expr(to)?;
        let rc = self.fresh();
        writeln!(self.body, "  {rc} = call i32 @{symbol}(ptr {a}, ptr {b})").ok();
        self.emit_check_rc(&rc)
    }

    pub(super) fn emit_array_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "readdir") =>
            {
                self.emit_readdir(arg_expr(&args[0]).ok_or_else(|| diag("host_fs: readdir path"))?)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_fs: expected readdir array")),
        }
    }

    pub(super) fn emit_readdir(&mut self, path: &Expr) -> Result<String, Diagnostic> {
        let p = self.emit_string_expr(path)?;
        let out_names = self.fresh();
        let out_count = self.fresh();
        let rc = self.fresh();
        writeln!(self.body, "  {out_names} = alloca ptr, align 8").ok();
        writeln!(self.body, "  {out_count} = alloca i64, align 8").ok();
        writeln!(self.body, "  store ptr null, ptr {out_names}").ok();
        writeln!(self.body, "  store i64 0, ptr {out_count}").ok();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(ptr {p}, ptr {out_names}, ptr {out_count})",
            HOST_FS_READDIR.symbol
        )
        .ok();
        self.emit_check_rc(&rc)?;
        let names = self.fresh();
        let n = self.fresh();
        writeln!(self.body, "  {names} = load ptr, ptr {out_names}").ok();
        writeln!(self.body, "  {n} = load i64, ptr {out_count}").ok();
        let arr = self.fresh();
        writeln!(
            self.body,
            "  {}",
            ARRAY_NEW.call_to(&arr, &format!("i64 {n}"))
        )
        .ok();
        let i_slot = self.fresh();
        let loop_cond = format!("rd_loop_cond_{}", self.next_tmp);
        let loop_body = format!("rd_loop_body_{}", self.next_tmp);
        let loop_end = format!("rd_loop_end_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  {i_slot} = alloca i64, align 8").ok();
        writeln!(self.body, "  store i64 0, ptr {i_slot}").ok();
        writeln!(self.body, "  br label %{loop_cond}").ok();
        writeln!(self.body, "{loop_cond}:").ok();
        let i_load = self.fresh();
        let cmp = self.fresh();
        writeln!(self.body, "  {i_load} = load i64, ptr {i_slot}").ok();
        writeln!(self.body, "  {cmp} = icmp slt i64 {i_load}, {n}").ok();
        writeln!(
            self.body,
            "  br i1 {cmp}, label %{loop_body}, label %{loop_end}"
        )
        .ok();
        writeln!(self.body, "{loop_body}:").ok();
        let name_pp = self.fresh();
        let name_p = self.fresh();
        let i_next = self.fresh();
        writeln!(
            self.body,
            "  {name_pp} = getelementptr inbounds ptr, ptr {names}, i64 {i_load}"
        )
        .ok();
        writeln!(self.body, "  {name_p} = load ptr, ptr {name_pp}").ok();
        writeln!(
            self.body,
            "  call void @{}(ptr {arr}, i64 {i_load}, ptr {name_p})",
            ARRAY_SET.symbol
        )
        .ok();
        writeln!(self.body, "  {i_next} = add i64 {i_load}, 1").ok();
        writeln!(self.body, "  store i64 {i_next}, ptr {i_slot}").ok();
        writeln!(self.body, "  br label %{loop_cond}").ok();
        writeln!(self.body, "{loop_end}:").ok();
        Ok(arr)
    }

    pub(super) fn emit_write_text_call(
        &mut self,
        path: &Expr,
        text: &Expr,
        symbol: &str,
    ) -> Result<(), Diagnostic> {
        let p = self.emit_string_expr(path)?;
        let t = self.emit_string_expr(text)?;
        let rc = self.fresh();
        writeln!(self.body, "  {rc} = call i32 @{symbol}(ptr {p}, ptr {t})").ok();
        self.emit_check_rc(&rc)
    }

    pub(super) fn emit_write_bytes_call(
        &mut self,
        path: &Expr,
        data: &Expr,
        symbol: &str,
    ) -> Result<(), Diagnostic> {
        let p = self.emit_string_expr(path)?;
        let (d, n) = self.emit_bytes_ptr_len(data)?;
        let rc = self.fresh();
        writeln!(
            self.body,
            "  {rc} = call i32 @{symbol}(ptr {p}, ptr {d}, i64 {n})"
        )
        .ok();
        self.emit_check_rc(&rc)
    }

    pub(super) fn emit_bytes_ptr_len(&mut self, expr: &Expr) -> Result<(String, String), Diagnostic> {
        match expr {
            Expr::String { value, .. } => {
                let s = value.to_string_lossy();
                let p = self.emit_cstr_ptr(&s);
                Ok((p, s.len().to_string()))
            }
            Expr::Local { id, .. } => match self.slot_of.get(id) {
                Some(SlotTy::DynBytes) => {
                    let dp = self.slot_ptr(*id)?;
                    let lp = self.slot_len_ptr(*id)?;
                    let d = self.fresh();
                    let n = self.fresh();
                    writeln!(self.body, "  {d} = load ptr, ptr {dp}").ok();
                    writeln!(self.body, "  {n} = load i64, ptr {lp}").ok();
                    Ok((d, n))
                }
                Some(SlotTy::String) => {
                    let sp = self.slot_ptr(*id)?;
                    let s = self.fresh();
                    writeln!(self.body, "  {s} = load ptr, ptr {sp}").ok();
                    let n = self.emit_cstr_len(&s)?;
                    Ok((s, n))
                }
                _ => Err(diag("host_fs: bytes arg unsupported")),
            },
            _ => Err(diag("host_fs: bytes arg unsupported")),
        }
    }

    pub(super) fn emit_cstr_len(&mut self, s: &str) -> Result<String, Diagnostic> {
        let i = self.fresh();
        let ch = self.fresh();
        let is0 = self.fresh();
        let loop_l = format!("wlen_loop_{}", self.next_tmp);
        let done_l = format!("wlen_done_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  {i} = alloca i64, align 8").ok();
        writeln!(self.body, "  store i64 0, ptr {i}").ok();
        writeln!(self.body, "  br label %{loop_l}").ok();
        writeln!(self.body, "{loop_l}:").ok();
        let iv = self.fresh();
        writeln!(self.body, "  {iv} = load i64, ptr {i}").ok();
        let cp = self.fresh();
        writeln!(
            self.body,
            "  {cp} = getelementptr inbounds i8, ptr {s}, i64 {iv}"
        )
        .ok();
        writeln!(self.body, "  {ch} = load i8, ptr {cp}").ok();
        writeln!(self.body, "  {is0} = icmp eq i8 {ch}, 0").ok();
        let inc_l = format!("wlen_inc_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  br i1 {is0}, label %{done_l}, label %{inc_l}").ok();
        writeln!(self.body, "{inc_l}:").ok();
        let iv2 = self.fresh();
        let iv3 = self.fresh();
        writeln!(self.body, "  {iv2} = load i64, ptr {i}").ok();
        writeln!(self.body, "  {iv3} = add i64 {iv2}, 1").ok();
        writeln!(self.body, "  store i64 {iv3}, ptr {i}").ok();
        writeln!(self.body, "  br label %{loop_l}").ok();
        writeln!(self.body, "{done_l}:").ok();
        let n = self.fresh();
        writeln!(self.body, "  {n} = load i64, ptr {i}").ok();
        Ok(n)
    }

    pub(super) fn emit_stdout_write(&mut self, arg: &Expr) -> Result<(), Diagnostic> {
        match arg {
            Expr::String { value, .. } => {
                let s = value.to_string_lossy();
                let p = self.emit_cstr_ptr(&s);
                let n = s.len();
                writeln!(
                    self.body,
                    "  {}",
                    HOST_STDOUT_WRITE.call(&format!("ptr {p}, i64 {n}"))
                )
                .ok();
                Ok(())
            }
            Expr::Local { id, .. } => match self.slot_of.get(id) {
                Some(SlotTy::DynBytes) => {
                    let dp = self.slot_ptr(*id)?;
                    let lp = self.slot_len_ptr(*id)?;
                    let d = self.fresh();
                    let n = self.fresh();
                    writeln!(self.body, "  {d} = load ptr, ptr {dp}").ok();
                    writeln!(self.body, "  {n} = load i64, ptr {lp}").ok();
                    writeln!(
                        self.body,
                        "  {}",
                        HOST_STDOUT_WRITE.call(&format!("ptr {d}, i64 {n}"))
                    )
                    .ok();
                    Ok(())
                }
                Some(SlotTy::String) => {
                    let sp = self.slot_ptr(*id)?;
                    let s = self.fresh();
                    let n = self.fresh();
                    writeln!(self.body, "  {s} = load ptr, ptr {sp}").ok();
                    // strlen via loop-free: print_str already adds newline; for write use host
                    // Approximate: walk C string length
                    let i = self.fresh();
                    let ch = self.fresh();
                    let is0 = self.fresh();
                    let loop_l = format!("slen_loop_{}", self.next_tmp);
                    let done_l = format!("slen_done_{}", self.next_tmp);
                    self.next_tmp += 1;
                    writeln!(self.body, "  {i} = alloca i64, align 8").ok();
                    writeln!(self.body, "  store i64 0, ptr {i}").ok();
                    writeln!(self.body, "  br label %{loop_l}").ok();
                    writeln!(self.body, "{loop_l}:").ok();
                    let iv = self.fresh();
                    writeln!(self.body, "  {iv} = load i64, ptr {i}").ok();
                    let cp = self.fresh();
                    writeln!(
                        self.body,
                        "  {cp} = getelementptr inbounds i8, ptr {s}, i64 {iv}"
                    )
                    .ok();
                    writeln!(self.body, "  {ch} = load i8, ptr {cp}").ok();
                    writeln!(self.body, "  {is0} = icmp eq i8 {ch}, 0").ok();
                    writeln!(
                        self.body,
                        "  br i1 {is0}, label %{done_l}, label %slen_inc_{}",
                        self.next_tmp
                    )
                    .ok();
                    let inc_l = format!("slen_inc_{}", self.next_tmp);
                    self.next_tmp += 1;
                    writeln!(self.body, "{inc_l}:").ok();
                    let iv2 = self.fresh();
                    let iv3 = self.fresh();
                    writeln!(self.body, "  {iv2} = load i64, ptr {i}").ok();
                    writeln!(self.body, "  {iv3} = add i64 {iv2}, 1").ok();
                    writeln!(self.body, "  store i64 {iv3}, ptr {i}").ok();
                    writeln!(self.body, "  br label %{loop_l}").ok();
                    writeln!(self.body, "{done_l}:").ok();
                    writeln!(self.body, "  {n} = load i64, ptr {i}").ok();
                    writeln!(
                        self.body,
                        "  {}",
                        HOST_STDOUT_WRITE.call(&format!("ptr {s}, i64 {n}"))
                    )
                    .ok();
                    Ok(())
                }
                _ => Err(diag("host_fs: stdoutWrite unsupported arg")),
            },
            _ => Err(diag("host_fs: stdoutWrite unsupported arg")),
        }
    }
}
