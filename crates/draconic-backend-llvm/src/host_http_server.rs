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
//!
//! P04: `readFileText` + string `+` + linked one-arg string functions (`greet`)
//! so a native HTTP server can read fs config and a git module export.

use std::collections::HashMap;

use crate::host_catalog::is_named_callee;
use draconic_ast::BinaryOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Pattern, Stmt};
use draconic_runtime::abi::{
    llvm_declares, CSTR_CONCAT, GC_INIT, HOST_FS_READ_TEXT, HOST_HANDLE_CLOSE,
    HOST_HTTP_PARSE_REQUEST, HOST_HTTP_PARSE_RESPONSE, HOST_HTTP_RESPONSE_HEADER,
    HOST_HTTP_SERVE_STATIC, HOST_HTTP_WRITE_REQUEST, HOST_HTTP_WRITE_RESPONSE, HOST_PROCESS_EXIT,
    HOST_STDERR_WRITE, HOST_STDOUT_WRITE, HOST_TCP_ACCEPT, HOST_TCP_CONNECT, HOST_TCP_LISTEN,
    HOST_TCP_LOCAL_PORT, HOST_TCP_READ, HOST_TCP_WRITE, HOST_TLS_CLIENT_WRAP, HOST_TLS_READ,
    HOST_TLS_SERVER_WRAP, HOST_TLS_WRITE, HOST_WS_HANDSHAKE_RESPONSE, PRINT_I64, PRINT_STR,
};

mod classify;
mod emit;
mod io;

use classify::classify;

pub(crate) fn is_host_http_server_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_http_server(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_http_server module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

pub(crate) fn walk_host_http_server(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_host_http_server_module(module) {
        return None;
    }
    Some(emit_host_http_server(module))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LocalSlot {
    Handle,
    Number,
    String,
    DynBytes,
    HttpReq,
    HttpRes,
}

struct ModuleInfo {
    slots: Vec<(LocalId, LocalSlot)>,
    print_locals: Vec<(LocalId, LocalSlot)>,
    /// H10.05 client observations: auto-print string/number locals at end.
    client_print: bool,
    /// P04: one-arg string functions (linked `greet`) → (param, return expr).
    string_fns: HashMap<LocalId, (LocalId, Expr)>,
    fn_names: HashMap<String, LocalId>,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, LocalSlot)>,
    slot_of: HashMap<LocalId, LocalSlot>,
    print_locals: Vec<(LocalId, LocalSlot)>,
    has_tcp: bool,
    has_http: bool,
    has_client: bool,
    string_fns: HashMap<LocalId, (LocalId, Expr)>,
    fn_names: HashMap<String, LocalId>,
    local_name: HashMap<LocalId, String>,
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
    fn emit_todo_static_serve_accept_loop_ir() {
        // H17.03 shape: listen + while(true) accept/httpServeStatic/close.
        let m = lower_src(
            r#"
            let s = tcpListen(18083);
            stdoutWrite("Draconic todo server listening on http://127.0.0.1:18083\n");
            while (true) {
              let a = tcpAccept(s);
              httpServeStatic(a, "./public");
              closeTcp(a);
            }
            "#,
        );
        assert!(is_host_http_server_module(&m));
        let ir = emit_host_http_server(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_tcp_listen"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tcp_accept"), "{ir}");
        assert!(ir.contains("draconic_rt_host_http_serve_static"), "{ir}");
        assert!(ir.contains("hs_while_head"), "{ir}");
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

    #[test]
    fn emit_flagship_http_fs_git_shape_ir() {
        // P04 shape: greet + readFileText + string concat + accept loop.
        let m = lower_src(
            r#"
            function greet(name) { return "hello, " + name; }
            let name = readFileText("config.txt");
            let banner = greet(name) + " " + "0.1.0";
            let s = tcpListen(18084);
            stdoutWrite("flagship-service listening on 18084\n");
            while (true) {
              let a = tcpAccept(s);
              let raw = tcpRead(a, 65536);
              let req = httpParseRequest(raw);
              let path = req.path;
              let body = banner + " " + path;
              let resp = httpWriteResponse(200, "OK", "Content-Type: text/plain\r\n", body);
              tcpWrite(a, resp);
              closeTcp(a);
            }
            "#,
        );
        assert!(is_host_http_server_module(&m), "P04 shape should classify");
        let ir = emit_host_http_server(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_fs_read_text"), "{ir}");
        assert!(ir.contains("draconic_rt_cstr_concat"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tcp_listen"), "{ir}");
        assert!(ir.contains("draconic_rt_host_http_parse_request"), "{ir}");
        assert!(ir.contains("draconic_rt_host_http_write_response"), "{ir}");
        assert!(ir.contains("hello, "), "{ir}");
    }
}
