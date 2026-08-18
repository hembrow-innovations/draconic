//! H10.03–H10.05 + H11.03 + H12.01: HTTP/1.1 server + client over TCP or TLS.
//!
//! Combines host TCP (listen/accept/connect/read/write/close) with HTTP parse/write
//! so a Program can serve one or more requests on loopback without closing between:
//! accept → (`tcpRead` → `httpParseRequest` → `httpWriteResponse` → `tcpWrite`)+ → close.
//!
//! H10.05 client path: `httpWriteRequest` + `tcpWrite` + `tcpRead` + `httpParseResponse`.
//!
//! H11.03 HTTPS: same shapes with `tlsClientWrap` / `tlsServerWrap` + `tlsRead` /
//! `tlsWrite` / `closeTls` instead of plain TCP I/O (dual-process loopback).
//!
//! H12.01: `wsHandshakeResponse(key)` → RFC 6455 101 upgrade response bytes.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_HANDLE_CLOSE, HOST_HTTP_PARSE_REQUEST, HOST_HTTP_PARSE_RESPONSE,
    HOST_HTTP_RESPONSE_HEADER, HOST_HTTP_WRITE_REQUEST, HOST_HTTP_WRITE_RESPONSE,
    HOST_PROCESS_EXIT, HOST_STDERR_WRITE, HOST_STDOUT_WRITE, HOST_TCP_ACCEPT, HOST_TCP_CONNECT,
    HOST_TCP_LISTEN, HOST_TCP_LOCAL_PORT, HOST_TCP_READ, HOST_TCP_WRITE, HOST_TLS_CLIENT_WRAP,
    HOST_TLS_READ, HOST_TLS_SERVER_WRAP, HOST_TLS_WRITE, HOST_WS_HANDSHAKE_RESPONSE, PRINT_I64,
    PRINT_STR,
};

pub(crate) fn is_host_http_server_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_http_server(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_http_server module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Handle,
    Number,
    String,
    DynBytes,
    HttpReq,
    HttpRes,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
    /// H10.05 client observations: auto-print string/number locals at end.
    client_print: bool,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    print_locals: Vec<(LocalId, SlotTy)>,
    has_tcp: bool,
    has_http: bool,
    has_client: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_locals: Vec::new(),
        has_tcp: false,
        has_http: false,
        has_client: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !(ctx.has_tcp && ctx.has_http) {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_locals: ctx.print_locals,
        client_print: ctx.has_client,
    })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            // H10.05: only auto-print response field / header observations.
            if is_client_observation(init, ctx) {
                ctx.print_locals.push((*local, ty));
            }
            Some(())
        }
        Stmt::Expr { expr, .. } => classify_side_effect(expr, ctx),
        // H17.01: accept-loop server body (`while (true) { … }`).
        Stmt::Block { body, .. } => {
            for s in body {
                classify_stmt(s, ctx)?;
            }
            Some(())
        }
        Stmt::While { test, body, .. } => {
            classify_while_test(test)?;
            classify_stmt(body, ctx)
        }
        _ => None,
    }
}

fn classify_while_test(test: &Expr) -> Option<()> {
    match test {
        Expr::Boolean { value: true, .. } => Some(()),
        Expr::Number { raw, .. } if raw == "1" => Some(()),
        _ => None,
    }
}

fn is_client_observation(expr: &Expr, ctx: &ClassifyCtx) -> bool {
    match expr {
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let Some(name) = string_lit(property) else {
                return false;
            };
            match object.as_ref() {
                Expr::Local { id, .. } => match (ctx.slot_of.get(id), name.as_str()) {
                    (Some(SlotTy::HttpRes), "version" | "reason" | "body" | "status") => true,
                    _ => false,
                },
                _ => false,
            }
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpResponseHeader") =>
        {
            true
        }
        _ => false,
    }
}

