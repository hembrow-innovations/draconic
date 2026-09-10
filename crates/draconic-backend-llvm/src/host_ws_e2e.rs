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

mod classify;
mod emit;

use classify::classify;

pub(crate) fn is_host_ws_e2e_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_ws_e2e(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_ws_e2e module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

pub(crate) fn walk_host_ws_e2e(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_host_ws_e2e_module(module) {
        return None;
    }
    Some(emit_host_ws_e2e(module))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LocalSlot {
    Handle,
    Number,
    String,
    DynBytes,
    WsFrame,
}

struct ModuleInfo {
    slots: Vec<(LocalId, LocalSlot)>,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, LocalSlot)>,
    slot_of: HashMap<LocalId, LocalSlot>,
    has_tcp: bool,
    has_ws_client: bool,
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

struct Emitter<'a> {
    module: &'a Module,
    info: &'a ModuleInfo,
    out: String,
    body: String,
    next_tmp: usize,
    str_globals: Vec<(String, String)>,
    local_name: HashMap<LocalId, String>,
    slot_of: HashMap<LocalId, LocalSlot>,
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
