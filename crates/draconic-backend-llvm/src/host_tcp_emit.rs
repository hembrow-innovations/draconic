use std::fmt::Write as _;

use super::*;

impl<'a> super::Emitter<'a> {
    pub(super) fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        let mut local_name = HashMap::new();
        for Local { id, name, .. } in &module.locals {
            local_name.insert(*id, name.clone());
        }
        let mut slot_of = HashMap::new();
        for (id, ty) in &info.slots {
            slot_of.insert(*id, *ty);
        }
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            next_tmp: 0,
            next_label: 0,
            str_globals: Vec::new(),
            local_name,
            slot_of,
        }
    }

    pub(super) fn finish(self) -> String {
        self.out
    }

    pub(super) fn fresh(&mut self) -> String {
        let n = self.next_tmp;
        self.next_tmp += 1;
        format!("%t{n}")
    }

    pub(super) fn fresh_label(&mut self, prefix: &str) -> String {
        let n = self.next_label;
        self.next_label += 1;
        format!("{prefix}{n}")
    }

    pub(super) fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_tcp: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    pub(super) fn slot_len_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_tcp: unknown local"))?;
        Ok(format!("%slot_{name}_len"))
    }

    pub(super) fn intern_cstr(&mut self, s: &str) -> String {
        if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            return g.clone();
        }
        let g = format!(".str.tcp.{}", self.str_globals.len());
        self.str_globals.push((s.to_string(), g.clone()));
        g
    }

    pub(super) fn emit_cstr_ptr(&mut self, s: &str) -> String {
        let g = self.intern_cstr(s);
        let n = s.len() + 1;
        let p = self.fresh();
        writeln!(
            self.body,
            "  {p} = getelementptr inbounds [{n} x i8], ptr @{g}, i64 0, i64 0"
        )
        .ok();
        p
    }

    pub(super) fn emit_host_err_exit(&mut self, code: &str) -> Result<(), Diagnostic> {
        let msg = format!("{code}\n");
        let p = self.emit_cstr_ptr(&msg);
        let n = msg.len();
        writeln!(
            self.body,
            "  {}",
            HOST_STDERR_WRITE.call(&format!("ptr {p}, i64 {n}"))
        )
        .ok();
        writeln!(self.body, "  {}", HOST_PROCESS_EXIT.call("i32 1")).ok();
        writeln!(self.body, "  unreachable").ok();
        Ok(())
    }

    pub(super) fn emit_check_rc(&mut self, rc: &str) -> Result<(), Diagnostic> {
        let ok = self.fresh_label("tcp_ok");
        let bad = self.fresh_label("tcp_err");
        let conn_l = self.fresh_label("tcp_econn");
        let not_conn = self.fresh_label("tcp_not_econn");
        let addr_l = self.fresh_label("tcp_eaddr");
        let not_addr = self.fresh_label("tcp_not_eaddr");
        let perm_l = self.fresh_label("tcp_eperm");
        let other_l = self.fresh_label("tcp_eio");
        let cmp = self.fresh();
        writeln!(self.body, "  {cmp} = icmp eq i32 {rc}, 0").ok();
        writeln!(self.body, "  br i1 {cmp}, label %{ok}, label %{bad}").ok();
        writeln!(self.body, "{bad}:").ok();
        let is_conn = self.fresh();
        writeln!(self.body, "  {is_conn} = icmp eq i32 {rc}, 10").ok();
        writeln!(
            self.body,
            "  br i1 {is_conn}, label %{conn_l}, label %{not_conn}"
        )
        .ok();
        writeln!(self.body, "{conn_l}:").ok();
        self.emit_host_err_exit("ECONN")?;
        writeln!(self.body, "{not_conn}:").ok();
        let is_addr = self.fresh();
        writeln!(self.body, "  {is_addr} = icmp eq i32 {rc}, 11").ok();
        writeln!(
            self.body,
            "  br i1 {is_addr}, label %{addr_l}, label %{not_addr}"
        )
        .ok();
        writeln!(self.body, "{addr_l}:").ok();
        self.emit_host_err_exit("EADDR")?;
        writeln!(self.body, "{not_addr}:").ok();
        let is_perm = self.fresh();
        writeln!(self.body, "  {is_perm} = icmp eq i32 {rc}, 6").ok();
        writeln!(
            self.body,
            "  br i1 {is_perm}, label %{perm_l}, label %{other_l}"
        )
        .ok();
        writeln!(self.body, "{perm_l}:").ok();
        self.emit_host_err_exit("EPERM")?;
        writeln!(self.body, "{other_l}:").ok();
        self.emit_host_err_exit("EIO")?;
        writeln!(self.body, "{ok}:").ok();
        Ok(())
    }

    pub(super) fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM host_tcp (H06.01–H06.04 listen/accept/connect/io)"
        )
        .ok();
        self.out.push_str(&llvm_declares(&[
            GC_INIT,
            PRINT_STR,
            PRINT_F64,
            PRINT_BOOL,
            HOST_TCP_LISTEN,
            HOST_TCP_LOCAL_PORT,
            HOST_TCP_ACCEPT,
            HOST_TCP_CONNECT,
            HOST_TCP_PEER_PORT,
            HOST_TCP_PEER_ADDRESS,
            HOST_TCP_READ,
            HOST_TCP_WRITE,
            HOST_TCP_SHUTDOWN,
            HOST_TLS_CLIENT_WRAP,
            HOST_TLS_SERVER_WRAP,
            HOST_TLS_READ,
            HOST_TLS_WRITE,
            HOST_HANDLE_CLOSE,
            HOST_STDOUT_WRITE,
            HOST_STDERR_WRITE,
            HOST_PROCESS_EXIT,
        ]));
        writeln!(self.out).ok();

        for (id, ty) in &self.info.slots {
            let ptr = self.slot_ptr(*id)?;
            match ty {
                SlotTy::Handle | SlotTy::Number => {
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                }
                SlotTy::Bool => {
                    writeln!(self.body, "  {ptr} = alloca i8, align 1").ok();
                }
                SlotTy::String => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                }
                SlotTy::DynBytes => {
                    let lp = self.slot_len_ptr(*id)?;
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  {lp} = alloca i64, align 8").ok();
                    writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
                    writeln!(self.body, "  store i64 0, ptr {lp}").ok();
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for (id, kind) in &self.info.print_locals {
            let ptr = self.slot_ptr(*id)?;
            match kind {
                SlotTy::Bool => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {v}"))).ok();
                }
                SlotTy::String => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
                SlotTy::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                }
                SlotTy::Handle | SlotTy::DynBytes => {}
            }
        }

        let body = std::mem::take(&mut self.body);
        for (content, gname) in &self.str_globals {
            let n = content.len() + 1;
            let esc = escape_llvm_string(content);
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
        self.out.push_str(&body);
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    pub(super) fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let init = init
                    .as_ref()
                    .ok_or_else(|| diag("host_tcp: declare needs init"))?;
                let ptr = self.slot_ptr(*local)?;
                let ty = self
                    .slot_of
                    .get(local)
                    .copied()
                    .ok_or_else(|| diag("host_tcp: unknown slot"))?;
                match ty {
                    SlotTy::Handle => {
                        let v = self.emit_handle_expr(init)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Bool => {
                        let v = self.emit_bool_expr(init)?;
                        writeln!(self.body, "  store i8 {v}, ptr {ptr}").ok();
                    }
                    SlotTy::String => {
                        let v = self.emit_string_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::DynBytes => self.emit_dynbytes_into(*local, init)?,
                }
                Ok(())
            }
            Stmt::Expr { expr, .. } => self.emit_expr_stmt(expr),
            _ => Err(diag("host_tcp: unsupported stmt")),
        }
    }

    pub(super) fn emit_dynbytes_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 2
                    && (is_named_callee(callee, "tcpRead")
                        || is_named_callee(callee, "tlsRead")) =>
            {
                let is_tls = is_named_callee(callee, "tlsRead");
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_tcp: read handle"))?,
                )?;
                let max_f = self.emit_number_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_tcp: read maxLen"))?,
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
                let sym = if is_tls {
                    HOST_TLS_READ.symbol
                } else {
                    HOST_TCP_READ.symbol
                };
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{sym}(i64 {h}, i64 {max_i}, ptr {out_data}, ptr {out_len})"
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
            _ => Err(diag("host_tcp: expected tcpRead/tlsRead for DynBytes")),
        }
    }

    pub(super) fn emit_expr_stmt(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1
                    && (is_named_callee(callee, "closeTcp")
                        || is_named_callee(callee, "closeTls")) =>
            {
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_tcp: close handle"))?,
                )?;
                let rc = self.fresh();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i64 {h})",
                    HOST_HANDLE_CLOSE.symbol
                )
                .ok();
                self.emit_check_rc(&rc)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2
                    && (is_named_callee(callee, "tcpWrite")
                        || is_named_callee(callee, "tlsWrite")) =>
            {
                let is_tls = is_named_callee(callee, "tlsWrite");
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_tcp: write handle"))?,
                )?;
                let (d, n) = self.emit_bytes_ptr_len(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_tcp: write data"))?,
                )?;
                let rc = self.fresh();
                let sym = if is_tls {
                    HOST_TLS_WRITE.symbol
                } else {
                    HOST_TCP_WRITE.symbol
                };
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{sym}(i64 {h}, ptr {d}, i64 {n})"
                )
                .ok();
                self.emit_check_rc(&rc)
            }
            Expr::Call { callee, args, .. }
                if (args.len() == 1 || args.len() == 2)
                    && is_named_callee(callee, "tcpShutdown") =>
            {
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_tcp: tcpShutdown handle"))?,
                )?;
                let how_i = if args.len() == 2 {
                    let hf = self.emit_number_expr(
                        arg_expr(&args[1]).ok_or_else(|| diag("host_tcp: tcpShutdown how"))?,
                    )?;
                    let hi = self.fresh();
                    writeln!(self.body, "  {hi} = fptosi double {hf} to i32").ok();
                    hi
                } else {
                    "1".to_string()
                };
                let rc = self.fresh();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i64 {h}, i32 {how_i})",
                    HOST_TCP_SHUTDOWN.symbol
                )
                .ok();
                self.emit_check_rc(&rc)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "stdoutWrite") =>
            {
                self.emit_stdout_write(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_tcp: stdoutWrite arg"))?,
                )
            }
            _ => Err(diag("host_tcp: unsupported expr stmt")),
        }
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
                _ => Err(diag("host_tcp: bytes arg unsupported")),
            },
            _ => Err(diag("host_tcp: bytes arg unsupported")),
        }
    }

    pub(super) fn emit_cstr_len(&mut self, s: &str) -> Result<String, Diagnostic> {
        let i = self.fresh();
        let loop_l = format!("wlen_loop_{}", self.next_tmp);
        let done_l = format!("wlen_done_{}", self.next_tmp);
        let inc_l = format!("wlen_inc_{}", self.next_tmp);
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
        let ch = self.fresh();
        writeln!(self.body, "  {ch} = load i8, ptr {cp}").ok();
        let is0 = self.fresh();
        writeln!(self.body, "  {is0} = icmp eq i8 {ch}, 0").ok();
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
                    writeln!(self.body, "  {s} = load ptr, ptr {sp}").ok();
                    let n = self.emit_cstr_len(&s)?;
                    writeln!(
                        self.body,
                        "  {}",
                        HOST_STDOUT_WRITE.call(&format!("ptr {s}, i64 {n}"))
                    )
                    .ok();
                    Ok(())
                }
                _ => Err(diag("host_tcp: stdoutWrite unsupported arg")),
            },
            _ => Err(diag("host_tcp: stdoutWrite unsupported arg")),
        }
    }

    pub(super) fn emit_handle_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if (args.len() == 1 || args.len() == 2) && is_named_callee(callee, "tcpListen") =>
            {
                let port_f = self.emit_number_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_tcp: tcpListen port"))?,
                )?;
                let port_i = self.fresh();
                writeln!(self.body, "  {port_i} = fptosi double {port_f} to i32").ok();
                let backlog_i = if args.len() == 2 {
                    let bf = self.emit_number_expr(
                        arg_expr(&args[1]).ok_or_else(|| diag("host_tcp: tcpListen backlog"))?,
                    )?;
                    let bi = self.fresh();
                    writeln!(self.body, "  {bi} = fptosi double {bf} to i32").ok();
                    bi
                } else {
                    "0".to_string()
                };
                let out_h = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_h} = alloca i64, align 8").ok();
                writeln!(self.body, "  store i64 -1, ptr {out_h}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i32 {port_i}, i32 {backlog_i}, ptr {out_h})",
                    HOST_TCP_LISTEN.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let iv = self.fresh();
                let fv = self.fresh();
                writeln!(self.body, "  {iv} = load i64, ptr {out_h}").ok();
                writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                Ok(fv)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "tcpAccept") =>
            {
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_tcp: tcpAccept listen"))?,
                )?;
                let out_h = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_h} = alloca i64, align 8").ok();
                writeln!(self.body, "  store i64 -1, ptr {out_h}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i64 {h}, ptr {out_h})",
                    HOST_TCP_ACCEPT.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let iv = self.fresh();
                let fv = self.fresh();
                writeln!(self.body, "  {iv} = load i64, ptr {out_h}").ok();
                writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                Ok(fv)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "tcpConnect") =>
            {
                let host = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_tcp: tcpConnect host"))?,
                )?;
                let port_f = self.emit_number_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_tcp: tcpConnect port"))?,
                )?;
                let port_i = self.fresh();
                writeln!(self.body, "  {port_i} = fptosi double {port_f} to i32").ok();
                let out_h = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_h} = alloca i64, align 8").ok();
                writeln!(self.body, "  store i64 -1, ptr {out_h}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {host}, i32 {port_i}, ptr {out_h})",
                    HOST_TCP_CONNECT.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let iv = self.fresh();
                let fv = self.fresh();
                writeln!(self.body, "  {iv} = load i64, ptr {out_h}").ok();
                writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                Ok(fv)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 3 && is_named_callee(callee, "tlsClientWrap") =>
            {
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_tcp: tlsClientWrap conn"))?,
                )?;
                let name = self.emit_string_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_tcp: tlsClientWrap serverName"))?,
                )?;
                let insecure_f = self.emit_number_expr(
                    arg_expr(&args[2]).ok_or_else(|| diag("host_tcp: tlsClientWrap insecure"))?,
                )?;
                let insecure_i = self.fresh();
                writeln!(
                    self.body,
                    "  {insecure_i} = fptosi double {insecure_f} to i32"
                )
                .ok();
                let out_h = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_h} = alloca i64, align 8").ok();
                writeln!(self.body, "  store i64 -1, ptr {out_h}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i64 {h}, ptr {name}, i32 {insecure_i}, ptr {out_h})",
                    HOST_TLS_CLIENT_WRAP.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let iv = self.fresh();
                let fv = self.fresh();
                writeln!(self.body, "  {iv} = load i64, ptr {out_h}").ok();
                writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                Ok(fv)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 3 && is_named_callee(callee, "tlsServerWrap") =>
            {
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_tcp: tlsServerWrap conn"))?,
                )?;
                let cert = self.emit_string_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_tcp: tlsServerWrap certPath"))?,
                )?;
                let key = self.emit_string_expr(
                    arg_expr(&args[2]).ok_or_else(|| diag("host_tcp: tlsServerWrap keyPath"))?,
                )?;
                let out_h = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_h} = alloca i64, align 8").ok();
                writeln!(self.body, "  store i64 -1, ptr {out_h}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i64 {h}, ptr {cert}, ptr {key}, ptr {out_h})",
                    HOST_TLS_SERVER_WRAP.symbol
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
            _ => Err(diag("host_tcp: expected handle expr")),
        }
    }
}
