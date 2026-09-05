//! H13.01: HTTP/2 preface + single-stream request/response (RFC 9113).
//!
//! - `http2ClientPreface` / `http2ServerPreface` / `http2SettingsAck` → DynBytes
//! - `http2EncodeRequest(method, path, body)` / `http2EncodeResponse(status, body)` → DynBytes
//! - `http2ParseRequest` → `{ method, path, body, streamId }`
//! - `http2ParseResponse` → `{ status, body, streamId }`
//! - TCP loopback e2e + `stdoutWrite`

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_HANDLE_CLOSE, HOST_HTTP2_CLIENT_OPEN, HOST_HTTP2_CLIENT_PREFACE,
    HOST_HTTP2_ENCODE_REQUEST, HOST_HTTP2_ENCODE_RESPONSE, HOST_HTTP2_PARSE_REQUEST,
    HOST_HTTP2_PARSE_RESPONSE, HOST_HTTP2_SERVER_PREFACE, HOST_HTTP2_SERVER_REPLY,
    HOST_HTTP2_SETTINGS_ACK, HOST_PROCESS_EXIT, HOST_STDERR_WRITE, HOST_STDOUT_WRITE,
    HOST_TCP_ACCEPT, HOST_TCP_CONNECT, HOST_TCP_LISTEN, HOST_TCP_LOCAL_PORT, HOST_TCP_READ,
    HOST_TCP_WRITE, PRINT_I64,
};

pub(crate) fn is_host_http2_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_http2(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_http2 module"))?;
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
    H2Req,
    H2Res,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    print_numbers: Vec<LocalId>,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    print_numbers: Vec<LocalId>,
    has_h2: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_numbers: Vec::new(),
        has_h2: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_h2 {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_numbers: ctx.print_numbers,
    })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            if ty == SlotTy::Number {
                ctx.print_numbers.push(*local);
            }
            Some(())
        }
        Stmt::Expr { expr, .. } => classify_side_effect(expr, ctx),
        _ => None,
    }
}