fn classify_side_effect(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1
                && (is_named_callee(callee, "closeTcp") || is_named_callee(callee, "closeTls")) =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2
                && (is_named_callee(callee, "tcpWrite") || is_named_callee(callee, "tlsWrite")) =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_bytes_arg(arg_expr(&args[1])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "stdoutWrite") =>
        {
            classify_bytes_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "httpParseRequest") =>
        {
            ctx.has_http = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "httpParseResponse") =>
        {
            ctx.has_http = true;
            ctx.has_client = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteResponse") =>
        {
            ctx.has_http = true;
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            classify_string_arg(arg_expr(&args[3])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteRequest") =>
        {
            ctx.has_http = true;
            ctx.has_client = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            classify_string_arg(arg_expr(&args[3])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsHandshakeResponse") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpResponseHeader") =>
        {
            ctx.has_http = true;
            ctx.has_client = true;
            classify_res_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)
        }
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. }
            if (args.len() == 1 || args.len() == 2) && is_named_callee(callee, "tcpListen") =>
        {
            ctx.has_tcp = true;
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            if args.len() == 2 {
                classify_number_arg(arg_expr(&args[1])?, ctx)?;
            }
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpAccept") =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpConnect") =>
        {
            ctx.has_tcp = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_number_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpLocalPort") =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2
                && (is_named_callee(callee, "tcpRead") || is_named_callee(callee, "tlsRead")) =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_number_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 3 && is_named_callee(callee, "tlsClientWrap") =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_number_arg(arg_expr(&args[2])?, ctx)?;
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 3 && is_named_callee(callee, "tlsServerWrap") =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "httpParseRequest") =>
        {
            ctx.has_http = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::HttpReq)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteResponse") =>
        {
            ctx.has_http = true;
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            classify_string_arg(arg_expr(&args[3])?, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteRequest") =>
        {
            ctx.has_http = true;
            ctx.has_client = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            classify_string_arg(arg_expr(&args[3])?, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "httpParseResponse") =>
        {
            ctx.has_http = true;
            ctx.has_client = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::HttpRes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpResponseHeader") =>
        {
            ctx.has_http = true;
            ctx.has_client = true;
            classify_res_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsHandshakeResponse") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let ot = classify_expr(object, ctx)?;
            let name = string_lit(property)?;
            match (ot, name.as_str()) {
                (SlotTy::HttpReq, "method" | "path" | "version" | "body") => Some(SlotTy::String),
                (SlotTy::HttpRes, "version" | "reason" | "body") => Some(SlotTy::String),
                (SlotTy::HttpRes, "status") => Some(SlotTy::Number),
                (SlotTy::DynBytes, "length") => Some(SlotTy::Number),
                _ => None,
            }
        }
        Expr::String { .. } => Some(SlotTy::String),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        _ => None,
    }
}

fn classify_handle_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match classify_expr(expr, ctx)? {
        SlotTy::Handle => Some(()),
        _ => None,
    }
}

fn classify_number_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Number { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::Number | SlotTy::Handle => Some(()),
            _ => None,
        },
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpLocalPort") =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let ot = classify_expr(object, ctx)?;
            let name = string_lit(property)?;
            match (ot, name.as_str()) {
                (SlotTy::HttpRes, "status") => Some(()),
                _ => None,
            }
        }
        _ => None,
    }
}

fn classify_string_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::String => Some(()),
            _ => None,
        },
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let ot = classify_expr(object, ctx)?;
            let name = string_lit(property)?;
            match (ot, name.as_str()) {
                (SlotTy::HttpReq, "method" | "path" | "version" | "body") => Some(()),
                (SlotTy::HttpRes, "version" | "reason" | "body") => Some(()),
                _ => None,
            }
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteResponse") =>
        {
            ctx.has_http = true;
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            classify_string_arg(arg_expr(&args[3])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteRequest") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            classify_string_arg(arg_expr(&args[3])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpResponseHeader") =>
        {
            ctx.has_http = true;
            classify_res_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsHandshakeResponse") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)
        }
        _ => None,
    }
}

fn classify_bytes_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::String | SlotTy::DynBytes => Some(()),
            _ => None,
        },
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteResponse") =>
        {
            classify_string_arg(expr, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteRequest") =>
        {
            classify_string_arg(expr, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsHandshakeResponse") =>
        {
            classify_string_arg(expr, ctx)
        }
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let ot = classify_expr(object, ctx)?;
            let name = string_lit(property)?;
            match (ot, name.as_str()) {
                (SlotTy::HttpReq, "method" | "path" | "version" | "body") => Some(()),
                (SlotTy::HttpRes, "version" | "reason" | "body") => Some(()),
                _ => None,
            }
        }
        _ => None,
    }
}

fn classify_res_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::HttpRes => Some(()),
            _ => None,
        },
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "httpParseResponse") =>
        {
            ctx.has_http = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)
        }
        _ => None,
    }
}

