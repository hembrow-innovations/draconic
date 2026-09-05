//! H06.01–H06.05 + H11.01/H11.02: native TCP + TLS client/server wrap.
//!
//! - `tcpListen(port)` / `tcpListen(port, backlog)` → listen handle (number)
//! - `tcpLocalPort(h)` → bound port (ephemeral when listen port was 0)
//! - `tcpAccept(listen)` → connection handle
//! - `tcpConnect(host, port)` → connection handle (IPv4 dotted or DNS name; H09.02)
//! - `tcpPeerAddress(conn)` → peer IPv4 string
//! - `tcpPeerPort(conn)` → peer port number
//! - `tcpWrite(conn, data)` → write string/bytes (all bytes)
//! - `tcpRead(conn, maxLen)` → DynBytes; `.length` + `stdoutWrite`
//! - `tcpShutdown(conn)` / `tcpShutdown(conn, how)` — how 0=RD 1=WR 2=RDWR (default WR)
//! - `closeTcp(h)` → close listen/conn handle via Runtime handle_close
//! - `tlsClientWrap(conn, serverName, insecure)` → TLS handle (takes TCP conn)
//! - `tlsServerWrap(conn, certPath, keyPath)` → TLS handle (PEM cert+key; takes TCP conn)
//! - `tlsRead` / `tlsWrite` / `closeTls` — application data + close TLS+TCP
//!
//! Host errors: `E_CONN` (refused/reset/timeout) → stderr `ECONN` + exit 1;
//! `E_ADDR` (DNS resolve failure on connect-by-name) → stderr `EADDR` + exit 1;
//! `E_PERM` (grant deny, R02.02) → stderr `EPERM` + exit 1;
//! other non-OK → `EIO` + exit 1.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_HANDLE_CLOSE, HOST_PROCESS_EXIT, HOST_STDERR_WRITE,
    HOST_STDOUT_WRITE, HOST_TCP_ACCEPT, HOST_TCP_CONNECT, HOST_TCP_LISTEN, HOST_TCP_LOCAL_PORT,
    HOST_TCP_PEER_ADDRESS, HOST_TCP_PEER_PORT, HOST_TCP_READ, HOST_TCP_SHUTDOWN, HOST_TCP_WRITE,
    HOST_TLS_CLIENT_WRAP, HOST_TLS_READ, HOST_TLS_SERVER_WRAP, HOST_TLS_WRITE, PRINT_BOOL,
    PRINT_F64, PRINT_STR,
};

pub(crate) fn is_host_tcp_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_tcp(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_tcp module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Handle,
    Number,
    Bool,
    String,
    DynBytes,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    print_locals: Vec<(LocalId, SlotTy)>,
    has_tcp: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_locals: Vec::new(),
        has_tcp: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_tcp {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_locals: ctx.print_locals,
    })
}

fn is_dynbytes_length(expr: &Expr, ctx: &ClassifyCtx) -> bool {
    match expr {
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let name = match string_lit(property) {
                Some(n) => n,
                None => return false,
            };
            if name != "length" {
                return false;
            }
            match object.as_ref() {
                Expr::Local { id, .. } => ctx.slot_of.get(id) == Some(&SlotTy::DynBytes),
                _ => false,
            }
        }
        _ => false,
    }
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            // Auto-print bools/strings always; numbers only from DynBytes.length
            // (ports etc. must not pollute native.stdout).
            if matches!(ty, SlotTy::Bool | SlotTy::String)
                || (ty == SlotTy::Number && is_dynbytes_length(init, ctx))
            {
                ctx.print_locals.push((*local, ty));
            }
            Some(())
        }
        Stmt::Expr { expr, .. } => {
            classify_side_effect(expr, ctx)?;
            Some(())
        }
        _ => None,
    }
}

