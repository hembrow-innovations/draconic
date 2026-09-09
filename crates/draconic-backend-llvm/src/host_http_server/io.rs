use std::fmt::Write as _;

use super::classify::subst_local;
use super::*;

impl<'a> super::Emitter<'a> {
    pub(super) fn emit_stdout_write(&mut self, arg: &Expr) -> Result<(), Diagnostic> {
        let (d, n) = self.emit_bytes_ptr_len(arg)?;
        writeln!(
            self.body,
            "  {}",
            HOST_STDOUT_WRITE.call(&format!("ptr {d}, i64 {n}"))
        )
        .ok();
        Ok(())
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
                _ => Err(diag("host_http_server: bytes arg unsupported")),
            },
            Expr::Call { callee, args, .. }
                if args.len() == 4 && is_named_callee(callee, "httpWriteResponse") =>
            {
                let s = self.emit_write_response(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: status"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_http_server: reason"))?,
                    arg_expr(&args[2]).ok_or_else(|| diag("host_http_server: headers"))?,
                    arg_expr(&args[3]).ok_or_else(|| diag("host_http_server: body"))?,
                )?;
                let n = self.emit_cstr_len(&s)?;
                Ok((s, n))
            }
            Expr::Call { callee, args, .. }
                if args.len() == 4 && is_named_callee(callee, "httpWriteRequest") =>
            {
                let s = self.emit_write_request(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: method"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_http_server: path"))?,
                    arg_expr(&args[2]).ok_or_else(|| diag("host_http_server: headers"))?,
                    arg_expr(&args[3]).ok_or_else(|| diag("host_http_server: body"))?,
                )?;
                let n = self.emit_cstr_len(&s)?;
                Ok((s, n))
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "wsHandshakeResponse") =>
            {
                let s = self.emit_ws_handshake(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: ws key"))?,
                )?;
                let n = self.emit_cstr_len(&s)?;
                Ok((s, n))
            }
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let s = self.emit_string_expr(expr)?;
                let n = self.emit_cstr_len(&s)?;
                let _ = (object, property);
                Ok((s, n))
            }
            _ => Err(diag("host_http_server: bytes arg unsupported")),
        }
    }

    pub(super) fn emit_cstr_len(&mut self, ptr: &str) -> Result<String, Diagnostic> {
        let n = self.fresh();
        writeln!(self.body, "  {n} = call i64 @strlen(ptr {ptr})").ok();
        Ok(n)
    }

    pub(super) fn emit_handle_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if (args.len() == 1 || args.len() == 2) && is_named_callee(callee, "tcpListen") =>
            {
                let port_f = self.emit_number_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: tcpListen port"))?,
                )?;
                let port_i = self.fresh();
                writeln!(self.body, "  {port_i} = fptosi double {port_f} to i32").ok();
                let backlog_i = if args.len() == 2 {
                    let bf = self.emit_number_expr(
                        arg_expr(&args[1])
                            .ok_or_else(|| diag("host_http_server: tcpListen backlog"))?,
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
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: tcpAccept"))?,
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
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: tcpConnect host"))?,
                )?;
                let port_f = self.emit_number_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_http_server: tcpConnect port"))?,
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
                    arg_expr(&args[0])
                        .ok_or_else(|| diag("host_http_server: tlsClientWrap conn"))?,
                )?;
                let name = self.emit_string_expr(
                    arg_expr(&args[1])
                        .ok_or_else(|| diag("host_http_server: tlsClientWrap serverName"))?,
                )?;
                let insecure_f = self.emit_number_expr(
                    arg_expr(&args[2])
                        .ok_or_else(|| diag("host_http_server: tlsClientWrap insecure"))?,
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
                    arg_expr(&args[0])
                        .ok_or_else(|| diag("host_http_server: tlsServerWrap conn"))?,
                )?;
                let cert = self.emit_string_expr(
                    arg_expr(&args[1])
                        .ok_or_else(|| diag("host_http_server: tlsServerWrap certPath"))?,
                )?;
                let key = self.emit_string_expr(
                    arg_expr(&args[2])
                        .ok_or_else(|| diag("host_http_server: tlsServerWrap keyPath"))?,
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
            _ => Err(diag("host_http_server: expected handle expr")),
        }
    }

    pub(super) fn emit_handle_i64(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        let f = self.emit_handle_expr(expr)?;
        let i = self.fresh();
        writeln!(self.body, "  {i} = fptosi double {f} to i64").ok();
        Ok(i)
    }

    pub(super) fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let v = self.fresh();
                let n: f64 = raw.parse().unwrap_or(0.0);
                let lit = if n.fract() == 0.0 {
                    format!("{n:.1}")
                } else {
                    format!("{n}")
                };
                writeln!(self.body, "  {v} = fadd double {lit}, 0.0").ok();
                Ok(v)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "tcpLocalPort") =>
            {
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: tcpLocalPort"))?,
                )?;
                let out_p = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_p} = alloca i32, align 4").ok();
                writeln!(self.body, "  store i32 0, ptr {out_p}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i64 {h}, ptr {out_p})",
                    HOST_TCP_LOCAL_PORT.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let iv = self.fresh();
                let fv = self.fresh();
                writeln!(self.body, "  {iv} = load i32, ptr {out_p}").ok();
                writeln!(self.body, "  {fv} = sitofp i32 {iv} to double").ok();
                Ok(fv)
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
                let prop =
                    string_lit(property).ok_or_else(|| diag("host_http_server: member prop"))?;
                let id = match object.as_ref() {
                    Expr::Local { id, .. } => *id,
                    _ => return Err(diag("host_http_server: member object must be local")),
                };
                match (self.slot_of.get(&id), prop.as_str()) {
                    (Some(SlotTy::HttpRes), "status") => {
                        let fp = self.slot_req_field(id, "status")?;
                        let i = self.fresh();
                        let d = self.fresh();
                        writeln!(self.body, "  {i} = load i32, ptr {fp}").ok();
                        writeln!(self.body, "  {d} = sitofp i32 {i} to double").ok();
                        Ok(d)
                    }
                    _ => Err(diag("host_http_server: unsupported number member")),
                }
            }
            _ => Err(diag("host_http_server: unsupported number expr")),
        }
    }

    pub(super) fn emit_write_response(
        &mut self,
        status: &Expr,
        reason: &Expr,
        headers: &Expr,
        body: &Expr,
    ) -> Result<String, Diagnostic> {
        let st_f = self.emit_number_expr(status)?;
        let st_i = self.fresh();
        writeln!(self.body, "  {st_i} = fptosi double {st_f} to i32").ok();
        let r = self.emit_string_expr(reason)?;
        let h = self.emit_string_expr(headers)?;
        let b = self.emit_string_expr(body)?;
        let blen = self.emit_cstr_len(&b)?;
        let out = self.fresh();
        let rc = self.fresh();
        writeln!(self.body, "  {out} = alloca ptr, align 8").ok();
        writeln!(self.body, "  store ptr null, ptr {out}").ok();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(i32 {st_i}, ptr {r}, ptr {h}, ptr {b}, i64 {blen}, ptr {out})",
            HOST_HTTP_WRITE_RESPONSE.symbol
        )
        .ok();
        self.emit_check_rc(&rc)?;
        let v = self.fresh();
        writeln!(self.body, "  {v} = load ptr, ptr {out}").ok();
        Ok(v)
    }

    pub(super) fn emit_write_request(
        &mut self,
        method: &Expr,
        path: &Expr,
        headers: &Expr,
        body: &Expr,
    ) -> Result<String, Diagnostic> {
        let m = self.emit_string_expr(method)?;
        let p = self.emit_string_expr(path)?;
        let h = self.emit_string_expr(headers)?;
        let b = self.emit_string_expr(body)?;
        let blen = self.emit_cstr_len(&b)?;
        let out = self.fresh();
        let rc = self.fresh();
        writeln!(self.body, "  {out} = alloca ptr, align 8").ok();
        writeln!(self.body, "  store ptr null, ptr {out}").ok();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(ptr {m}, ptr {p}, ptr {h}, ptr {b}, i64 {blen}, ptr {out})",
            HOST_HTTP_WRITE_REQUEST.symbol
        )
        .ok();
        self.emit_check_rc(&rc)?;
        let v = self.fresh();
        writeln!(self.body, "  {v} = load ptr, ptr {out}").ok();
        Ok(v)
    }

    pub(super) fn emit_ws_handshake(&mut self, key: &Expr) -> Result<String, Diagnostic> {
        let k = self.emit_string_expr(key)?;
        let out = self.fresh();
        let rc = self.fresh();
        writeln!(self.body, "  {out} = alloca ptr, align 8").ok();
        writeln!(self.body, "  store ptr null, ptr {out}").ok();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(ptr {k}, ptr {out})",
            HOST_WS_HANDSHAKE_RESPONSE.symbol
        )
        .ok();
        self.emit_check_rc(&rc)?;
        let v = self.fresh();
        writeln!(self.body, "  {v} = load ptr, ptr {out}").ok();
        Ok(v)
    }

    pub(super) fn emit_response_header(&mut self, res: &Expr, name: &Expr) -> Result<String, Diagnostic> {
        let id = match res {
            Expr::Local { id, .. } => *id,
            _ => return Err(diag("host_http_server: res must be local")),
        };
        let rp = self.slot_req_field(id, "raw")?;
        let lp = self.slot_req_field(id, "raw_len")?;
        let raw = self.fresh();
        let len = self.fresh();
        writeln!(self.body, "  {raw} = load ptr, ptr {rp}").ok();
        writeln!(self.body, "  {len} = load i64, ptr {lp}").ok();
        let nm = self.emit_string_expr(name)?;
        let out = self.fresh();
        let rc = self.fresh();
        writeln!(self.body, "  {out} = alloca ptr, align 8").ok();
        writeln!(self.body, "  store ptr null, ptr {out}").ok();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(ptr {raw}, i64 {len}, ptr {nm}, ptr {out})",
            HOST_HTTP_RESPONSE_HEADER.symbol
        )
        .ok();
        self.emit_check_rc(&rc)?;
        let v = self.fresh();
        writeln!(self.body, "  {v} = load ptr, ptr {out}").ok();
        Ok(v)
    }

    pub(super) fn string_fn_id(&self, callee: &Expr) -> Option<LocalId> {
        match callee {
            Expr::Local { id, .. } if self.info.string_fns.contains_key(id) => Some(*id),
            Expr::IdentName { name, .. } => self.info.fn_names.get(name).copied(),
            _ => None,
        }
    }

    pub(super) fn emit_read_file_text(&mut self, path: &Expr) -> Result<String, Diagnostic> {
        let p = self.emit_string_expr(path)?;
        let out = self.fresh();
        let rc = self.fresh();
        writeln!(self.body, "  {out} = alloca ptr, align 8").ok();
        writeln!(self.body, "  store ptr null, ptr {out}").ok();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(ptr {p}, ptr {out})",
            HOST_FS_READ_TEXT.symbol
        )
        .ok();
        self.emit_check_rc(&rc)?;
        let v = self.fresh();
        writeln!(self.body, "  {v} = load ptr, ptr {out}").ok();
        Ok(v)
    }

    pub(super) fn emit_string_fn_call(&mut self, fn_id: LocalId, arg: &Expr) -> Result<String, Diagnostic> {
        let (param, ret) = self
            .info
            .string_fns
            .get(&fn_id)
            .cloned()
            .ok_or_else(|| diag("host_http_server: unknown string fn"))?;
        let subst = subst_local(&ret, param, arg);
        self.emit_string_expr(&subst)
    }

    pub(super) fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => {
                let s = value.to_string_lossy().to_string();
                Ok(self.emit_cstr_ptr(&s))
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 4 && is_named_callee(callee, "httpWriteResponse") =>
            {
                self.emit_write_response(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: status"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_http_server: reason"))?,
                    arg_expr(&args[2]).ok_or_else(|| diag("host_http_server: headers"))?,
                    arg_expr(&args[3]).ok_or_else(|| diag("host_http_server: body"))?,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 4 && is_named_callee(callee, "httpWriteRequest") =>
            {
                self.emit_write_request(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: method"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_http_server: path"))?,
                    arg_expr(&args[2]).ok_or_else(|| diag("host_http_server: headers"))?,
                    arg_expr(&args[3]).ok_or_else(|| diag("host_http_server: body"))?,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "httpResponseHeader") =>
            {
                self.emit_response_header(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: res"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_http_server: header name"))?,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "wsHandshakeResponse") =>
            {
                self.emit_ws_handshake(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: ws key"))?,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "readFileText") =>
            {
                self.emit_read_file_text(
                    arg_expr(&args[0])
                        .ok_or_else(|| diag("host_http_server: readFileText path"))?,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && self.string_fn_id(callee).is_some() =>
            {
                let fid = self.string_fn_id(callee).expect("string fn");
                self.emit_string_fn_call(
                    fid,
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: string fn arg"))?,
                )
            }
            Expr::Binary {
                left,
                op: BinaryOp::Add,
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
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let prop =
                    string_lit(property).ok_or_else(|| diag("host_http_server: member prop"))?;
                let id = match object.as_ref() {
                    Expr::Local { id, .. } => *id,
                    _ => return Err(diag("host_http_server: member object must be local")),
                };
                match (self.slot_of.get(&id), prop.as_str()) {
                    (Some(SlotTy::HttpReq), "method" | "path" | "version" | "body") => {
                        let fp = self.slot_req_field(id, prop.as_str())?;
                        let v = self.fresh();
                        writeln!(self.body, "  {v} = load ptr, ptr {fp}").ok();
                        Ok(v)
                    }
                    (Some(SlotTy::HttpRes), "version" | "reason" | "body") => {
                        let fp = self.slot_req_field(id, prop.as_str())?;
                        let v = self.fresh();
                        writeln!(self.body, "  {v} = load ptr, ptr {fp}").ok();
                        Ok(v)
                    }
                    _ => Err(diag("host_http_server: unsupported string member")),
                }
            }
            _ => Err(diag("host_http_server: unsupported string expr")),
        }
    }
}
