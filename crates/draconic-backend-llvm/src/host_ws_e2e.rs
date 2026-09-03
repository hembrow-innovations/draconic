//! H12.03: WebSocket client dial + echo e2e (RFC 6455).
//!
//! Combines TCP (listen/accept/connect/read/write/close) with:
//! - `wsClientHandshakeRequest(path, host, key)` → request string
//! - `wsClientCheckAccept(response, key)` → void (EINVAL on fail)
//! - `wsHandshakeResponse(key)` → 101 response string
//! - `wsEncodeTextClient` / `wsEncodeText` / `wsEncodeBinary` / `wsEncodeClose` /
//!   `wsEncodePing` / `wsEncodePong` / `wsDecodeFrame` + frame fields
//! - `stdoutWrite`

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_HANDLE_CLOSE, HOST_PROCESS_EXIT, HOST_STDERR_WRITE,
    HOST_STDOUT_WRITE, HOST_TCP_ACCEPT, HOST_TCP_CONNECT, HOST_TCP_LISTEN, HOST_TCP_LOCAL_PORT,
    HOST_TCP_READ, HOST_TCP_WRITE, HOST_WS_CLIENT_CHECK_ACCEPT, HOST_WS_CLIENT_HANDSHAKE_REQUEST,
    HOST_WS_DECODE_FRAME, HOST_WS_ENCODE_BINARY, HOST_WS_ENCODE_CLOSE, HOST_WS_ENCODE_PING,
    HOST_WS_ENCODE_PONG, HOST_WS_ENCODE_TEXT, HOST_WS_ENCODE_TEXT_CLIENT,
    HOST_WS_HANDSHAKE_RESPONSE, PRINT_I64, PRINT_STR,
};