fn classify_side_effect(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "closeTcp") =>
        {
            classify_handle_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpWrite") =>
        {
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_bytes_arg(arg_expr(&args[1])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "stdoutWrite") =>
        {
            classify_bytes_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "http2ParseRequest") =>
        {
            ctx.has_h2 = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "http2ParseResponse") =>
        {
            ctx.has_h2 = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)
        }
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. }
            if (args.len() == 1 || args.len() == 2) && is_named_callee(callee, "tcpListen") =>
        {
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            if args.len() == 2 {
                classify_number_arg(arg_expr(&args[1])?, ctx)?;
            }
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpAccept") =>
        {
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpConnect") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_number_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpLocalPort") =>
        {
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpRead") =>
        {
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_number_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.is_empty() && is_named_callee(callee, "http2ClientPreface") =>
        {
            ctx.has_h2 = true;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.is_empty() && is_named_callee(callee, "http2ServerPreface") =>
        {
            ctx.has_h2 = true;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.is_empty() && is_named_callee(callee, "http2SettingsAck") =>
        {
            ctx.has_h2 = true;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 3 && is_named_callee(callee, "http2EncodeRequest") =>
        {
            ctx.has_h2 = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_bytes_arg(arg_expr(&args[2])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 3 && is_named_callee(callee, "http2ClientOpen") =>
        {
            ctx.has_h2 = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_bytes_arg(arg_expr(&args[2])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "http2EncodeResponse") =>
        {
            ctx.has_h2 = true;
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            classify_bytes_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "http2ServerReply") =>
        {
            ctx.has_h2 = true;
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            classify_bytes_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "http2ParseRequest") =>
        {
            ctx.has_h2 = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::H2Req)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "http2ParseResponse") =>
        {
            ctx.has_h2 = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::H2Res)
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
                (SlotTy::DynBytes, "length") => Some(SlotTy::Number),
                (SlotTy::H2Req, "method" | "path" | "body") => Some(SlotTy::String),
                (SlotTy::H2Req, "streamId") => Some(SlotTy::Number),
                (SlotTy::H2Res, "body") => Some(SlotTy::String),
                (SlotTy::H2Res, "status" | "streamId") => Some(SlotTy::Number),
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
    match expr {
        Expr::Local { id, .. } => matches!(ctx.slot_of.get(id)?, SlotTy::Handle).then_some(()),
        Expr::Call { .. } => matches!(classify_expr(expr, ctx)?, SlotTy::Handle).then_some(()),
        _ => None,
    }
}

fn classify_number_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Number { .. } => Some(()),
        Expr::Local { id, .. } => {
            matches!(ctx.slot_of.get(id)?, SlotTy::Number | SlotTy::Handle).then_some(())
        }
        Expr::Call { .. } => {
            matches!(classify_expr(expr, ctx)?, SlotTy::Number | SlotTy::Handle).then_some(())
        }
        Expr::Member { .. } => matches!(classify_expr(expr, ctx)?, SlotTy::Number).then_some(()),
        _ => None,
    }
}

fn classify_string_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => matches!(ctx.slot_of.get(id)?, SlotTy::String).then_some(()),
        Expr::Call { .. } => matches!(classify_expr(expr, ctx)?, SlotTy::String).then_some(()),
        Expr::Member { .. } => matches!(classify_expr(expr, ctx)?, SlotTy::String).then_some(()),
        _ => None,
    }
}

fn classify_bytes_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => {
            matches!(ctx.slot_of.get(id)?, SlotTy::String | SlotTy::DynBytes).then_some(())
        }
        Expr::Call { .. } => {
            matches!(classify_expr(expr, ctx)?, SlotTy::DynBytes | SlotTy::String).then_some(())
        }
        Expr::Member { .. } => {
            matches!(classify_expr(expr, ctx)?, SlotTy::String | SlotTy::DynBytes).then_some(())
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

    fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_http2: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn slot_len_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_http2: unknown local"))?;
        Ok(format!("%slot_{name}_len"))
    }

    fn slot_field(&self, id: LocalId, field: &str) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_http2: unknown field local"))?;
        Ok(format!("%slot_{name}_{field}"))
    }

    fn emit_cstr_ptr(&mut self, s: &str) -> String {
        let g = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".str.h2.{}", self.str_globals.len());
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
        let ok = self.fresh();
        let fail = format!("h2_err_{}", self.next_tmp);
        let cont = format!("h2_ok_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  {ok} = icmp eq i32 {rc}, 0").ok();
        writeln!(self.body, "  br i1 {ok}, label %{cont}, label %{fail}").ok();
        writeln!(self.body, "{fail}:").ok();
        let is_inval = self.fresh();
        let inval_l = format!("h2_inval_{}", self.next_tmp);
        let other_l = format!("h2_other_{}", self.next_tmp);
        self.next_tmp += 1;
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
        writeln!(self.body, "{cont}:").ok();
        Ok(())
    }

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(self.out, "; Draconic LLVM host_http2 (H13.01)").ok();
        self.out.push_str(&llvm_declares(&[
            GC_INIT,
            PRINT_I64,
            HOST_TCP_LISTEN,
            HOST_TCP_LOCAL_PORT,
            HOST_TCP_ACCEPT,
            HOST_TCP_CONNECT,
            HOST_TCP_READ,
            HOST_TCP_WRITE,
            HOST_HANDLE_CLOSE,
            HOST_HTTP2_CLIENT_PREFACE,
            HOST_HTTP2_SERVER_PREFACE,
            HOST_HTTP2_SETTINGS_ACK,
            HOST_HTTP2_ENCODE_REQUEST,
            HOST_HTTP2_ENCODE_RESPONSE,
            HOST_HTTP2_CLIENT_OPEN,
            HOST_HTTP2_SERVER_REPLY,
            HOST_HTTP2_PARSE_REQUEST,
            HOST_HTTP2_PARSE_RESPONSE,
            HOST_STDOUT_WRITE,
            HOST_STDERR_WRITE,
            HOST_PROCESS_EXIT,
        ]));
        writeln!(self.out, "declare i64 @strlen(ptr)").ok();
        writeln!(self.out, "declare ptr @malloc(i64)").ok();
        writeln!(
            self.out,
            "declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg)"
        )
        .ok();
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
                SlotTy::H2Req => {
                    for f in ["method", "path", "body"] {
                        let p = self.slot_field(*id, f)?;
                        writeln!(self.body, "  {p} = alloca ptr, align 8").ok();
                    }
                    let bl = self.slot_field(*id, "body_len")?;
                    let sid = self.slot_field(*id, "stream_id")?;
                    writeln!(self.body, "  {bl} = alloca i64, align 8").ok();
                    writeln!(self.body, "  {sid} = alloca i32, align 4").ok();
                }
                SlotTy::H2Res => {
                    let st = self.slot_field(*id, "status")?;
                    let body = self.slot_field(*id, "body")?;
                    let bl = self.slot_field(*id, "body_len")?;
                    let sid = self.slot_field(*id, "stream_id")?;
                    writeln!(self.body, "  {st} = alloca i32, align 4").ok();
                    writeln!(self.body, "  {body} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  {bl} = alloca i64, align 8").ok();
                    writeln!(self.body, "  {sid} = alloca i32, align 4").ok();
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for id in &self.info.print_numbers {
            let ptr = self.slot_ptr(*id)?;
            let v = self.fresh();
            let i = self.fresh();
            writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
            writeln!(self.body, "  {i} = fptosi double {v} to i64").ok();
            writeln!(self.body, "  {}", PRINT_I64.call(&format!("i64 {i}"))).ok();
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
                    .ok_or_else(|| diag("host_http2: declare needs init"))?;
                let ty = self
                    .slot_of
                    .get(local)
                    .copied()
                    .ok_or_else(|| diag("host_http2: unknown slot"))?;
                match ty {
                    SlotTy::Handle | SlotTy::Number => {
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
                    SlotTy::H2Req => self.emit_h2req_into(*local, init)?,
                    SlotTy::H2Res => self.emit_h2res_into(*local, init)?,
                }
                Ok(())
            }
            Stmt::Expr { expr, .. } => self.emit_expr_stmt(expr),
            _ => Err(diag("host_http2: unsupported stmt")),
        }
    }

    fn emit_expr_stmt(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "closeTcp") =>
            {
                let h = self
                    .emit_handle(arg_expr(&args[0]).ok_or_else(|| diag("host_http2: closeTcp"))?)?;
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
                if args.len() == 2 && is_named_callee(callee, "tcpWrite") =>
            {
                let h = self.emit_handle(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http2: tcpWrite h"))?,
                )?;
                let (d, n) = self.emit_bytes_ptr_len(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_http2: tcpWrite b"))?,
                )?;
                let rc = self.fresh();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i64 {h}, ptr {d}, i64 {n})",
                    HOST_TCP_WRITE.symbol
                )
                .ok();
                self.emit_check_rc(&rc)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "stdoutWrite") =>
            {
                let (d, n) = self.emit_bytes_ptr_len(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http2: stdoutWrite"))?,
                )?;
                writeln!(
                    self.body,
                    "  {}",
                    HOST_STDOUT_WRITE.call(&format!("ptr {d}, i64 {n}"))
                )
                .ok();
                Ok(())
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1
                    && (is_named_callee(callee, "http2ParseRequest")
                        || is_named_callee(callee, "http2ParseResponse")) =>
            {
                let (d, n) = self.emit_bytes_ptr_len(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http2: parse discard"))?,
                )?;
                if is_named_callee(callee, "http2ParseRequest") {
                    let om = self.fresh();
                    let op = self.fresh();
                    let ob = self.fresh();
                    let obl = self.fresh();
                    let os = self.fresh();
                    let rc = self.fresh();
                    writeln!(self.body, "  {om} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  {op} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  {ob} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  {obl} = alloca i64, align 8").ok();
                    writeln!(self.body, "  {os} = alloca i32, align 4").ok();
                    writeln!(
                        self.body,
                        "  {rc} = call i32 @{}(ptr {d}, i64 {n}, ptr {om}, ptr {op}, ptr {ob}, ptr {obl}, ptr {os})",
                        HOST_HTTP2_PARSE_REQUEST.symbol
                    )
                    .ok();
                    self.emit_check_rc(&rc)
                } else {
                    let ost = self.fresh();
                    let ob = self.fresh();
                    let obl = self.fresh();
                    let os = self.fresh();
                    let rc = self.fresh();
                    writeln!(self.body, "  {ost} = alloca i32, align 4").ok();
                    writeln!(self.body, "  {ob} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  {obl} = alloca i64, align 8").ok();
                    writeln!(self.body, "  {os} = alloca i32, align 4").ok();
                    writeln!(
                        self.body,
                        "  {rc} = call i32 @{}(ptr {d}, i64 {n}, ptr {ost}, ptr {ob}, ptr {obl}, ptr {os})",
                        HOST_HTTP2_PARSE_RESPONSE.symbol
                    )
                    .ok();
                    self.emit_check_rc(&rc)
                }
            }
            _ => Err(diag("host_http2: unsupported expr stmt")),
        }
    }

    fn emit_dynbytes_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
        let out_data = self.fresh();
        let out_len = self.fresh();
        let rc = self.fresh();
        writeln!(self.body, "  {out_data} = alloca ptr, align 8").ok();
        writeln!(self.body, "  {out_len} = alloca i64, align 8").ok();
        writeln!(self.body, "  store ptr null, ptr {out_data}").ok();
        writeln!(self.body, "  store i64 0, ptr {out_len}").ok();

        match expr {
            Expr::Call { callee, args, .. }
                if args.is_empty() && is_named_callee(callee, "http2ClientPreface") =>
            {
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {out_data}, ptr {out_len})",
                    HOST_HTTP2_CLIENT_PREFACE.symbol
                )
                .ok();
            }
            Expr::Call { callee, args, .. }
                if args.is_empty() && is_named_callee(callee, "http2ServerPreface") =>
            {
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {out_data}, ptr {out_len})",
                    HOST_HTTP2_SERVER_PREFACE.symbol
                )
                .ok();
            }
            Expr::Call { callee, args, .. }
                if args.is_empty() && is_named_callee(callee, "http2SettingsAck") =>
            {
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {out_data}, ptr {out_len})",
                    HOST_HTTP2_SETTINGS_ACK.symbol
                )
                .ok();
            }
            Expr::Call { callee, args, .. }
                if args.len() == 3
                    && (is_named_callee(callee, "http2EncodeRequest")
                        || is_named_callee(callee, "http2ClientOpen")) =>
            {
                let m = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http2: method"))?,
                )?;
                let p = self.emit_string_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_http2: path"))?,
                )?;
                let (bd, bn) = self.emit_bytes_ptr_len(
                    arg_expr(&args[2]).ok_or_else(|| diag("host_http2: body"))?,
                )?;
                let sym = if is_named_callee(callee, "http2ClientOpen") {
                    HOST_HTTP2_CLIENT_OPEN.symbol
                } else {
                    HOST_HTTP2_ENCODE_REQUEST.symbol
                };
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{sym}(ptr {m}, ptr {p}, ptr {bd}, i64 {bn}, ptr {out_data}, ptr {out_len})"
                )
                .ok();
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2
                    && (is_named_callee(callee, "http2EncodeResponse")
                        || is_named_callee(callee, "http2ServerReply")) =>
            {
                let st = self.emit_number_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http2: status"))?,
                )?;
                let sti = self.fresh();
                writeln!(self.body, "  {sti} = fptosi double {st} to i32").ok();
                let (bd, bn) = self.emit_bytes_ptr_len(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_http2: rbody"))?,
                )?;
                let sym = if is_named_callee(callee, "http2ServerReply") {
                    HOST_HTTP2_SERVER_REPLY.symbol
                } else {
                    HOST_HTTP2_ENCODE_RESPONSE.symbol
                };
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{sym}(i32 {sti}, ptr {bd}, i64 {bn}, ptr {out_data}, ptr {out_len})"
                )
                .ok();
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "tcpRead") =>
            {
                let h = self.emit_handle(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http2: tcpRead h"))?,
                )?;
                let max = self.emit_number_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_http2: tcpRead n"))?,
                )?;
                let maxi = self.fresh();
                writeln!(self.body, "  {maxi} = fptosi double {max} to i64").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i64 {h}, i64 {maxi}, ptr {out_data}, ptr {out_len})",
                    HOST_TCP_READ.symbol
                )
                .ok();
            }
            _ => return Err(diag("host_http2: unsupported dynbytes init")),
        }
        self.emit_check_rc(&rc)?;
        let d = self.fresh();
        let n = self.fresh();
        writeln!(self.body, "  {d} = load ptr, ptr {out_data}").ok();
        writeln!(self.body, "  {n} = load i64, ptr {out_len}").ok();
        let ptr = self.slot_ptr(local)?;
        let lp = self.slot_len_ptr(local)?;
        writeln!(self.body, "  store ptr {d}, ptr {ptr}").ok();
        writeln!(self.body, "  store i64 {n}, ptr {lp}").ok();
        Ok(())
    }

    fn emit_h2req_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
        let Expr::Call { callee, args, .. } = expr else {
            return Err(diag("host_http2: h2req needs call"));
        };
        if !(args.len() == 1 && is_named_callee(callee, "http2ParseRequest")) {
            return Err(diag("host_http2: expected http2ParseRequest"));
        }
        let (d, n) = self.emit_bytes_ptr_len(
            arg_expr(&args[0]).ok_or_else(|| diag("host_http2: parse req raw"))?,
        )?;
        let om = self.slot_field(local, "method")?;
        let op = self.slot_field(local, "path")?;
        let ob = self.slot_field(local, "body")?;
        let obl = self.slot_field(local, "body_len")?;
        let os = self.slot_field(local, "stream_id")?;
        let rc = self.fresh();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(ptr {d}, i64 {n}, ptr {om}, ptr {op}, ptr {ob}, ptr {obl}, ptr {os})",
            HOST_HTTP2_PARSE_REQUEST.symbol
        )
        .ok();
        self.emit_check_rc(&rc)
    }

    fn emit_h2res_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
        let Expr::Call { callee, args, .. } = expr else {
            return Err(diag("host_http2: h2res needs call"));
        };
        if !(args.len() == 1 && is_named_callee(callee, "http2ParseResponse")) {
            return Err(diag("host_http2: expected http2ParseResponse"));
        }
        let (d, n) = self.emit_bytes_ptr_len(
            arg_expr(&args[0]).ok_or_else(|| diag("host_http2: parse res raw"))?,
        )?;
        let ost = self.slot_field(local, "status")?;
        let ob = self.slot_field(local, "body")?;
        let obl = self.slot_field(local, "body_len")?;
        let os = self.slot_field(local, "stream_id")?;
        let rc = self.fresh();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(ptr {d}, i64 {n}, ptr {ost}, ptr {ob}, ptr {obl}, ptr {os})",
            HOST_HTTP2_PARSE_RESPONSE.symbol
        )
        .ok();
        self.emit_check_rc(&rc)
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let v = self.fresh();
                let lit = if raw.contains('.') || raw.contains('e') || raw.contains('E') {
                    raw.clone()
                } else {
                    format!("{raw}.0")
                };
                writeln!(self.body, "  {v} = fadd double {lit}, 0.0").ok();
                Ok(v)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "tcpLocalPort") =>
            {
                let h = self.emit_handle(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http2: localPort"))?,
                )?;
                let out = self.fresh();
                let rc = self.fresh();
                let port = self.fresh();
                let d = self.fresh();
                writeln!(self.body, "  {out} = alloca i32, align 4").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i64 {h}, ptr {out})",
                    HOST_TCP_LOCAL_PORT.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                writeln!(self.body, "  {port} = load i32, ptr {out}").ok();
                writeln!(self.body, "  {d} = sitofp i32 {port} to double").ok();
                Ok(d)
            }
            Expr::Call { callee, args, .. }
                if (args.len() == 1 || args.len() == 2) && is_named_callee(callee, "tcpListen") =>
            {
                let port = self.emit_number_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http2: listen port"))?,
                )?;
                let pi = self.fresh();
                writeln!(self.body, "  {pi} = fptosi double {port} to i32").ok();
                let backlog = if args.len() == 2 {
                    let b = self.emit_number_expr(
                        arg_expr(&args[1]).ok_or_else(|| diag("host_http2: backlog"))?,
                    )?;
                    let bi = self.fresh();
                    writeln!(self.body, "  {bi} = fptosi double {b} to i32").ok();
                    bi
                } else {
                    let bi = self.fresh();
                    writeln!(self.body, "  {bi} = add i32 0, 128").ok();
                    bi
                };
                let out = self.fresh();
                let rc = self.fresh();
                let h = self.fresh();
                let d = self.fresh();
                writeln!(self.body, "  {out} = alloca i64, align 8").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i32 {pi}, i32 {backlog}, ptr {out})",
                    HOST_TCP_LISTEN.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                writeln!(self.body, "  {h} = load i64, ptr {out}").ok();
                writeln!(self.body, "  {d} = sitofp i64 {h} to double").ok();
                Ok(d)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "tcpAccept") =>
            {
                let h = self
                    .emit_handle(arg_expr(&args[0]).ok_or_else(|| diag("host_http2: accept"))?)?;
                let out = self.fresh();
                let rc = self.fresh();
                let c = self.fresh();
                let d = self.fresh();
                writeln!(self.body, "  {out} = alloca i64, align 8").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i64 {h}, ptr {out})",
                    HOST_TCP_ACCEPT.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                writeln!(self.body, "  {c} = load i64, ptr {out}").ok();
                writeln!(self.body, "  {d} = sitofp i64 {c} to double").ok();
                Ok(d)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "tcpConnect") =>
            {
                let host = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http2: connect host"))?,
                )?;
                let port = self.emit_number_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_http2: connect port"))?,
                )?;
                let pi = self.fresh();
                writeln!(self.body, "  {pi} = fptosi double {port} to i32").ok();
                let out = self.fresh();
                let rc = self.fresh();
                let c = self.fresh();
                let d = self.fresh();
                writeln!(self.body, "  {out} = alloca i64, align 8").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {host}, i32 {pi}, ptr {out})",
                    HOST_TCP_CONNECT.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                writeln!(self.body, "  {c} = load i64, ptr {out}").ok();
                writeln!(self.body, "  {d} = sitofp i64 {c} to double").ok();
                Ok(d)
            }
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let name = string_lit(property).ok_or_else(|| diag("host_http2: member"))?;
                match &**object {
                    Expr::Local { id, .. } => {
                        let ty = self
                            .slot_of
                            .get(id)
                            .copied()
                            .ok_or_else(|| diag("host_http2: member local"))?;
                        match (ty, name.as_str()) {
                            (SlotTy::DynBytes, "length") => {
                                let lp = self.slot_len_ptr(*id)?;
                                let n = self.fresh();
                                let d = self.fresh();
                                writeln!(self.body, "  {n} = load i64, ptr {lp}").ok();
                                writeln!(self.body, "  {d} = sitofp i64 {n} to double").ok();
                                Ok(d)
                            }
                            (SlotTy::H2Req, "streamId") | (SlotTy::H2Res, "streamId") => {
                                let p = self.slot_field(*id, "stream_id")?;
                                let v = self.fresh();
                                let d = self.fresh();
                                writeln!(self.body, "  {v} = load i32, ptr {p}").ok();
                                writeln!(self.body, "  {d} = sitofp i32 {v} to double").ok();
                                Ok(d)
                            }
                            (SlotTy::H2Res, "status") => {
                                let p = self.slot_field(*id, "status")?;
                                let v = self.fresh();
                                let d = self.fresh();
                                writeln!(self.body, "  {v} = load i32, ptr {p}").ok();
                                writeln!(self.body, "  {d} = sitofp i32 {v} to double").ok();
                                Ok(d)
                            }
                            _ => Err(diag("host_http2: bad number member")),
                        }
                    }
                    _ => Err(diag("host_http2: number member obj")),
                }
            }
            _ => Err(diag("host_http2: unsupported number expr")),
        }
    }

    fn emit_handle(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        let d = self.emit_number_expr(expr)?;
        let h = self.fresh();
        writeln!(self.body, "  {h} = fptosi double {d} to i64").ok();
        Ok(h)
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
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let name = string_lit(property).ok_or_else(|| diag("host_http2: str member"))?;
                match &**object {
                    Expr::Local { id, .. } => {
                        let ty = self
                            .slot_of
                            .get(id)
                            .copied()
                            .ok_or_else(|| diag("host_http2: str member local"))?;
                        match (ty, name.as_str()) {
                            (SlotTy::H2Req, "method") => {
                                let p = self.slot_field(*id, "method")?;
                                let v = self.fresh();
                                writeln!(self.body, "  {v} = load ptr, ptr {p}").ok();
                                Ok(v)
                            }
                            (SlotTy::H2Req, "path") => {
                                let p = self.slot_field(*id, "path")?;
                                let v = self.fresh();
                                writeln!(self.body, "  {v} = load ptr, ptr {p}").ok();
                                Ok(v)
                            }
                            (SlotTy::H2Req, "body") | (SlotTy::H2Res, "body") => {
                                self.emit_body_cstr(*id)
                            }
                            _ => Err(diag("host_http2: bad string member")),
                        }
                    }
                    _ => Err(diag("host_http2: string member obj")),
                }
            }
            _ => Err(diag("host_http2: unsupported string expr")),
        }
    }

    fn emit_body_cstr(&mut self, id: LocalId) -> Result<String, Diagnostic> {
        let bp = self.slot_field(id, "body")?;
        let bl = self.slot_field(id, "body_len")?;
        let d = self.fresh();
        let n = self.fresh();
        writeln!(self.body, "  {d} = load ptr, ptr {bp}").ok();
        writeln!(self.body, "  {n} = load i64, ptr {bl}").ok();
        let buf = self.fresh();
        let np1 = self.fresh();
        writeln!(self.body, "  {np1} = add i64 {n}, 1").ok();
        writeln!(self.body, "  {buf} = call ptr @malloc(i64 {np1})").ok();
        let copy_ok = format!("h2_body_copy_{}", self.next_tmp);
        let copy_skip = format!("h2_body_skip_{}", self.next_tmp);
        let copy_done = format!("h2_body_done_{}", self.next_tmp);
        self.next_tmp += 1;
        let is_null = self.fresh();
        writeln!(self.body, "  {is_null} = icmp eq ptr {d}, null").ok();
        writeln!(
            self.body,
            "  br i1 {is_null}, label %{copy_skip}, label %{copy_ok}"
        )
        .ok();
        writeln!(self.body, "{copy_ok}:").ok();
        writeln!(
            self.body,
            "  call void @llvm.memcpy.p0.p0.i64(ptr {buf}, ptr {d}, i64 {n}, i1 false)"
        )
        .ok();
        writeln!(self.body, "  br label %{copy_done}").ok();
        writeln!(self.body, "{copy_skip}:").ok();
        writeln!(self.body, "  br label %{copy_done}").ok();
        writeln!(self.body, "{copy_done}:").ok();
        let endp = self.fresh();
        writeln!(
            self.body,
            "  {endp} = getelementptr inbounds i8, ptr {buf}, i64 {n}"
        )
        .ok();
        writeln!(self.body, "  store i8 0, ptr {endp}").ok();
        Ok(buf)
    }

    fn emit_bytes_ptr_len(&mut self, expr: &Expr) -> Result<(String, String), Diagnostic> {
        match expr {
            Expr::String { value, .. } => {
                let s = value.to_string_lossy().to_string();
                let p = self.emit_cstr_ptr(&s);
                let n = self.fresh();
                writeln!(self.body, "  {n} = add i64 0, {}", s.len()).ok();
                Ok((p, n))
            }
            Expr::Local { id, .. } => {
                let ty = self
                    .slot_of
                    .get(id)
                    .copied()
                    .ok_or_else(|| diag("host_http2: bytes local"))?;
                match ty {
                    SlotTy::String => {
                        let ptr = self.slot_ptr(*id)?;
                        let p = self.fresh();
                        let n = self.fresh();
                        writeln!(self.body, "  {p} = load ptr, ptr {ptr}").ok();
                        writeln!(self.body, "  {n} = call i64 @strlen(ptr {p})").ok();
                        Ok((p, n))
                    }
                    SlotTy::DynBytes => {
                        let ptr = self.slot_ptr(*id)?;
                        let lp = self.slot_len_ptr(*id)?;
                        let p = self.fresh();
                        let n = self.fresh();
                        writeln!(self.body, "  {p} = load ptr, ptr {ptr}").ok();
                        writeln!(self.body, "  {n} = load i64, ptr {lp}").ok();
                        Ok((p, n))
                    }
                    _ => Err(diag("host_http2: bad bytes local ty")),
                }
            }
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let name = string_lit(property).ok_or_else(|| diag("host_http2: bytes member"))?;
                match &**object {
                    Expr::Local { id, .. } => match name.as_str() {
                        "body" => {
                            let bp = self.slot_field(*id, "body")?;
                            let bl = self.slot_field(*id, "body_len")?;
                            let p = self.fresh();
                            let n = self.fresh();
                            writeln!(self.body, "  {p} = load ptr, ptr {bp}").ok();
                            writeln!(self.body, "  {n} = load i64, ptr {bl}").ok();
                            Ok((p, n))
                        }
                        "path" | "method" => {
                            let p = self.emit_string_expr(expr)?;
                            let n = self.fresh();
                            writeln!(self.body, "  {n} = call i64 @strlen(ptr {p})").ok();
                            Ok((p, n))
                        }
                        _ => Err(diag("host_http2: bad bytes member")),
                    },
                    _ => Err(diag("host_http2: bytes member obj")),
                }
            }
            Expr::Call { .. } => {
                // materialize into temp via unsupported — callers should assign first
                Err(diag("host_http2: bytes from call needs binding"))
            }
            _ => Err(diag("host_http2: unsupported bytes expr")),
        }
    }
}
