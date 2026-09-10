use std::fmt::Write as _;

use super::*;
use crate::emitter::escape_llvm_string;

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
        let n = self.next_tmp;
        self.next_tmp += 1;
        format!("{prefix}_{n}")
    }

    pub(super) fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_http_server: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    pub(super) fn slot_len_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_http_server: unknown local"))?;
        Ok(format!("%slot_{name}_len"))
    }

    pub(super) fn slot_req_field(&self, id: LocalId, field: &str) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_http_server: unknown req local"))?;
        Ok(format!("%slot_{name}_{field}"))
    }

    pub(super) fn emit_cstr_ptr(&mut self, s: &str) -> String {
        let g = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".str.httpsrv.{}", self.str_globals.len());
            self.str_globals.push((s.to_string(), g.clone()));
            g
        };
        let p = self.fresh();
        let n = s.len() + 1;
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
        let ok = self.fresh_label("hs_ok");
        let bad = self.fresh_label("hs_err");
        let conn_l = self.fresh_label("hs_econn");
        let not_conn = self.fresh_label("hs_not_conn");
        let inval_l = self.fresh_label("hs_einval");
        let other_l = self.fresh_label("hs_eio");
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
        let is_inval = self.fresh();
        writeln!(self.body, "  {is_inval} = icmp eq i32 {rc}, 1").ok();
        writeln!(
            self.body,
            "  br i1 {is_inval}, label %{inval_l}, label %{other_l}"
        )
        .ok();
        writeln!(self.body, "{inval_l}:").ok();
        self.emit_host_err_exit("EINVAL")?;
        writeln!(self.body, "{other_l}:").ok();
        self.emit_host_err_exit("EIO")?;
        writeln!(self.body, "{ok}:").ok();
        Ok(())
    }

    pub(super) fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM host_http_server (H10.03–H10.05 TCP+HTTP; H11.03 TLS)"
        )
        .ok();
        self.out.push_str(&llvm_declares(&[
            GC_INIT,
            PRINT_STR,
            PRINT_I64,
            HOST_TCP_LISTEN,
            HOST_TCP_LOCAL_PORT,
            HOST_TCP_ACCEPT,
            HOST_TCP_CONNECT,
            HOST_TCP_READ,
            HOST_TCP_WRITE,
            HOST_TLS_CLIENT_WRAP,
            HOST_TLS_SERVER_WRAP,
            HOST_TLS_READ,
            HOST_TLS_WRITE,
            HOST_HANDLE_CLOSE,
            HOST_HTTP_PARSE_REQUEST,
            HOST_HTTP_WRITE_RESPONSE,
            HOST_HTTP_SERVE_STATIC,
            HOST_HTTP_WRITE_REQUEST,
            HOST_HTTP_PARSE_RESPONSE,
            HOST_HTTP_RESPONSE_HEADER,
            HOST_WS_HANDSHAKE_RESPONSE,
            HOST_STDOUT_WRITE,
            HOST_STDERR_WRITE,
            HOST_PROCESS_EXIT,
            HOST_FS_READ_TEXT,
            CSTR_CONCAT,
        ]));
        writeln!(self.out, "declare i64 @strlen(ptr)").ok();
        writeln!(self.out).ok();

        for (id, ty) in &self.info.slots {
            match ty {
                SlotTy::Handle | SlotTy::Number => {
                    let ptr = self.slot_ptr(*id)?;
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                }
                SlotTy::String => {
                    let ptr = self.slot_ptr(*id)?;
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                }
                SlotTy::DynBytes => {
                    let ptr = self.slot_ptr(*id)?;
                    let lp = self.slot_len_ptr(*id)?;
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  {lp} = alloca i64, align 8").ok();
                    writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
                    writeln!(self.body, "  store i64 0, ptr {lp}").ok();
                }
                SlotTy::HttpReq => {
                    for f in ["method", "path", "version", "body", "raw"] {
                        let p = self.slot_req_field(*id, f)?;
                        writeln!(self.body, "  {p} = alloca ptr, align 8").ok();
                    }
                    let plen = self.slot_req_field(*id, "raw_len")?;
                    writeln!(self.body, "  {plen} = alloca i64, align 8").ok();
                }
                SlotTy::HttpRes => {
                    for f in ["version", "reason", "body", "raw"] {
                        let p = self.slot_req_field(*id, f)?;
                        writeln!(self.body, "  {p} = alloca ptr, align 8").ok();
                    }
                    let pst = self.slot_req_field(*id, "status")?;
                    writeln!(self.body, "  {pst} = alloca i32, align 4").ok();
                    let plen = self.slot_req_field(*id, "raw_len")?;
                    writeln!(self.body, "  {plen} = alloca i64, align 8").ok();
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        if self.info.client_print {
            for (id, ty) in &self.info.print_locals {
                match ty {
                    SlotTy::String => {
                        let ptr = self.slot_ptr(*id)?;
                        let v = self.fresh();
                        writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                        writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                    }
                    SlotTy::Number => {
                        let ptr = self.slot_ptr(*id)?;
                        let v = self.fresh();
                        let i = self.fresh();
                        writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                        writeln!(self.body, "  {i} = fptosi double {v} to i64").ok();
                        writeln!(self.body, "  {}", PRINT_I64.call(&format!("i64 {i}"))).ok();
                    }
                    _ => {}
                }
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
                    .ok_or_else(|| diag("host_http_server: declare needs init"))?;
                let ty = self
                    .slot_of
                    .get(local)
                    .copied()
                    .ok_or_else(|| diag("host_http_server: unknown slot"))?;
                match ty {
                    SlotTy::Handle => {
                        let v = self.emit_handle_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
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
                    SlotTy::DynBytes => self.emit_dynbytes_into(*local, init)?,
                    SlotTy::HttpReq => self.emit_http_req_into(*local, init)?,
                    SlotTy::HttpRes => self.emit_http_res_into(*local, init)?,
                }
                Ok(())
            }
            Stmt::Expr { expr, .. } => self.emit_expr_stmt(expr),
            Stmt::Block { body, .. } => {
                for s in body {
                    self.emit_stmt(s)?;
                }
                Ok(())
            }
            // H17.01: infinite accept loop (`while (true)` / `while (1)`).
            Stmt::While { test, body, .. } => {
                match test {
                    Expr::Boolean { value: true, .. } => {}
                    Expr::Number { raw, .. } if raw == "1" => {}
                    _ => {
                        return Err(diag(
                            "host_http_server: while test must be true or 1 (accept loop)",
                        ))
                    }
                }
                let head = self.fresh_label("hs_while_head");
                let bod = self.fresh_label("hs_while_body");
                let end = self.fresh_label("hs_while_end");
                writeln!(self.body, "  br label %{head}").ok();
                writeln!(self.body, "{head}:").ok();
                writeln!(self.body, "  br i1 true, label %{bod}, label %{end}").ok();
                writeln!(self.body, "{bod}:").ok();
                self.emit_stmt(body)?;
                writeln!(self.body, "  br label %{head}").ok();
                writeln!(self.body, "{end}:").ok();
                Ok(())
            }
            Stmt::Function { .. } => Ok(()),
            _ => Err(diag("host_http_server: unsupported stmt")),
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
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: read handle"))?,
                )?;
                let max_f = self.emit_number_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_http_server: read maxLen"))?,
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
            _ => Err(diag(
                "host_http_server: expected tcpRead/tlsRead for DynBytes",
            )),
        }
    }

    pub(super) fn emit_http_req_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "httpParseRequest") =>
            {
                let raw_e =
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: parse raw"))?;
                let (raw, raw_len) = self.emit_bytes_ptr_len(raw_e)?;
                let om = self.slot_req_field(local, "method")?;
                let op = self.slot_req_field(local, "path")?;
                let ov = self.slot_req_field(local, "version")?;
                let ob = self.slot_req_field(local, "body")?;
                let oraw = self.slot_req_field(local, "raw")?;
                let orlen = self.slot_req_field(local, "raw_len")?;
                let rc = self.fresh();
                writeln!(self.body, "  store ptr null, ptr {om}").ok();
                writeln!(self.body, "  store ptr null, ptr {op}").ok();
                writeln!(self.body, "  store ptr null, ptr {ov}").ok();
                writeln!(self.body, "  store ptr null, ptr {ob}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {raw}, i64 {raw_len}, ptr {om}, ptr {op}, ptr {ov}, ptr {ob})",
                    HOST_HTTP_PARSE_REQUEST.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                writeln!(self.body, "  store ptr {raw}, ptr {oraw}").ok();
                writeln!(self.body, "  store i64 {raw_len}, ptr {orlen}").ok();
                Ok(())
            }
            _ => Err(diag("host_http_server: expected httpParseRequest")),
        }
    }

    pub(super) fn emit_http_res_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "httpParseResponse") =>
            {
                let raw_e =
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: parse res raw"))?;
                let (raw, raw_len) = self.emit_bytes_ptr_len(raw_e)?;
                let ov = self.slot_req_field(local, "version")?;
                let os = self.slot_req_field(local, "status")?;
                let or_ = self.slot_req_field(local, "reason")?;
                let ob = self.slot_req_field(local, "body")?;
                let oraw = self.slot_req_field(local, "raw")?;
                let orlen = self.slot_req_field(local, "raw_len")?;
                let rc = self.fresh();
                writeln!(self.body, "  store ptr null, ptr {ov}").ok();
                writeln!(self.body, "  store i32 0, ptr {os}").ok();
                writeln!(self.body, "  store ptr null, ptr {or_}").ok();
                writeln!(self.body, "  store ptr null, ptr {ob}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {raw}, i64 {raw_len}, ptr {ov}, ptr {os}, ptr {or_}, ptr {ob})",
                    HOST_HTTP_PARSE_RESPONSE.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                writeln!(self.body, "  store ptr {raw}, ptr {oraw}").ok();
                writeln!(self.body, "  store i64 {raw_len}, ptr {orlen}").ok();
                Ok(())
            }
            _ => Err(diag("host_http_server: expected httpParseResponse")),
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
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: close handle"))?,
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
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: write handle"))?,
                )?;
                let (d, n) = self.emit_bytes_ptr_len(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_http_server: write data"))?,
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
                if args.len() == 1 && is_named_callee(callee, "stdoutWrite") =>
            {
                self.emit_stdout_write(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: stdoutWrite"))?,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "httpServeStatic") =>
            {
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: serve conn"))?,
                )?;
                let root = self.emit_string_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_http_server: serve root"))?,
                )?;
                let rc = self.fresh();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i64 {h}, ptr {root})",
                    HOST_HTTP_SERVE_STATIC.symbol
                )
                .ok();
                self.emit_check_rc(&rc)
            }
            _ => Err(diag("host_http_server: unsupported expr stmt")),
        }
    }
}
