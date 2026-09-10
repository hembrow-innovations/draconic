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

mod classify;
mod emit;
mod values;

use classify::classify;

pub(crate) fn is_host_tcp_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_tcp(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_tcp module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

pub(crate) fn walk_host_tcp(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_host_tcp_module(module) {
        return None;
    }
    Some(emit_host_tcp(module))
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
