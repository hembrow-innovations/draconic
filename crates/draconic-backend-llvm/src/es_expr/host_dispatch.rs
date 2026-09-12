use draconic_diagnostics::Diagnostic;
use draconic_ir::Module;

use super::diag;
use super::walk::Seen;

pub(super) fn emit_host(module: &Module, seen: &Seen) -> Result<String, Diagnostic> {
    if seen.host_note_prefix("H15.03") {
        return crate::host_process_async::emit_host_process_async(module);
    }
    if seen.host_note_prefix("H15") {
        return crate::host_subprocess::emit_host_subprocess(module);
    }
    if seen.host_note_prefix("H14") {
        return crate::host_signals::emit_host_signals(module);
    }
    if seen.host_note_prefix("H02.03") {
        return crate::host_stdio::emit_host_stdio(module);
    }
    if seen.host_note_prefix("H03") {
        return crate::host_path::emit_host_path(module);
    }
    if seen.host_note_prefix("H07") {
        return crate::host_tcp_async::emit_host_tcp_async(module);
    }
    if seen.host_note_prefix("H08") {
        return crate::host_udp::emit_host_udp(module);
    }
    if seen.host_note_prefix("H09") {
        return crate::host_dns::emit_host_dns(module);
    }
    if seen.host_note_prefix("H12.03") {
        return crate::host_ws_e2e::emit_host_ws_e2e(module);
    }
    if seen.host_note_prefix("H13") {
        return crate::host_http2::emit_host_http2(module);
    }
    if seen.host_note_prefix("H17.03")
        || (seen.host_note(http_server_http_note) && seen.host_note(http_server_tcp_note))
    {
        return crate::host_http_server::emit_host_http_server(module);
    }
    if seen.host_note_prefix("H12.02") {
        return crate::host_ws::emit_host_ws(module);
    }
    if seen.host_note_prefix("H10") || seen.host_note_prefix("H12.01") {
        return crate::host_http::emit_host_http(module);
    }
    if seen.host_note_prefix("H06") || seen.host_note_prefix("H11") {
        return crate::host_tcp::emit_host_tcp(module);
    }
    if seen.host_note_prefix("H05.03") || seen.host_note_prefix("H05.04") {
        return crate::host_timers::emit_host_timers(module);
    }
    if seen.host_note_prefix("H05") || seen.has_date_now {
        return crate::host_time::emit_host_time(module);
    }
    if seen.host_note_prefix("C06") {
        return crate::host_atomics::emit_host_atomics(module);
    }
    if seen.host_note_prefix("C01.01") && seen.host_note_prefix("C02") {
        return crate::host_worker_channels::emit_host_worker_channels(module);
    }
    if seen.host_note_prefix("C03") {
        return crate::host_once::emit_host_once(module);
    }
    if seen.host_note_prefix("C05") {
        return crate::host_cancel::emit_host_cancel(module);
    }
    if seen.host_note_prefix("C01") {
        return crate::host_workers::emit_host_workers(module);
    }
    if seen.host_note_prefix("C02") {
        return crate::host_channels::emit_host_channels(module);
    }
    if seen.host_note_prefix("H01") {
        return crate::host_process::emit_host_process(module);
    }
    if seen.host_note_prefix("H16") {
        return crate::host_os::emit_host_os(module);
    }
    if seen.host_only_note_prefix(&["H02.01", "H02.02"]) {
        return crate::host_stdio::emit_host_stdio(module);
    }
    if seen.host_note_prefix("H04") {
        if seen.has_script {
            return crate::host_docs::emit_host_docs(module);
        }
        return crate::host_fs::emit_host_fs(module);
    }
    Err(diag("unsupported IR node"))
}

fn http_server_http_note(note: &str) -> bool {
    note.contains("HTTP/1.1 request parse")
        || note.starts_with("H10.02")
        || note.contains("HTTP/1.1 request write")
        || note.contains("HTTP/1.1 response parse")
        || note.starts_with("H12.01")
}

fn http_server_tcp_note(note: &str) -> bool {
    note.contains("H17.04 TCP listen")
        || note.contains("TCP accept")
        || note.contains("TCP connect")
        || note.contains("TCP read")
        || note.contains("TCP write")
        || note.contains("TLS client wrap")
        || note.contains("TLS server wrap")
}