fn classify_side_effect(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1
                && (is_named_callee(callee, "closeTcp") || is_named_callee(callee, "closeTls")) =>
        {
            ctx.has_tcp = true;
            classify_expr(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2
                && (is_named_callee(callee, "tcpWrite") || is_named_callee(callee, "tlsWrite")) =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            let dt = classify_expr(arg_expr(&args[1])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            if !matches!(dt, SlotTy::String | SlotTy::DynBytes) {
                return None;
            }
            Some(())
        }
        Expr::Call { callee, args, .. }
            if (args.len() == 1 || args.len() == 2) && is_named_callee(callee, "tcpShutdown") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            if args.len() == 2 {
                let ht2 = classify_expr(arg_expr(&args[1])?, ctx)?;
                if ht2 != SlotTy::Number {
                    return None;
                }
            }
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "stdoutWrite") =>
        {
            let t = classify_expr(arg_expr(&args[0])?, ctx)?;
            if matches!(t, SlotTy::String | SlotTy::DynBytes) {
                Some(())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 3 && is_named_callee(callee, "tlsClientWrap") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            let nt = classify_expr(arg_expr(&args[1])?, ctx)?;
            if nt != SlotTy::String {
                return None;
            }
            let it = classify_expr(arg_expr(&args[2])?, ctx)?;
            if it != SlotTy::Number {
                return None;
            }
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 3 && is_named_callee(callee, "tlsServerWrap") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            let ct = classify_expr(arg_expr(&args[1])?, ctx)?;
            if ct != SlotTy::String {
                return None;
            }
            let kt = classify_expr(arg_expr(&args[2])?, ctx)?;
            if kt != SlotTy::String {
                return None;
            }
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tlsRead") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            let mt = classify_expr(arg_expr(&args[1])?, ctx)?;
            if mt != SlotTy::Number {
                return None;
            }
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if (args.len() == 1 || args.len() == 2) && is_named_callee(callee, "tcpListen") =>
        {
            ctx.has_tcp = true;
            classify_expr(arg_expr(&args[0])?, ctx)?;
            if args.len() == 2 {
                classify_expr(arg_expr(&args[1])?, ctx)?;
            }
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpLocalPort") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpAccept") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpConnect") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            let pt = classify_expr(arg_expr(&args[1])?, ctx)?;
            if ht != SlotTy::String || pt != SlotTy::Number {
                return None;
            }
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpPeerAddress") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpPeerPort") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpRead") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            let mt = classify_expr(arg_expr(&args[1])?, ctx)?;
            if ht != SlotTy::Handle || mt != SlotTy::Number {
                return None;
            }
            Some(SlotTy::DynBytes)
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
                _ => None,
            }
        }
        Expr::Binary {
            op:
                BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::EqEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEq
                | BinaryOp::NotEqEq,
            left,
            right,
            ..
        } => {
            let lt = classify_expr(left, ctx)?;
            let rt = classify_expr(right, ctx)?;
            if matches!(lt, SlotTy::Number | SlotTy::Handle)
                && matches!(rt, SlotTy::Number | SlotTy::Handle)
            {
                Some(SlotTy::Bool)
            } else {
                None
            }
        }
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            let _ = classify_expr(arg, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::String { .. } => Some(SlotTy::String),
        _ => None,
    }
}