pub(crate) fn is_host_ws_e2e_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_ws_e2e(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_ws_e2e module"))?;
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
    WsFrame,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    has_tcp: bool,
    has_ws_client: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        slot_of: HashMap::new(),
        has_tcp: false,
        has_ws_client: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    // TCP optional: `wsClientCheckAccept` alone is valid (H12.03 negative path).
    if !ctx.has_ws_client {
        return None;
    }
    Some(ModuleInfo { slots: ctx.slots })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
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
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpWrite") =>
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
            if args.len() == 2 && is_named_callee(callee, "wsClientCheckAccept") =>
        {
            ctx.has_ws_client = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsDecodeFrame") =>
        {
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
            if args.len() == 2 && is_named_callee(callee, "tcpRead") =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_number_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 3 && is_named_callee(callee, "wsClientHandshakeRequest") =>
        {
            ctx.has_ws_client = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsHandshakeResponse") =>
        {
            // Allowed in e2e when paired with client APIs; does not claim alone.
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsEncodeTextClient") =>
        {
            ctx.has_ws_client = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsEncodeText") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsEncodeBinary") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "wsEncodeClose") =>
        {
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsEncodePing") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsEncodePong") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsDecodeFrame") =>
        {
            classify_bytes_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::WsFrame)
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
                (SlotTy::WsFrame, "fin" | "opcode" | "closeCode") => Some(SlotTy::Number),
                (SlotTy::WsFrame, "payload") => Some(SlotTy::String),
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
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::Handle => Some(()),
            _ => None,
        },
        Expr::Call { .. } => matches!(classify_expr(expr, ctx)?, SlotTy::Handle).then_some(()),
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
        Expr::Call { .. } => {
            let ty = classify_expr(expr, ctx)?;
            matches!(ty, SlotTy::Number | SlotTy::Handle).then_some(())
        }
        Expr::Member { .. } => matches!(classify_expr(expr, ctx)?, SlotTy::Number).then_some(()),
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
        Expr::Call { .. } => matches!(classify_expr(expr, ctx)?, SlotTy::String).then_some(()),
        Expr::Member { .. } => matches!(classify_expr(expr, ctx)?, SlotTy::String).then_some(()),
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
        Expr::Call { .. } => {
            let ty = classify_expr(expr, ctx)?;
            matches!(ty, SlotTy::DynBytes | SlotTy::String).then_some(())
        }
        Expr::Member { .. } => {
            let ty = classify_expr(expr, ctx)?;
            matches!(ty, SlotTy::String | SlotTy::DynBytes).then_some(())
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
            .ok_or_else(|| diag("host_ws_e2e: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn slot_len_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_ws_e2e: unknown local"))?;
        Ok(format!("%slot_{name}_len"))
    }

    fn slot_frame_field(&self, id: LocalId, field: &str) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_ws_e2e: unknown frame local"))?;
        Ok(format!("%slot_{name}_{field}"))
    }

    fn emit_cstr_ptr(&mut self, s: &str) -> String {
        let g = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".str.wse2e.{}", self.str_globals.len());
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
        let fail = format!("wse2e_err_{}", self.next_tmp);
        let cont = format!("wse2e_ok_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  {ok} = icmp eq i32 {rc}, 0").ok();
        writeln!(self.body, "  br i1 {ok}, label %{cont}, label %{fail}").ok();
        writeln!(self.body, "{fail}:").ok();
        let is_inval = self.fresh();
        let inval_l = format!("wse2e_inval_{}", self.next_tmp);
        let other_l = format!("wse2e_other_{}", self.next_tmp);
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
        writeln!(self.out, "; Draconic LLVM host_ws_e2e (H12.03 client dial)").ok();
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
            HOST_HANDLE_CLOSE,
            HOST_WS_HANDSHAKE_RESPONSE,
            HOST_WS_CLIENT_HANDSHAKE_REQUEST,
            HOST_WS_CLIENT_CHECK_ACCEPT,
            HOST_WS_ENCODE_TEXT,
            HOST_WS_ENCODE_BINARY,
            HOST_WS_ENCODE_CLOSE,
            HOST_WS_ENCODE_PING,
            HOST_WS_ENCODE_PONG,
            HOST_WS_ENCODE_TEXT_CLIENT,
            HOST_WS_DECODE_FRAME,
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
                SlotTy::WsFrame => {
                    for f in ["fin", "opcode", "close_code"] {
                        let p = self.slot_frame_field(*id, f)?;
                        writeln!(self.body, "  {p} = alloca i32, align 4").ok();
                    }
                    let pp = self.slot_frame_field(*id, "payload")?;
                    let pl = self.slot_frame_field(*id, "payload_len")?;
                    writeln!(self.body, "  {pp} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  {pl} = alloca i64, align 8").ok();
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
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
                    .ok_or_else(|| diag("host_ws_e2e: declare needs init"))?;
                let ty = self
                    .slot_of
                    .get(local)
                    .copied()
                    .ok_or_else(|| diag("host_ws_e2e: unknown slot"))?;
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
                    SlotTy::WsFrame => self.emit_frame_into(*local, init)?,
                }
                Ok(())
            }
            Stmt::Expr { expr, .. } => self.emit_expr_stmt(expr),
            _ => Err(diag("host_ws_e2e: unsupported stmt")),
        }
    }

    fn emit_expr_stmt(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "closeTcp") =>
            {
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: closeTcp"))?,
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
                if args.len() == 2 && is_named_callee(callee, "tcpWrite") =>
            {
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: tcpWrite h"))?,
                )?;
                let (d, n) = self.emit_bytes_ptr_len(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_ws_e2e: tcpWrite data"))?,
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
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: stdoutWrite"))?,
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
                if args.len() == 2 && is_named_callee(callee, "wsClientCheckAccept") =>
            {
                let (d, n) = self.emit_bytes_ptr_len(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: check data"))?,
                )?;
                let key = self.emit_string_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_ws_e2e: check key"))?,
                )?;
                let rc = self.fresh();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {d}, i64 {n}, ptr {key})",
                    HOST_WS_CLIENT_CHECK_ACCEPT.symbol
                )
                .ok();
                self.emit_check_rc(&rc)
            }
            _ => Err(diag("host_ws_e2e: unsupported expr stmt")),
        }
    }

    fn emit_dynbytes_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "tcpRead") =>
            {
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: tcpRead h"))?,
                )?;
                let max_f = self.emit_number_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_ws_e2e: tcpRead max"))?,
                )?;
                let max_i = self.fresh();
                writeln!(self.body, "  {max_i} = fptosi double {max_f} to i64").ok();
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
                    HOST_TCP_READ.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let d = self.fresh();
                let n = self.fresh();
                writeln!(self.body, "  {d} = load ptr, ptr {out_data}").ok();
                writeln!(self.body, "  {n} = load i64, ptr {out_len}").ok();
                let dp = self.slot_ptr(local)?;
                let lp = self.slot_len_ptr(local)?;
                writeln!(self.body, "  store ptr {d}, ptr {dp}").ok();
                writeln!(self.body, "  store i64 {n}, ptr {lp}").ok();
                Ok(())
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "wsEncodeTextClient") =>
            {
                let p = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: text client"))?,
                )?;
                let out_data = self.fresh();
                let out_len = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_data} = alloca ptr, align 8").ok();
                writeln!(self.body, "  {out_len} = alloca i64, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {out_data}").ok();
                writeln!(self.body, "  store i64 0, ptr {out_len}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {p}, ptr {out_data}, ptr {out_len})",
                    HOST_WS_ENCODE_TEXT_CLIENT.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let d = self.fresh();
                let n = self.fresh();
                writeln!(self.body, "  {d} = load ptr, ptr {out_data}").ok();
                writeln!(self.body, "  {n} = load i64, ptr {out_len}").ok();
                let dp = self.slot_ptr(local)?;
                let lp = self.slot_len_ptr(local)?;
                writeln!(self.body, "  store ptr {d}, ptr {dp}").ok();
                writeln!(self.body, "  store i64 {n}, ptr {lp}").ok();
                Ok(())
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "wsEncodeText") =>
            {
                let p = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: text"))?,
                )?;
                let out_data = self.fresh();
                let out_len = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_data} = alloca ptr, align 8").ok();
                writeln!(self.body, "  {out_len} = alloca i64, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {out_data}").ok();
                writeln!(self.body, "  store i64 0, ptr {out_len}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {p}, ptr {out_data}, ptr {out_len})",
                    HOST_WS_ENCODE_TEXT.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                self.store_dynbytes_from_out(local, &out_data, &out_len)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "wsEncodeBinary") =>
            {
                let (d, n) = self.emit_bytes_ptr_len(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: bin payload"))?,
                )?;
                let out_data = self.fresh();
                let out_len = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_data} = alloca ptr, align 8").ok();
                writeln!(self.body, "  {out_len} = alloca i64, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {out_data}").ok();
                writeln!(self.body, "  store i64 0, ptr {out_len}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {d}, i64 {n}, ptr {out_data}, ptr {out_len})",
                    HOST_WS_ENCODE_BINARY.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                self.store_dynbytes_from_out(local, &out_data, &out_len)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "wsEncodeClose") =>
            {
                let code_f = self.emit_number_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: close code"))?,
                )?;
                let code_i = self.fresh();
                writeln!(self.body, "  {code_i} = fptosi double {code_f} to i32").ok();
                let reason = self.emit_string_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_ws_e2e: close reason"))?,
                )?;
                let out_data = self.fresh();
                let out_len = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_data} = alloca ptr, align 8").ok();
                writeln!(self.body, "  {out_len} = alloca i64, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {out_data}").ok();
                writeln!(self.body, "  store i64 0, ptr {out_len}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i32 {code_i}, ptr {reason}, ptr {out_data}, ptr {out_len})",
                    HOST_WS_ENCODE_CLOSE.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                self.store_dynbytes_from_out(local, &out_data, &out_len)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "wsEncodePing") =>
            {
                let p = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: ping payload"))?,
                )?;
                let out_data = self.fresh();
                let out_len = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_data} = alloca ptr, align 8").ok();
                writeln!(self.body, "  {out_len} = alloca i64, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {out_data}").ok();
                writeln!(self.body, "  store i64 0, ptr {out_len}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {p}, ptr {out_data}, ptr {out_len})",
                    HOST_WS_ENCODE_PING.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                self.store_dynbytes_from_out(local, &out_data, &out_len)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "wsEncodePong") =>
            {
                let p = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: pong payload"))?,
                )?;
                let out_data = self.fresh();
                let out_len = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_data} = alloca ptr, align 8").ok();
                writeln!(self.body, "  {out_len} = alloca i64, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {out_data}").ok();
                writeln!(self.body, "  store i64 0, ptr {out_len}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {p}, ptr {out_data}, ptr {out_len})",
                    HOST_WS_ENCODE_PONG.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                self.store_dynbytes_from_out(local, &out_data, &out_len)
            }
            _ => Err(diag("host_ws_e2e: expected dynbytes producer")),
        }
    }

    fn store_dynbytes_from_out(
        &mut self,
        local: LocalId,
        out_data: &str,
        out_len: &str,
    ) -> Result<(), Diagnostic> {
        let d = self.fresh();
        let n = self.fresh();
        writeln!(self.body, "  {d} = load ptr, ptr {out_data}").ok();
        writeln!(self.body, "  {n} = load i64, ptr {out_len}").ok();
        let dp = self.slot_ptr(local)?;
        let lp = self.slot_len_ptr(local)?;
        writeln!(self.body, "  store ptr {d}, ptr {dp}").ok();
        writeln!(self.body, "  store i64 {n}, ptr {lp}").ok();
        Ok(())
    }

    fn emit_frame_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "wsDecodeFrame") =>
            {
                let (d, n) = self.emit_bytes_ptr_len(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: decode data"))?,
                )?;
                let out_fin = self.fresh();
                let out_op = self.fresh();
                let out_pay = self.fresh();
                let out_plen = self.fresh();
                let out_cc = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_fin} = alloca i32, align 4").ok();
                writeln!(self.body, "  {out_op} = alloca i32, align 4").ok();
                writeln!(self.body, "  {out_pay} = alloca ptr, align 8").ok();
                writeln!(self.body, "  {out_plen} = alloca i64, align 8").ok();
                writeln!(self.body, "  {out_cc} = alloca i32, align 4").ok();
                writeln!(self.body, "  store i32 0, ptr {out_fin}").ok();
                writeln!(self.body, "  store i32 0, ptr {out_op}").ok();
                writeln!(self.body, "  store ptr null, ptr {out_pay}").ok();
                writeln!(self.body, "  store i64 0, ptr {out_plen}").ok();
                writeln!(self.body, "  store i32 -1, ptr {out_cc}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {d}, i64 {n}, ptr {out_fin}, ptr {out_op}, ptr {out_pay}, ptr {out_plen}, ptr {out_cc})",
                    HOST_WS_DECODE_FRAME.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                for (field, tmp) in [
                    ("fin", &out_fin),
                    ("opcode", &out_op),
                    ("close_code", &out_cc),
                ] {
                    let v = self.fresh();
                    let dest = self.slot_frame_field(local, field)?;
                    writeln!(self.body, "  {v} = load i32, ptr {tmp}").ok();
                    writeln!(self.body, "  store i32 {v}, ptr {dest}").ok();
                }
                let pd = self.fresh();
                let pn = self.fresh();
                writeln!(self.body, "  {pd} = load ptr, ptr {out_pay}").ok();
                writeln!(self.body, "  {pn} = load i64, ptr {out_plen}").ok();
                let pp = self.slot_frame_field(local, "payload")?;
                let pl = self.slot_frame_field(local, "payload_len")?;
                writeln!(self.body, "  store ptr {pd}, ptr {pp}").ok();
                writeln!(self.body, "  store i64 {pn}, ptr {pl}").ok();
                Ok(())
            }
            _ => Err(diag("host_ws_e2e: expected wsDecodeFrame")),
        }
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
                    let n = self.fresh();
                    writeln!(self.body, "  {n} = call i64 @strlen(ptr {s})").ok();
                    Ok((s, n))
                }
                _ => Err(diag("host_ws_e2e: bytes arg unsupported")),
            },
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let prop = string_lit(property).ok_or_else(|| diag("host_ws_e2e: member prop"))?;
                let id = match object.as_ref() {
                    Expr::Local { id, .. } => *id,
                    _ => return Err(diag("host_ws_e2e: member object must be local")),
                };
                match (self.slot_of.get(&id), prop.as_str()) {
                    (Some(SlotTy::WsFrame), "payload") => {
                        let s = self.emit_frame_payload_cstr(id)?;
                        let n = self.fresh();
                        writeln!(self.body, "  {n} = call i64 @strlen(ptr {s})").ok();
                        Ok((s, n))
                    }
                    _ => Err(diag("host_ws_e2e: bytes member unsupported")),
                }
            }
            _ => Err(diag("host_ws_e2e: bytes arg unsupported")),
        }
    }

    fn emit_frame_payload_cstr(&mut self, id: LocalId) -> Result<String, Diagnostic> {
        let pp = self.slot_frame_field(id, "payload")?;
        let pl = self.slot_frame_field(id, "payload_len")?;
        let d = self.fresh();
        let n = self.fresh();
        writeln!(self.body, "  {d} = load ptr, ptr {pp}").ok();
        writeln!(self.body, "  {n} = load i64, ptr {pl}").ok();
        let buf = self.fresh();
        let np1 = self.fresh();
        writeln!(self.body, "  {np1} = add i64 {n}, 1").ok();
        writeln!(self.body, "  {buf} = call ptr @malloc(i64 {np1})").ok();
        let copy_ok = format!("wse2e_pay_copy_{}", self.next_tmp);
        let copy_skip = format!("wse2e_pay_skip_{}", self.next_tmp);
        let copy_done = format!("wse2e_pay_done_{}", self.next_tmp);
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

    fn emit_handle_i64(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        let f = self.emit_number_expr(expr)?;
        let i = self.fresh();
        writeln!(self.body, "  {i} = fptosi double {f} to i64").ok();
        Ok(i)
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let t = self.fresh();
                let lit = if raw.contains('.') || raw.contains('e') || raw.contains('E') {
                    raw.clone()
                } else {
                    format!("{raw}.0")
                };
                writeln!(self.body, "  {t} = fadd double {lit}, 0.0").ok();
                Ok(t)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            Expr::Call { callee, args, .. }
                if (args.len() == 1 || args.len() == 2) && is_named_callee(callee, "tcpListen") =>
            {
                let port_f = self.emit_number_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: tcpListen port"))?,
                )?;
                let port_i = self.fresh();
                writeln!(self.body, "  {port_i} = fptosi double {port_f} to i32").ok();
                let backlog_i = if args.len() == 2 {
                    let bf = self.emit_number_expr(
                        arg_expr(&args[1]).ok_or_else(|| diag("host_ws_e2e: tcpListen backlog"))?,
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
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: tcpAccept"))?,
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
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: tcpConnect host"))?,
                )?;
                let port_f = self.emit_number_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_ws_e2e: tcpConnect port"))?,
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
                if args.len() == 1 && is_named_callee(callee, "tcpLocalPort") =>
            {
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: tcpLocalPort"))?,
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
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let prop = string_lit(property).ok_or_else(|| diag("host_ws_e2e: member prop"))?;
                let id = match object.as_ref() {
                    Expr::Local { id, .. } => *id,
                    _ => return Err(diag("host_ws_e2e: member object must be local")),
                };
                match (self.slot_of.get(&id), prop.as_str()) {
                    (Some(SlotTy::DynBytes), "length") => {
                        let lp = self.slot_len_ptr(id)?;
                        let i = self.fresh();
                        let d = self.fresh();
                        writeln!(self.body, "  {i} = load i64, ptr {lp}").ok();
                        writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                        Ok(d)
                    }
                    (Some(SlotTy::WsFrame), "fin" | "opcode" | "closeCode") => {
                        let field = if prop == "closeCode" {
                            "close_code"
                        } else {
                            prop.as_str()
                        };
                        let fp = self.slot_frame_field(id, field)?;
                        let i = self.fresh();
                        let d = self.fresh();
                        writeln!(self.body, "  {i} = load i32, ptr {fp}").ok();
                        writeln!(self.body, "  {d} = sitofp i32 {i} to double").ok();
                        Ok(d)
                    }
                    _ => Err(diag("host_ws_e2e: unsupported number member")),
                }
            }
            _ => Err(diag("host_ws_e2e: unsupported number expr")),
        }
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
                if args.len() == 3 && is_named_callee(callee, "wsClientHandshakeRequest") =>
            {
                let path = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: hs path"))?,
                )?;
                let host = self.emit_string_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_ws_e2e: hs host"))?,
                )?;
                let key = self.emit_string_expr(
                    arg_expr(&args[2]).ok_or_else(|| diag("host_ws_e2e: hs key"))?,
                )?;
                let out = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out} = alloca ptr, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {out}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {path}, ptr {host}, ptr {key}, ptr {out})",
                    HOST_WS_CLIENT_HANDSHAKE_REQUEST.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {out}").ok();
                Ok(v)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "wsHandshakeResponse") =>
            {
                let key = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws_e2e: ws key"))?,
                )?;
                let out = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out} = alloca ptr, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {out}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {key}, ptr {out})",
                    HOST_WS_HANDSHAKE_RESPONSE.symbol
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
                computed: false,
                ..
            } => {
                let prop = string_lit(property).ok_or_else(|| diag("host_ws_e2e: member prop"))?;
                let id = match object.as_ref() {
                    Expr::Local { id, .. } => *id,
                    _ => return Err(diag("host_ws_e2e: member object must be local")),
                };
                match (self.slot_of.get(&id), prop.as_str()) {
                    (Some(SlotTy::WsFrame), "payload") => self.emit_frame_payload_cstr(id),
                    _ => Err(diag("host_ws_e2e: unsupported string member")),
                }
            }
            _ => Err(diag("host_ws_e2e: unsupported string expr")),
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
    fn emit_ws_client_echo_ir() {
        let m = lower_src(
            r#"
            let key = "dGhlIHNhbXBsZSBub25jZQ==";
            let s = tcpListen(0);
            let c = tcpConnect("127.0.0.1", tcpLocalPort(s));
            let a = tcpAccept(s);
            let req = wsClientHandshakeRequest("/echo", "127.0.0.1", key);
            tcpWrite(c, req);
            let raw = tcpRead(a, 4096);
            let resp = wsHandshakeResponse(key);
            tcpWrite(a, resp);
            let out = tcpRead(c, 4096);
            wsClientCheckAccept(out, key);
            let f = wsEncodeTextClient("hello");
            tcpWrite(c, f);
            let rawF = tcpRead(a, 4096);
            let d = wsDecodeFrame(rawF);
            let echo = wsEncodeText(d.payload);
            tcpWrite(a, echo);
            let got = tcpRead(c, 4096);
            let d2 = wsDecodeFrame(got);
            stdoutWrite(d2.payload);
            closeTcp(a);
            closeTcp(c);
            closeTcp(s);
            "#,
        );
        assert!(is_host_ws_e2e_module(&m));
        let ir = emit_host_ws_e2e(&m).expect("emit");
        assert!(
            ir.contains("draconic_rt_host_ws_client_handshake_request"),
            "{ir}"
        );
        assert!(
            ir.contains("draconic_rt_host_ws_client_check_accept"),
            "{ir}"
        );
        assert!(
            ir.contains("draconic_rt_host_ws_encode_text_client"),
            "{ir}"
        );
        assert!(ir.contains("draconic_rt_host_ws_decode_frame"), "{ir}");
    }

    #[test]
    fn emit_ws_client_check_accept_only() {
        let m = lower_src(
            r#"
            let key = "dGhlIHNhbXBsZSBub25jZQ==";
            wsClientCheckAccept("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: bad\r\n\r\n", key);
            "#,
        );
        assert!(is_host_ws_e2e_module(&m));
        let ir = emit_host_ws_e2e(&m).expect("emit");
        assert!(
            ir.contains("draconic_rt_host_ws_client_check_accept"),
            "{ir}"
        );
    }
}