fn is_named_callee(expr: &Expr, want: &str) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == want)
}

fn arg_expr(arg: &Arg) -> Option<&Expr> {
    match arg {
        Arg::Expr(e) => Some(e),
        _ => None,
    }
}

fn string_lit(expr: &Expr) -> Option<String> {
    match expr {
        Expr::String { value, .. } => Some(value.to_string_lossy().to_string()),
        _ => None,
    }
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}

fn escape_llvm_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            c if (0x20..0x7f).contains(&c) && c != b'"' => out.push(c as char),
            c => out.push_str(&format!("\\{c:02X}")),
        }
    }
    out
}

struct Emitter<'a> {
    module: &'a Module,
    info: &'a ModuleInfo,
    out: String,
    body: String,
    next_tmp: usize,
    str_globals: Vec<(String, String)>,
    local_name: HashMap<LocalId, String>,
    slot_of: HashMap<LocalId, SlotTy>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
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

    fn finish(self) -> String {
        self.out
    }

    fn fresh(&mut self) -> String {
        let n = self.next_tmp;
        self.next_tmp += 1;
        format!("%t{n}")
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let n = self.next_tmp;
        self.next_tmp += 1;
        format!("{prefix}_{n}")
    }

    fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_http_server: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn slot_len_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_http_server: unknown local"))?;
        Ok(format!("%slot_{name}_len"))
    }

    fn slot_req_field(&self, id: LocalId, field: &str) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_http_server: unknown req local"))?;
        Ok(format!("%slot_{name}_{field}"))
    }

    fn emit_cstr_ptr(&mut self, s: &str) -> String {
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

    fn emit_host_err_exit(&mut self, code: &str) -> Result<(), Diagnostic> {
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

    fn emit_check_rc(&mut self, rc: &str) -> Result<(), Diagnostic> {
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

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
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
            HOST_HTTP_WRITE_REQUEST,
            HOST_HTTP_PARSE_RESPONSE,
            HOST_HTTP_RESPONSE_HEADER,
            HOST_WS_HANDSHAKE_RESPONSE,
            HOST_STDOUT_WRITE,
            HOST_STDERR_WRITE,
            HOST_PROCESS_EXIT,
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

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
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
            _ => Err(diag("host_http_server: unsupported stmt")),
        }
    }

    fn emit_dynbytes_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
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
            _ => Err(diag("host_http_server: expected tcpRead/tlsRead for DynBytes")),
        }
    }

    fn emit_http_req_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
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

    fn emit_http_res_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
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

    fn emit_expr_stmt(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
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
            _ => Err(diag("host_http_server: unsupported expr stmt")),
        }
    }

    fn emit_stdout_write(&mut self, arg: &Expr) -> Result<(), Diagnostic> {
        let (d, n) = self.emit_bytes_ptr_len(arg)?;
        writeln!(
            self.body,
            "  {}",
            HOST_STDOUT_WRITE.call(&format!("ptr {d}, i64 {n}"))
        )
        .ok();
        Ok(())
    }

    fn emit_bytes_ptr_len(&mut self, expr: &Expr) -> Result<(String, String), Diagnostic> {
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

    fn emit_cstr_len(&mut self, ptr: &str) -> Result<String, Diagnostic> {
        let n = self.fresh();
        writeln!(self.body, "  {n} = call i64 @strlen(ptr {ptr})").ok();
        Ok(n)
    }

    fn emit_handle_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
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
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: tlsClientWrap conn"))?,
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
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http_server: tlsServerWrap conn"))?,
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

    fn emit_handle_i64(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        let f = self.emit_handle_expr(expr)?;
        let i = self.fresh();
        writeln!(self.body, "  {i} = fptosi double {f} to i64").ok();
        Ok(i)
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
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

    fn emit_write_response(
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

    fn emit_write_request(
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

    fn emit_ws_handshake(&mut self, key: &Expr) -> Result<String, Diagnostic> {
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

    fn emit_response_header(&mut self, res: &Expr, name: &Expr) -> Result<String, Diagnostic> {
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

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    fn lower_src(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn emit_server_oneshot_ir() {
        let m = lower_src(
            r#"
            let s = tcpListen(0);
            let c = tcpConnect("127.0.0.1", tcpLocalPort(s));
            let a = tcpAccept(s);
            tcpWrite(c, "GET /hello HTTP/1.1\r\nHost: x\r\n\r\n");
            let raw = tcpRead(a, 4096);
            let req = httpParseRequest(raw);
            let path = req.path;
            let resp = httpWriteResponse(200, "OK", "Content-Type: text/plain\r\n", path);
            tcpWrite(a, resp);
            closeTcp(a);
            let out = tcpRead(c, 4096);
            stdoutWrite(out);
            closeTcp(c);
            closeTcp(s);
            "#,
        );
        assert!(is_host_http_server_module(&m));
        let ir = emit_host_http_server(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_tcp_listen"), "{ir}");
        assert!(ir.contains("draconic_rt_host_http_parse_request"), "{ir}");
        assert!(ir.contains("draconic_rt_host_http_write_response"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tcp_write"), "{ir}");
    }

    #[test]
    fn emit_https_client_ir() {
        // H11.03: HTTP/1.1 client over TLS (insecure).
        let m = lower_src(
            r#"
            let c = tcpConnect("127.0.0.1", 4433);
            let t = tlsClientWrap(c, "localhost", 1);
            let reqMsg = httpWriteRequest("GET", "/hello", "Host: localhost\r\n", "");
            tlsWrite(t, reqMsg);
            let out = tlsRead(t, 4096);
            let res = httpParseResponse(out);
            let v = res.version;
            let st = res.status;
            let r = res.reason;
            let b = res.body;
            closeTls(t);
            "#,
        );
        assert!(is_host_http_server_module(&m));
        let ir = emit_host_http_server(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_tls_client_wrap"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tls_write"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tls_read"), "{ir}");
        assert!(ir.contains("draconic_rt_host_http_write_request"), "{ir}");
        assert!(ir.contains("draconic_rt_host_http_parse_response"), "{ir}");
    }

    #[test]
    fn emit_https_server_ir() {
        // H11.03: HTTP/1.1 server over TLS.
        let m = lower_src(
            r#"
            let s = tcpListen(4433);
            let a = tcpAccept(s);
            let t = tlsServerWrap(a, "/tmp/cert.pem", "/tmp/key.pem");
            let raw = tlsRead(t, 4096);
            let req = httpParseRequest(raw);
            let path = req.path;
            let resp = httpWriteResponse(200, "OK", "Content-Type: text/plain\r\n", path);
            tlsWrite(t, resp);
            closeTls(t);
            closeTcp(s);
            "#,
        );
        assert!(is_host_http_server_module(&m));
        let ir = emit_host_http_server(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_tls_server_wrap"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tls_read"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tls_write"), "{ir}");
        assert!(ir.contains("draconic_rt_host_http_parse_request"), "{ir}");
        assert!(ir.contains("draconic_rt_host_http_write_response"), "{ir}");
    }

    #[test]
    fn emit_http_echo_accept_loop_ir() {
        // H17.01 shape: listen + while(true) accept/parse/write/close.
        let m = lower_src(
            r#"
            let s = tcpListen(8080);
            stdoutWrite("http-echo listening on 8080\n");
            while (true) {
              let a = tcpAccept(s);
              let raw = tcpRead(a, 65536);
              let req = httpParseRequest(raw);
              let path = req.path;
              let resp = httpWriteResponse(200, "OK", "Content-Type: text/plain\r\n", path);
              tcpWrite(a, resp);
              closeTcp(a);
            }
            "#,
        );
        assert!(is_host_http_server_module(&m));
        let ir = emit_host_http_server(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_tcp_listen"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tcp_accept"), "{ir}");
        assert!(ir.contains("draconic_rt_host_http_parse_request"), "{ir}");
        assert!(ir.contains("draconic_rt_host_http_write_response"), "{ir}");
        assert!(ir.contains("hs_while_head"), "{ir}");
        assert!(ir.contains("hs_while_body"), "{ir}");
    }
}