fn string_lit(expr: &Expr) -> Option<String> {
    match expr {
        Expr::String { value, .. } => Some(value.to_string_lossy().to_string()),
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
    next_label: usize,
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
            next_label: 0,
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
        let n = self.next_label;
        self.next_label += 1;
        format!("{prefix}{n}")
    }

    fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_tcp: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn slot_len_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_tcp: unknown local"))?;
        Ok(format!("%slot_{name}_len"))
    }

    fn intern_cstr(&mut self, s: &str) -> String {
        if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            return g.clone();
        }
        let g = format!(".str.tcp.{}", self.str_globals.len());
        self.str_globals.push((s.to_string(), g.clone()));
        g
    }

    fn emit_cstr_ptr(&mut self, s: &str) -> String {
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

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
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

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
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

    fn emit_dynbytes_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
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

    fn emit_expr_stmt(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
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
                _ => Err(diag("host_tcp: bytes arg unsupported")),
            },
            _ => Err(diag("host_tcp: bytes arg unsupported")),
        }
    }

    fn emit_cstr_len(&mut self, s: &str) -> Result<String, Diagnostic> {
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

    fn emit_stdout_write(&mut self, arg: &Expr) -> Result<(), Diagnostic> {
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

    fn emit_handle_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
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
                    arg_expr(&args[0]).ok_or_else(|| diag("host_tcp: tcpLocalPort handle"))?,
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
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "tcpPeerPort") =>
            {
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_tcp: tcpPeerPort handle"))?,
                )?;
                let out_p = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_p} = alloca i32, align 4").ok();
                writeln!(self.body, "  store i32 0, ptr {out_p}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i64 {h}, ptr {out_p})",
                    HOST_TCP_PEER_PORT.symbol
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
                let name = string_lit(property).ok_or_else(|| diag("host_tcp: bad prop"))?;
                match object.as_ref() {
                    Expr::Local { id, .. }
                        if name == "length" && self.slot_of.get(id) == Some(&SlotTy::DynBytes) =>
                    {
                        let lp = self.slot_len_ptr(*id)?;
                        let n = self.fresh();
                        let f = self.fresh();
                        writeln!(self.body, "  {n} = load i64, ptr {lp}").ok();
                        writeln!(self.body, "  {f} = sitofp i64 {n} to double").ok();
                        Ok(f)
                    }
                    _ => Err(diag("host_tcp: unsupported member number")),
                }
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_tcp: expected number expr")),
        }
    }

    fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Binary {
                op, left, right, ..
            } if matches!(
                op,
                BinaryOp::Gt
                    | BinaryOp::GtEq
                    | BinaryOp::Lt
                    | BinaryOp::LtEq
                    | BinaryOp::EqEq
                    | BinaryOp::EqEqEq
                    | BinaryOp::NotEq
                    | BinaryOp::NotEqEq
            ) =>
            {
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let cmp = self.fresh();
                let pred = match op {
                    BinaryOp::Gt => "ogt",
                    BinaryOp::GtEq => "oge",
                    BinaryOp::Lt => "olt",
                    BinaryOp::LtEq => "ole",
                    BinaryOp::EqEq | BinaryOp::EqEqEq => "oeq",
                    BinaryOp::NotEq | BinaryOp::NotEqEq => "one",
                    _ => unreachable!(),
                };
                writeln!(self.body, "  {cmp} = fcmp {pred} double {l}, {r}").ok();
                let b = self.fresh();
                writeln!(self.body, "  {b} = zext i1 {cmp} to i8").ok();
                Ok(b)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_tcp: expected bool expr")),
        }
    }

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => Ok(self.emit_cstr_ptr(&value.to_string_lossy())),
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "tcpPeerAddress") =>
            {
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_tcp: tcpPeerAddress handle"))?,
                )?;
                let out_p = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_p} = alloca ptr, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {out_p}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i64 {h}, ptr {out_p})",
                    HOST_TCP_PEER_ADDRESS.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {out_p}").ok();
                Ok(v)
            }
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } => self.emit_typeof(arg),
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_tcp: expected string expr")),
        }
    }

    fn emit_typeof(&mut self, arg: &Expr) -> Result<String, Diagnostic> {
        match arg {
            Expr::Local { id, .. } => {
                let ty = self
                    .slot_of
                    .get(id)
                    .copied()
                    .ok_or_else(|| diag("host_tcp: typeof unknown local"))?;
                let s = match ty {
                    SlotTy::Handle | SlotTy::Number => "number",
                    SlotTy::Bool => "boolean",
                    SlotTy::String => "string",
                    SlotTy::DynBytes => "object",
                };
                Ok(self.emit_cstr_ptr(s))
            }
            _ => Err(diag("host_tcp: typeof unsupported arg")),
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
    fn emit_tcp_listen_ephemeral() {
        let m = lower_src(
            r#"
            let s = tcpListen(0);
            let p = tcpLocalPort(s);
            let ok = p > 0;
            closeTcp(s);
            "#,
        );
        assert!(is_host_tcp_module(&m));
        let ir = emit_host_tcp(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_tcp_listen"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tcp_local_port"), "{ir}");
        assert!(ir.contains("draconic_rt_host_handle_close"), "{ir}");
    }

    #[test]
    fn emit_tcp_accept_peer() {
        let m = lower_src(
            r#"
            let s = tcpListen(0);
            let p = tcpLocalPort(s);
            let c = tcpConnect("127.0.0.1", p);
            let a = tcpAccept(s);
            let peer = tcpPeerAddress(a);
            let ok = tcpPeerPort(a) > 0;
            closeTcp(a);
            closeTcp(c);
            closeTcp(s);
            "#,
        );
        assert!(is_host_tcp_module(&m));
        let ir = emit_host_tcp(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_tcp_accept"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tcp_connect"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tcp_peer_address"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tcp_peer_port"), "{ir}");
    }

    #[test]
    fn emit_tcp_connect_maps_econn() {
        let m = lower_src(
            r#"
            let c = tcpConnect("127.0.0.1", 1);
            closeTcp(c);
            "#,
        );
        assert!(is_host_tcp_module(&m));
        let ir = emit_host_tcp(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_tcp_connect"), "{ir}");
        assert!(
            ir.contains("ECONN\\0A") || ir.contains("ECONN\\n") || ir.contains("c\"ECONN"),
            "{ir}"
        );
        assert!(ir.contains("icmp eq i32") && ir.contains(", 10"), "{ir}");
    }

    #[test]
    fn emit_tcp_connect_maps_eaddr() {
        let m = lower_src(
            r#"
            let c = tcpConnect("localhost", 1);
            closeTcp(c);
            "#,
        );
        assert!(is_host_tcp_module(&m));
        let ir = emit_host_tcp(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_tcp_connect"), "{ir}");
        assert!(
            ir.contains("EADDR\\0A") || ir.contains("EADDR\\n") || ir.contains("c\"EADDR"),
            "{ir}"
        );
        assert!(ir.contains("icmp eq i32") && ir.contains(", 11"), "{ir}");
    }

    #[test]
    fn emit_tcp_read_write_shutdown() {
        let m = lower_src(
            r#"
            let s = tcpListen(0);
            let p = tcpLocalPort(s);
            let c = tcpConnect("127.0.0.1", p);
            let a = tcpAccept(s);
            tcpWrite(c, "hello-tcp");
            let u = tcpRead(a, 64);
            let n = u.length;
            stdoutWrite(u);
            tcpShutdown(c);
            let eof = tcpRead(a, 64);
            let en = eof.length;
            closeTcp(a);
            closeTcp(c);
            closeTcp(s);
            "#,
        );
        assert!(is_host_tcp_module(&m));
        let ir = emit_host_tcp(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_tcp_write"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tcp_read"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tcp_shutdown"), "{ir}");
        assert!(ir.contains("draconic_rt_host_stdout_write"), "{ir}");
    }

    #[test]
    fn emit_tcp_loopback_echo() {
        let m = lower_src(
            r#"
            let s = tcpListen(0);
            let c = tcpConnect("127.0.0.1", tcpLocalPort(s));
            let a = tcpAccept(s);
            tcpWrite(c, "echo-me");
            let req = tcpRead(a, 64);
            tcpWrite(a, req);
            let res = tcpRead(c, 64);
            stdoutWrite(res);
            let n = res.length;
            closeTcp(a);
            closeTcp(c);
            closeTcp(s);
            "#,
        );
        assert!(is_host_tcp_module(&m));
        let ir = emit_host_tcp(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_tcp_listen"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tcp_connect"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tcp_accept"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tcp_write"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tcp_read"), "{ir}");
        assert!(ir.contains("draconic_rt_host_stdout_write"), "{ir}");
        assert!(ir.contains("draconic_rt_host_handle_close"), "{ir}");
    }

    #[test]
    fn emit_tls_client_wrap_read_write() {
        let m = lower_src(
            r#"
            let c = tcpConnect("127.0.0.1", 443);
            let t = tlsClientWrap(c, "localhost", 1);
            tlsWrite(t, "hi");
            let res = tlsRead(t, 64);
            stdoutWrite(res);
            closeTls(t);
            "#,
        );
        assert!(is_host_tcp_module(&m));
        let ir = emit_host_tcp(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_tls_client_wrap"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tls_write"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tls_read"), "{ir}");
        assert!(ir.contains("draconic_rt_host_handle_close"), "{ir}");
    }

    #[test]
    fn emit_tls_server_wrap() {
        let m = lower_src(
            r#"
            let s = tcpListen(0);
            let a = tcpAccept(s);
            let t = tlsServerWrap(a, "/tmp/cert.pem", "/tmp/key.pem");
            closeTls(t);
            closeTcp(s);
            "#,
        );
        assert!(is_host_tcp_module(&m));
        let ir = emit_host_tcp(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_tls_server_wrap"), "{ir}");
        assert!(ir.contains("draconic_rt_host_handle_close"), "{ir}");
    }
}
