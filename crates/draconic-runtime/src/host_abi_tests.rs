//! H00.02: Host Runtime ABI scaffold — error codes, handles, path boundary.

use super::*;
use std::process::Command;

#[test]
fn host_error_codes_stable_in_abi_and_header() {
    assert_eq!(HOST_OK, 0);
    assert_eq!(HOST_E_INVAL, 1);
    assert_eq!(HOST_E_NOENT, 2);
    assert_eq!(HOST_E_NOSYS, 3);
    assert_eq!(HOST_E_BADF, 4);
    assert_eq!(HOST_E_EXIST, 5);
    assert_eq!(HOST_E_PERM, 6);
    assert_eq!(HOST_E_IO, 7);
    assert_eq!(HOST_E_NOMEM, 8);
    assert_eq!(HOST_E_AGAIN, 9);
    assert_eq!(HOST_E_CONN, 10);
    assert_eq!(HOST_E_ADDR, 11);
    assert_eq!(HOST_HANDLE_INVALID, -1);

    let host_hdr = c_host_runtime_header_source();
    let main_hdr = c_runtime_header_source();
    assert!(
        main_hdr.contains("draconic_rt_host.h"),
        "main runtime header must include host substrate header"
    );
    for name in [
        "DRACONIC_HOST_OK",
        "DRACONIC_HOST_E_INVAL",
        "DRACONIC_HOST_E_NOENT",
        "DRACONIC_HOST_E_NOSYS",
        "DRACONIC_HOST_E_BADF",
        "DRACONIC_HOST_E_EXIST",
        "DRACONIC_HOST_E_PERM",
        "DRACONIC_HOST_E_IO",
        "DRACONIC_HOST_E_NOMEM",
        "DRACONIC_HOST_E_AGAIN",
        "DRACONIC_HOST_E_CONN",
        "DRACONIC_HOST_E_ADDR",
        "DRACONIC_HOST_HANDLE_INVALID",
        "DraconicHostError",
        "DraconicHostHandle",
    ] {
        assert!(
            host_hdr.contains(name),
            "host header must define/declare {name}"
        );
    }
}

#[test]
fn host_symbols_present_in_source_header_and_abi() {
    let src = c_host_runtime_source();
    let host_hdr = c_host_runtime_header_source();
    for sym in HOST_SYMBOLS {
        assert!(src.contains(sym), "host C source must define {sym}");
        assert!(host_hdr.contains(sym), "host header must declare {sym}");
    }
    assert!(
        c_host_runtime_path().is_file(),
        "draconic_rt_host.c must exist on disk"
    );
    assert!(
        c_host_runtime_header_path().is_file(),
        "draconic_rt_host.h must exist on disk"
    );
    assert!(HOST_SYMBOLS.contains(&HOST_HANDLE_IS_VALID_SYMBOL));
    assert!(HOST_SYMBOLS.contains(&HOST_HANDLE_CLOSE_SYMBOL));
    assert!(HOST_SYMBOLS.contains(&HOST_PATH_FROM_UTF8_SYMBOL));
    assert!(HOST_SYMBOLS.contains(&HOST_PATH_FREE_SYMBOL));
}

#[test]
fn host_abi_fn_shapes() {
    assert_eq!(
        HOST_HANDLE_IS_VALID.declare(),
        "declare i32 @draconic_rt_host_handle_is_valid(i64)"
    );
    assert_eq!(
        HOST_HANDLE_CLOSE.declare(),
        "declare i32 @draconic_rt_host_handle_close(i64)"
    );
    assert_eq!(
        HOST_PATH_FROM_UTF8.declare(),
        "declare i32 @draconic_rt_host_path_from_utf8(ptr, i64, ptr)"
    );
    assert_eq!(
        HOST_PATH_FREE.declare(),
        "declare void @draconic_rt_host_path_free(ptr)"
    );
    assert_eq!(
        HOST_TCP_LISTEN.declare(),
        "declare i32 @draconic_rt_host_tcp_listen(i32, i32, ptr)"
    );
    assert_eq!(
        HOST_TCP_LOCAL_PORT.declare(),
        "declare i32 @draconic_rt_host_tcp_local_port(i64, ptr)"
    );
    assert_eq!(
        HOST_TCP_ACCEPT.declare(),
        "declare i32 @draconic_rt_host_tcp_accept(i64, ptr)"
    );
    assert_eq!(
        HOST_TCP_CONNECT.declare(),
        "declare i32 @draconic_rt_host_tcp_connect(ptr, i32, ptr)"
    );
    assert_eq!(
        HOST_TCP_PEER_PORT.declare(),
        "declare i32 @draconic_rt_host_tcp_peer_port(i64, ptr)"
    );
    assert_eq!(
        HOST_TCP_PEER_ADDRESS.declare(),
        "declare i32 @draconic_rt_host_tcp_peer_address(i64, ptr)"
    );
    assert_eq!(
        HOST_TCP_READ.declare(),
        "declare i32 @draconic_rt_host_tcp_read(i64, i64, ptr, ptr)"
    );
    assert_eq!(
        HOST_TCP_WRITE.declare(),
        "declare i32 @draconic_rt_host_tcp_write(i64, ptr, i64)"
    );
    assert_eq!(
        HOST_TCP_SHUTDOWN.declare(),
        "declare i32 @draconic_rt_host_tcp_shutdown(i64, i32)"
    );
    assert_eq!(
        HOST_TCP_SET_NONBLOCKING.declare(),
        "declare i32 @draconic_rt_host_tcp_set_nonblocking(i64, i32)"
    );
    assert_eq!(
        HOST_IO_WAIT.declare(),
        "declare i32 @draconic_rt_host_io_wait(i64, i32, ptr, ptr, ptr)"
    );
    assert_eq!(
        HOST_IO_CANCEL.declare(),
        "declare void @draconic_rt_host_io_cancel(i64)"
    );
    assert_eq!(
        HOST_IO_PENDING.declare(),
        "declare i32 @draconic_rt_host_io_pending()"
    );
    assert_eq!(
        HOST_IO_POLL.declare(),
        "declare i32 @draconic_rt_host_io_poll(double)"
    );
    assert_eq!(
        HOST_TCP_ACCEPT_ASYNC.declare(),
        "declare ptr @draconic_rt_host_tcp_accept_async(i64)"
    );
    assert_eq!(
        HOST_TCP_CONNECT_ASYNC.declare(),
        "declare ptr @draconic_rt_host_tcp_connect_async(ptr, i32)"
    );
    assert_eq!(
        HOST_TCP_READ_ASYNC.declare(),
        "declare ptr @draconic_rt_host_tcp_read_async(i64, i64)"
    );
    assert_eq!(
        HOST_TCP_WRITE_ASYNC.declare(),
        "declare ptr @draconic_rt_host_tcp_write_async(i64, ptr, i64)"
    );
    assert_eq!(
        HOST_UDP_BIND.declare(),
        "declare i32 @draconic_rt_host_udp_bind(i32, ptr)"
    );
    assert_eq!(
        HOST_UDP_LOCAL_PORT.declare(),
        "declare i32 @draconic_rt_host_udp_local_port(i64, ptr)"
    );
    assert_eq!(
        HOST_UDP_SENDTO.declare(),
        "declare i32 @draconic_rt_host_udp_sendto(i64, ptr, i64, ptr, i32)"
    );
    assert_eq!(
        HOST_UDP_RECVFROM.declare(),
        "declare i32 @draconic_rt_host_udp_recvfrom(i64, i64, ptr, ptr, ptr, ptr)"
    );
    assert_eq!(
        HOST_DNS_LOOKUP.declare(),
        "declare i32 @draconic_rt_host_dns_lookup(ptr, ptr, ptr)"
    );
    assert_eq!(
        HOST_HTTP_PARSE_REQUEST.declare(),
        "declare i32 @draconic_rt_host_http_parse_request(ptr, i64, ptr, ptr, ptr, ptr)"
    );
    assert_eq!(
        HOST_HTTP_REQUEST_HEADER.declare(),
        "declare i32 @draconic_rt_host_http_request_header(ptr, i64, ptr, ptr)"
    );
    assert_eq!(
        HOST_HTTP_WRITE_RESPONSE.declare(),
        "declare i32 @draconic_rt_host_http_write_response(i32, ptr, ptr, ptr, i64, ptr)"
    );
    assert_eq!(
        HOST_HTTP_WRITE_REQUEST.declare(),
        "declare i32 @draconic_rt_host_http_write_request(ptr, ptr, ptr, ptr, i64, ptr)"
    );
    assert_eq!(
        HOST_HTTP_PARSE_RESPONSE.declare(),
        "declare i32 @draconic_rt_host_http_parse_response(ptr, i64, ptr, ptr, ptr, ptr)"
    );
    assert_eq!(
        HOST_HTTP_RESPONSE_HEADER.declare(),
        "declare i32 @draconic_rt_host_http_response_header(ptr, i64, ptr, ptr)"
    );
    assert_eq!(
        HOST_TLS_CLIENT_WRAP.declare(),
        "declare i32 @draconic_rt_host_tls_client_wrap(i64, ptr, i32, ptr)"
    );
    assert_eq!(
        HOST_TLS_SERVER_WRAP.declare(),
        "declare i32 @draconic_rt_host_tls_server_wrap(i64, ptr, ptr, ptr)"
    );
    assert_eq!(
        HOST_TLS_READ.declare(),
        "declare i32 @draconic_rt_host_tls_read(i64, i64, ptr, ptr)"
    );
    assert_eq!(
        HOST_TLS_WRITE.declare(),
        "declare i32 @draconic_rt_host_tls_write(i64, ptr, i64)"
    );
}

#[test]
fn static_lib_includes_host_object() {
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    // `nm` lists archive members; host symbols must be present after multi-file ar.
    let nm = Command::new("nm")
        .arg(&archive)
        .output()
        .expect("nm on archive");
    let out = String::from_utf8_lossy(&nm.stdout);
    let err = String::from_utf8_lossy(&nm.stderr);
    assert!(
        nm.status.success() || !out.is_empty(),
        "nm failed: status={:?} stderr={err}",
        nm.status
    );
    for sym in [
        "draconic_rt_host_handle_is_valid",
        "draconic_rt_host_handle_close",
        "draconic_rt_host_path_from_utf8",
        "draconic_rt_host_path_free",
        "draconic_rt_host_process_set_argv",
        "draconic_rt_host_process_user_argc",
        "draconic_rt_host_process_user_arg",
        "draconic_rt_host_env_get",
        "draconic_rt_host_env_set",
        "draconic_rt_host_env_delete",
        "draconic_rt_host_process_exit",
        "draconic_rt_host_process_set_exit_code",
        "draconic_rt_host_process_get_exit_code",
        "draconic_rt_host_process_pid",
        "draconic_rt_host_process_ppid",
        "draconic_rt_host_now_ms",
        "draconic_rt_host_monotonic_ms",
        "draconic_rt_host_stdout_write",
        "draconic_rt_host_stderr_write",
        "draconic_rt_host_stdin_read_line",
        "draconic_rt_host_stdin_read_bytes",
        "draconic_rt_host_path_normalize",
        "draconic_rt_host_path_join",
        "draconic_rt_host_path_dirname",
        "draconic_rt_host_path_basename",
        "draconic_rt_host_path_extname",
        "draconic_rt_host_path_is_absolute",
    ] {
        assert!(
            out.contains(sym),
            "archive must contain host symbol {sym}\nnm out={out}"
        );
    }
}

#[test]
fn host_process_argv_user_args() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_argv.c");
    let bin = dir.join("rt_host_argv");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
#include <string.h>
int main(int argc, char **argv) {
    draconic_rt_host_process_set_argv(argc, argv);
    int n = draconic_rt_host_process_user_argc();
    printf("%d\n", n);
    for (int i = 0; i < n; i++) {
        const char *a = draconic_rt_host_process_user_arg(i);
        printf("%s\n", a ? a : "");
    }
    if (draconic_rt_host_process_user_arg(n) != NULL) return 2;
    if (draconic_rt_host_process_user_arg(-1) != NULL) return 3;
    return 0;
}
"#,
    )
    .unwrap();
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg(format!("-I{}", header_dir.display()))
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("clang link")
        };
    assert!(status.success(), "link failed");
    let out = Command::new(&bin)
        .args(["alpha", "beta"])
        .output()
        .expect("run");
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "2\nalpha\nbeta\n",
        "stdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn host_process_env_get_set_delete() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_env.c");
    let bin = dir.join("rt_host_env");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(void) {
    const char *k = "DRACONIC_RT_HOST_ENV_TEST";
    char *v;
    if (draconic_rt_host_env_set(k, "alpha") != 0) return 1;
    v = draconic_rt_host_env_get(k);
    if (!v || strcmp(v, "alpha") != 0) { free(v); return 2; }
    free(v);
    if (draconic_rt_host_env_get("DRACONIC_RT_HOST_ENV_MISSING_XYZ") != NULL) return 3;
    if (draconic_rt_host_env_delete(k) != 0) return 4;
    if (draconic_rt_host_env_get(k) != NULL) return 5;
    printf("ok\n");
    return 0;
}
"#,
    )
    .unwrap();
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg(format!("-I{}", header_dir.display()))
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("clang link")
        };
    assert!(status.success(), "link failed");
    let out = Command::new(&bin).output().expect("run");
    assert!(
        out.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ok\n");
}

#[test]
fn host_process_exit_code_and_exit() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_exit.c");
    let bin = dir.join("rt_host_exit");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
int main(void) {
    if (draconic_rt_host_process_get_exit_code() != 0) return 1;
    draconic_rt_host_process_set_exit_code(5);
    if (draconic_rt_host_process_get_exit_code() != 5) return 2;
    /* Immediate terminate with 7 (never returns). */
    draconic_rt_host_process_exit(7);
    return 99;
}
"#,
    )
    .unwrap();
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg(format!("-I{}", header_dir.display()))
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("clang link")
        };
    assert!(status.success(), "link failed");
    let out = Command::new(&bin).output().expect("run");
    assert_eq!(
        out.status.code(),
        Some(7),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn host_process_pid_ppid() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_pid.c");
    let bin = dir.join("rt_host_pid");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
int main(void) {
    int32_t p = draconic_rt_host_process_pid();
    int32_t pp = draconic_rt_host_process_ppid();
    if (p <= 0) return 1;
    if (pp < 0) return 2;
    printf("%d\n%d\n", (int)p, (int)pp);
    return 0;
}
"#,
    )
    .unwrap();
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg(format!("-I{}", header_dir.display()))
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("clang link")
        };
    assert!(status.success(), "link failed");
    let out = Command::new(&bin).output().expect("run");
    assert!(
        out.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut lines = stdout.lines();
    let p: i32 = lines
        .next()
        .expect("pid line")
        .parse()
        .expect("pid int");
    let pp: i32 = lines
        .next()
        .expect("ppid line")
        .parse()
        .expect("ppid int");
    assert!(p > 0, "pid={p}");
    assert!(pp >= 0, "ppid={pp}");
    // Child binary has its own pid; ppid should be this test process.
    assert_eq!(pp as u32, std::process::id());
}

#[test]
fn host_now_ms_wall_clock() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_now_ms.c");
    let bin = dir.join("rt_host_now_ms");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
int main(void) {
    double a = draconic_rt_host_now_ms();
    double b = draconic_rt_host_now_ms();
    /* After 2020-09 and before year ~2096. */
    if (!(a > 1600000000000.0 && a < 4000000000000.0)) return 1;
    if (!(b >= a)) return 2;
    printf("ok\n");
    return 0;
}
"#,
    )
    .unwrap();
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg(format!("-I{}", header_dir.display()))
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("clang link")
        };
    assert!(status.success(), "link failed");
    let out = Command::new(&bin).output().expect("run");
    assert!(
        out.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ok\n");
}

#[test]
fn host_monotonic_ms_steady() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_monotonic_ms.c");
    let bin = dir.join("rt_host_monotonic_ms");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
int main(void) {
    double a = draconic_rt_host_monotonic_ms();
    double b = draconic_rt_host_monotonic_ms();
    if (!(a >= 0.0)) return 1;
    if (!(b >= a)) return 2;
    if (!((b - a) < 60000.0)) return 3;
    printf("ok\n");
    return 0;
}
"#,
    )
    .unwrap();
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg(format!("-I{}", header_dir.display()))
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("clang link")
        };
    assert!(status.success(), "link failed");
    let out = Command::new(&bin).output().expect("run");
    assert!(
        out.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ok\n");
}

#[test]
fn host_stdout_write_bytes() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_stdout.c");
    let bin = dir.join("rt_host_stdout");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
int main(void) {
    const uint8_t msg[] = { 'h', 'i', '\n', 0 };
    if (draconic_rt_host_stdout_write(msg, 3) != DRACONIC_HOST_OK) return 1;
    if (draconic_rt_host_stdout_write(NULL, 0) != DRACONIC_HOST_OK) return 2;
    if (draconic_rt_host_stdout_write(NULL, 1) != DRACONIC_HOST_E_INVAL) return 3;
    return 0;
}
"#,
    )
    .unwrap();
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg(format!("-I{}", header_dir.display()))
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("clang link")
        };
    assert!(status.success(), "link failed");
    let out = Command::new(&bin).output().expect("run");
    assert!(
        out.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "hi\n");
}

#[test]
fn host_stderr_write_bytes() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_stderr.c");
    let bin = dir.join("rt_host_stderr");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
int main(void) {
    const uint8_t msg[] = { 'e', 'r', '\n', 0 };
    if (draconic_rt_host_stderr_write(msg, 3) != DRACONIC_HOST_OK) return 1;
    if (draconic_rt_host_stderr_write(NULL, 0) != DRACONIC_HOST_OK) return 2;
    if (draconic_rt_host_stderr_write(NULL, 1) != DRACONIC_HOST_E_INVAL) return 3;
    return 0;
}
"#,
    )
    .unwrap();
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg(format!("-I{}", header_dir.display()))
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("clang link")
        };
    assert!(status.success(), "link failed");
    let out = Command::new(&bin).output().expect("run");
    assert!(
        out.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&out.stderr), "er\n");
}

#[test]
fn host_stdin_read_line_and_bytes() {
    use std::io::Write;
    use std::process::Stdio;

    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_stdin.c");
    let bin = dir.join("rt_host_stdin");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(void) {
    char *line = draconic_rt_host_stdin_read_line();
    if (!line) return 1;
    if (strcmp(line, "hi") != 0) { free(line); return 2; }
    free(line);
    uint8_t *data = NULL;
    size_t n = 0;
    if (draconic_rt_host_stdin_read_bytes(3, &data, &n) != DRACONIC_HOST_OK) return 3;
    if (n != 3 || !data) return 4;
    if (data[0] != 'A' || data[1] != 'B' || data[2] != 'C') { free(data); return 5; }
    free(data);
    line = draconic_rt_host_stdin_read_line();
    if (line != NULL) { free(line); return 6; }
    return 0;
}
"#,
    )
    .unwrap();
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg(format!("-I{}", header_dir.display()))
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("clang link")
        };
    assert!(status.success(), "link failed");
    let mut child = Command::new(&bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    {
        let mut sin = child.stdin.take().expect("stdin");
        sin.write_all(b"hi\nABC").expect("write stdin");
    }
    let out = child.wait_with_output().expect("wait");
    assert!(
        out.status.success(),
        "exit={:?} stderr={} stdout={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn host_abi_path_and_handles_link_smoke() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_host_abi");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <string.h>
        #include <stdlib.h>

        int main(void) {
            char *path = NULL;
            DraconicHostError err;

            /* Valid UTF-8 path → OK, NUL-terminated copy. */
            err = draconic_rt_host_path_from_utf8("tmp/x", 5, &path);
            if (err != DRACONIC_HOST_OK || !path) {
                fprintf(stderr, "path_from_utf8 want OK got %d path=%p\n",
                        (int)err, (void *)path);
                return 1;
            }
            if (strcmp(path, "tmp/x") != 0) {
                fprintf(stderr, "path contents wrong: %s\n", path);
                return 2;
            }
            draconic_rt_host_path_free(path);
            path = NULL;

            /* Empty path is valid (zero-length relative). */
            err = draconic_rt_host_path_from_utf8("", 0, &path);
            if (err != DRACONIC_HOST_OK || !path || path[0] != '\0') {
                fprintf(stderr, "empty path failed err=%d\n", (int)err);
                return 3;
            }
            draconic_rt_host_path_free(path);
            path = NULL;

            /* Embedded NUL rejected. */
            err = draconic_rt_host_path_from_utf8("a\0b", 3, &path);
            if (err != DRACONIC_HOST_E_INVAL || path != NULL) {
                fprintf(stderr, "embedded NUL want E_INVAL got %d\n", (int)err);
                return 4;
            }

            /* Invalid UTF-8 rejected (overlong / bare continuation). */
            {
                const char bad[] = { (char)0x80, 0 };
                err = draconic_rt_host_path_from_utf8(bad, 1, &path);
                if (err != DRACONIC_HOST_E_INVAL || path != NULL) {
                    fprintf(stderr, "bad utf8 want E_INVAL got %d\n", (int)err);
                    return 5;
                }
            }

            /* NULL out_path → E_INVAL. */
            err = draconic_rt_host_path_from_utf8("x", 1, NULL);
            if (err != DRACONIC_HOST_E_INVAL) {
                fprintf(stderr, "null out want E_INVAL got %d\n", (int)err);
                return 6;
            }

            /* NULL data with len>0 → E_INVAL. */
            err = draconic_rt_host_path_from_utf8(NULL, 1, &path);
            if (err != DRACONIC_HOST_E_INVAL || path != NULL) {
                fprintf(stderr, "null data want E_INVAL got %d\n", (int)err);
                return 7;
            }

            /* Handles: invalid is never valid; close → E_BADF. */
            if (draconic_rt_host_handle_is_valid(DRACONIC_HOST_HANDLE_INVALID)) {
                fprintf(stderr, "INVALID handle must not be valid\n");
                return 8;
            }
            if (draconic_rt_host_handle_is_valid(0)) {
                fprintf(stderr, "handle 0 must not be valid\n");
                return 9;
            }
            err = draconic_rt_host_handle_close(DRACONIC_HOST_HANDLE_INVALID);
            if (err != DRACONIC_HOST_E_BADF) {
                fprintf(stderr, "close INVALID want E_BADF got %d\n", (int)err);
                return 10;
            }

            /* Non-UTF8 multi-byte path with valid UTF-8 (emoji dir) OK. */
            {
                /* U+1F4C1 📁 = F0 9F 93 81 */
                const char *emoji = "\xF0\x9F\x93\x81";
                err = draconic_rt_host_path_from_utf8(emoji, 4, &path);
                if (err != DRACONIC_HOST_OK || !path
                    || memcmp(path, emoji, 4) != 0 || path[4] != '\0') {
                    fprintf(stderr, "emoji path failed err=%d\n", (int)err);
                    return 11;
                }
                draconic_rt_host_path_free(path);
            }

            puts("host-abi-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(
        status.success(),
        "clang failed to link host ABI smoke against libdraconic_rt.a"
    );

    let output = Command::new(&bin).output().expect("run rt_host_abi");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "host ABI binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "host-abi-ok\n", "stdout={stdout:?}");
}

#[test]
fn host_path_dirname_basename_extname_is_absolute() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_path_h0302.c");
    let bin = dir.join("rt_host_path_h0302");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <string.h>
        #include <stdlib.h>

        static int expect_str(const char *got, const char *want, const char *label) {
            if (!got) {
                fprintf(stderr, "%s: null\n", label);
                return 0;
            }
            if (strcmp(got, want) != 0) {
                fprintf(stderr, "%s: got \"%s\" want \"%s\"\n", label, got, want);
                return 0;
            }
            return 1;
        }

        int main(void) {
            char *s;

            s = draconic_rt_host_path_dirname("/foo/bar/baz");
            if (!expect_str(s, "/foo/bar", "dirname abs")) return 1;
            free(s);
            s = draconic_rt_host_path_dirname("foo");
            if (!expect_str(s, ".", "dirname rel")) return 2;
            free(s);
            s = draconic_rt_host_path_dirname("foo\\bar\\baz");
            if (!expect_str(s, "foo/bar", "dirname backslash")) return 3;
            free(s);

            s = draconic_rt_host_path_basename("/foo/bar/baz.txt");
            if (!expect_str(s, "baz.txt", "basename")) return 4;
            free(s);
            s = draconic_rt_host_path_basename("/");
            if (!expect_str(s, "", "basename root")) return 5;
            free(s);

            s = draconic_rt_host_path_extname("index.coffee.md");
            if (!expect_str(s, ".md", "extname multi")) return 6;
            free(s);
            s = draconic_rt_host_path_extname(".index");
            if (!expect_str(s, "", "extname dotfile")) return 7;
            free(s);
            s = draconic_rt_host_path_extname("index.");
            if (!expect_str(s, ".", "extname trailing dot")) return 8;
            free(s);

            if (draconic_rt_host_path_is_absolute("/foo") != 1) return 9;
            if (draconic_rt_host_path_is_absolute("foo") != 0) return 10;
            if (draconic_rt_host_path_is_absolute("\\foo") != 1) return 11;
            if (draconic_rt_host_path_is_absolute("") != 0) return 12;

            puts("path-h0302-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for path H03.02 smoke");

    let output = Command::new(&bin).output().expect("run path h0302");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "path H03.02 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "path-h0302-ok\n"
    );
}

#[test]
fn host_fs_read_text_and_bytes() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_fs_h0401.c");
    let bin = dir.join("rt_host_fs_h0401");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    let hello = dir.join("hello.txt");
    std::fs::write(&hello, b"hello-h0401").unwrap();
    let empty = dir.join("empty.txt");
    std::fs::write(&empty, b"").unwrap();
    let hello_path = hello.to_string_lossy().replace('\\', "\\\\");
    let empty_path = empty.to_string_lossy().replace('\\', "\\\\");
    let missing_path = dir
        .join("__no_such_h0401__")
        .to_string_lossy()
        .replace('\\', "\\\\");

    std::fs::write(
        &main_c,
        format!(
            r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <string.h>
        #include <stdlib.h>

        int main(void) {{
            char *text = NULL;
            uint8_t *data = NULL;
            size_t len = 0;
            DraconicHostError err;

            err = draconic_rt_host_fs_read_text("{hello_path}", &text);
            if (err != DRACONIC_HOST_OK) return 1;
            if (!text || strcmp(text, "hello-h0401") != 0) return 2;
            free(text);

            err = draconic_rt_host_fs_read_file("{hello_path}", &data, &len);
            if (err != DRACONIC_HOST_OK) return 3;
            if (len != 11 || !data || memcmp(data, "hello-h0401", 11) != 0) return 4;
            free(data);

            err = draconic_rt_host_fs_read_text("{empty_path}", &text);
            if (err != DRACONIC_HOST_OK) return 5;
            if (!text || text[0] != '\0') return 6;
            free(text);

            err = draconic_rt_host_fs_read_file("{empty_path}", &data, &len);
            if (err != DRACONIC_HOST_OK) return 7;
            if (len != 0 || data != NULL) return 8;

            err = draconic_rt_host_fs_read_text("{missing_path}", &text);
            if (err != DRACONIC_HOST_E_NOENT) return 9;
            if (text != NULL) return 10;

            err = draconic_rt_host_fs_read_file(NULL, &data, &len);
            if (err != DRACONIC_HOST_E_INVAL) return 11;

            puts("fs-h0401-ok");
            return 0;
        }}
        "#
        ),
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for fs H04.01 smoke");

    let output = Command::new(&bin).output().expect("run fs h0401");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "fs H04.01 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "fs-h0401-ok\n");
}

#[test]
fn host_fs_write_append_text_and_bytes() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_fs_h0402.c");
    let bin = dir.join("rt_host_fs_h0402");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    let out_path = dir.join("out.txt");
    let out_path_s = out_path.to_string_lossy().replace('\\', "\\\\");
    let bin_path = dir.join("out.bin");
    let bin_path_s = bin_path.to_string_lossy().replace('\\', "\\\\");
    let missing_parent = dir
        .join("no_such_dir")
        .join("nested.txt")
        .to_string_lossy()
        .replace('\\', "\\\\");

    std::fs::write(
        &main_c,
        format!(
            r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <string.h>
        #include <stdlib.h>

        int main(void) {{
            char *text = NULL;
            uint8_t *data = NULL;
            size_t len = 0;
            DraconicHostError err;

            err = draconic_rt_host_fs_write_text("{out_path_s}", "wt-h0402");
            if (err != DRACONIC_HOST_OK) return 1;
            err = draconic_rt_host_fs_read_text("{out_path_s}", &text);
            if (err != DRACONIC_HOST_OK) return 2;
            if (!text || strcmp(text, "wt-h0402") != 0) return 3;
            free(text); text = NULL;

            err = draconic_rt_host_fs_write_text("{out_path_s}", "long-content");
            if (err != DRACONIC_HOST_OK) return 4;
            err = draconic_rt_host_fs_write_text("{out_path_s}", "short");
            if (err != DRACONIC_HOST_OK) return 5;
            err = draconic_rt_host_fs_read_text("{out_path_s}", &text);
            if (err != DRACONIC_HOST_OK) return 6;
            if (!text || strcmp(text, "short") != 0) return 7;
            free(text); text = NULL;

            err = draconic_rt_host_fs_write_text("{out_path_s}", "A");
            if (err != DRACONIC_HOST_OK) return 8;
            err = draconic_rt_host_fs_append_text("{out_path_s}", "B");
            if (err != DRACONIC_HOST_OK) return 9;
            err = draconic_rt_host_fs_append_text("{out_path_s}", "C");
            if (err != DRACONIC_HOST_OK) return 10;
            err = draconic_rt_host_fs_read_text("{out_path_s}", &text);
            if (err != DRACONIC_HOST_OK) return 11;
            if (!text || strcmp(text, "ABC") != 0) return 12;
            free(text); text = NULL;

            err = draconic_rt_host_fs_write_file("{bin_path_s}", (const uint8_t *)"xy", 2);
            if (err != DRACONIC_HOST_OK) return 13;
            err = draconic_rt_host_fs_append_file("{bin_path_s}", (const uint8_t *)"z", 1);
            if (err != DRACONIC_HOST_OK) return 14;
            err = draconic_rt_host_fs_read_file("{bin_path_s}", &data, &len);
            if (err != DRACONIC_HOST_OK) return 15;
            if (len != 3 || !data || memcmp(data, "xyz", 3) != 0) return 16;
            free(data); data = NULL;

            err = draconic_rt_host_fs_write_text("{out_path_s}", "");
            if (err != DRACONIC_HOST_OK) return 17;
            err = draconic_rt_host_fs_read_text("{out_path_s}", &text);
            if (err != DRACONIC_HOST_OK) return 18;
            if (!text || text[0] != '\0') return 19;
            free(text); text = NULL;

            err = draconic_rt_host_fs_write_text("{missing_parent}", "x");
            if (err != DRACONIC_HOST_E_NOENT) return 20;

            err = draconic_rt_host_fs_write_text(NULL, "x");
            if (err != DRACONIC_HOST_E_INVAL) return 21;

            puts("fs-h0402-ok");
            return 0;
        }}
        "#
        ),
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for fs H04.02 smoke");

    let output = Command::new(&bin).output().expect("run fs h0402");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "fs H04.02 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "fs-h0402-ok\n");
}

#[test]
fn host_fs_exists_and_stat() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_fs_h0403.c");
    let bin = dir.join("rt_host_fs_h0403");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    let hello = dir.join("hello.txt");
    std::fs::write(&hello, b"hello-h0403").unwrap();
    let sub = dir.join("subdir");
    std::fs::create_dir_all(&sub).unwrap();
    let hello_path = hello.to_string_lossy().replace('\\', "\\\\");
    let sub_path = sub.to_string_lossy().replace('\\', "\\\\");
    let missing_path = dir
        .join("__no_such_h0403__")
        .to_string_lossy()
        .replace('\\', "\\\\");

    std::fs::write(
        &main_c,
        format!(
            r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>

        int main(void) {{
            int64_t size = 0;
            int32_t is_file = 0;
            int32_t is_dir = 0;
            double mtime = 0.0;
            DraconicHostError err;

            if (draconic_rt_host_fs_exists("{hello_path}") != 1) return 1;
            if (draconic_rt_host_fs_exists("{sub_path}") != 1) return 2;
            if (draconic_rt_host_fs_exists("{missing_path}") != 0) return 3;
            if (draconic_rt_host_fs_exists(NULL) != 0) return 4;
            if (draconic_rt_host_fs_exists("") != 0) return 5;

            err = draconic_rt_host_fs_stat("{hello_path}", &size, &is_file, &is_dir, &mtime);
            if (err != DRACONIC_HOST_OK) return 6;
            if (size != 11) return 7;
            if (is_file != 1) return 8;
            if (is_dir != 0) return 9;
            if (!(mtime > 0.0)) return 10;

            err = draconic_rt_host_fs_stat("{sub_path}", &size, &is_file, &is_dir, &mtime);
            if (err != DRACONIC_HOST_OK) return 11;
            if (is_file != 0) return 12;
            if (is_dir != 1) return 13;

            err = draconic_rt_host_fs_stat("{missing_path}", &size, &is_file, &is_dir, &mtime);
            if (err != DRACONIC_HOST_E_NOENT) return 14;

            err = draconic_rt_host_fs_stat(NULL, &size, &is_file, &is_dir, &mtime);
            if (err != DRACONIC_HOST_E_INVAL) return 15;

            puts("fs-h0403-ok");
            return 0;
        }}
        "#
        ),
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for fs H04.03 smoke");

    let output = Command::new(&bin).output().expect("run fs h0403");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "fs H04.03 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "fs-h0403-ok\n");
}

#[test]
fn host_fs_dir_ops() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_fs_h0404.c");
    let bin = dir.join("rt_host_fs_h0404");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    let base = dir.join("h0404");
    let base_path = base.to_string_lossy().replace('\\', "\\\\");
    let nested = base.join("a").join("b");
    let nested_path = nested.to_string_lossy().replace('\\', "\\\\");
    let file_path = base
        .join("only.txt")
        .to_string_lossy()
        .replace('\\', "\\\\");
    let child = base.join("child");
    let child_path = child.to_string_lossy().replace('\\', "\\\\");

    std::fs::write(
        &main_c,
        format!(
            r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <stdlib.h>
        #include <string.h>

        int main(void) {{
            DraconicHostError err;
            char **names = NULL;
            int64_t count = 0;

            err = draconic_rt_host_fs_mkdir_all("{nested_path}");
            if (err != DRACONIC_HOST_OK) return 1;
            if (draconic_rt_host_fs_exists("{nested_path}") != 1) return 2;

            err = draconic_rt_host_fs_mkdir("{child_path}");
            if (err != DRACONIC_HOST_OK) return 3;
            err = draconic_rt_host_fs_mkdir("{child_path}");
            if (err != DRACONIC_HOST_E_EXIST) return 4;

            {{
                FILE *f = fopen("{file_path}", "w");
                if (!f) return 5;
                fputs("x", f);
                fclose(f);
            }}

            err = draconic_rt_host_fs_readdir("{base_path}", &names, &count);
            if (err != DRACONIC_HOST_OK) return 6;
            if (count < 2) return 7; /* child dir + only.txt at least */
            {{
                int found = 0;
                for (int64_t i = 0; i < count; i++) {{
                    if (names[i] && strcmp(names[i], "only.txt") == 0) found = 1;
                    free(names[i]);
                }}
                free(names);
                names = NULL;
                if (!found) return 8;
            }}

            err = draconic_rt_host_fs_remove_file("{file_path}");
            if (err != DRACONIC_HOST_OK) return 9;
            if (draconic_rt_host_fs_exists("{file_path}") != 0) return 10;

            err = draconic_rt_host_fs_rmdir("{child_path}");
            if (err != DRACONIC_HOST_OK) return 11;
            if (draconic_rt_host_fs_exists("{child_path}") != 0) return 12;

            err = draconic_rt_host_fs_mkdir(NULL);
            if (err != DRACONIC_HOST_E_INVAL) return 13;
            err = draconic_rt_host_fs_readdir("{base_path}/__no_such__", &names, &count);
            if (err != DRACONIC_HOST_E_NOENT) return 14;

            puts("fs-h0404-ok");
            return 0;
        }}
        "#
        ),
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for fs H04.04 smoke");

    let output = Command::new(&bin).output().expect("run fs h0404");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "fs H04.04 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "fs-h0404-ok\n");
}

#[test]
fn host_fs_rename_and_copy() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_fs_h0405.c");
    let bin = dir.join("rt_host_fs_h0405");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    let base = dir.join("h0405");
    std::fs::create_dir_all(&base).unwrap();
    let src = base
        .join("src.txt")
        .to_string_lossy()
        .replace('\\', "\\\\");
    let ren_dst = base
        .join("ren.txt")
        .to_string_lossy()
        .replace('\\', "\\\\");
    let cp_src = base
        .join("cp_src.txt")
        .to_string_lossy()
        .replace('\\', "\\\\");
    let cp_dst = base
        .join("cp_dst.txt")
        .to_string_lossy()
        .replace('\\', "\\\\");

    std::fs::write(
        &main_c,
        format!(
            r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <stdlib.h>
        #include <string.h>

        int main(void) {{
            DraconicHostError err;
            char *text = NULL;

            err = draconic_rt_host_fs_write_text("{src}", "ren-h0405");
            if (err != DRACONIC_HOST_OK) return 1;
            err = draconic_rt_host_fs_rename_file("{src}", "{ren_dst}");
            if (err != DRACONIC_HOST_OK) return 2;
            if (draconic_rt_host_fs_exists("{src}") != 0) return 3;
            err = draconic_rt_host_fs_read_text("{ren_dst}", &text);
            if (err != DRACONIC_HOST_OK) return 4;
            if (!text || strcmp(text, "ren-h0405") != 0) return 5;
            free(text);
            text = NULL;

            err = draconic_rt_host_fs_write_text("{cp_src}", "cp-h0405");
            if (err != DRACONIC_HOST_OK) return 6;
            err = draconic_rt_host_fs_copy_file("{cp_src}", "{cp_dst}");
            if (err != DRACONIC_HOST_OK) return 7;
            err = draconic_rt_host_fs_read_text("{cp_src}", &text);
            if (err != DRACONIC_HOST_OK) return 8;
            if (!text || strcmp(text, "cp-h0405") != 0) return 9;
            free(text);
            text = NULL;
            err = draconic_rt_host_fs_read_text("{cp_dst}", &text);
            if (err != DRACONIC_HOST_OK) return 10;
            if (!text || strcmp(text, "cp-h0405") != 0) return 11;
            free(text);

            err = draconic_rt_host_fs_rename_file(NULL, "{ren_dst}");
            if (err != DRACONIC_HOST_E_INVAL) return 12;
            err = draconic_rt_host_fs_copy_file("{cp_src}/__missing__", "{cp_dst}");
            if (err != DRACONIC_HOST_E_NOENT) return 13;

            puts("fs-h0405-ok");
            return 0;
        }}
        "#
        ),
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for fs H04.05 smoke");

    let output = Command::new(&bin).output().expect("run fs h0405");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "fs H04.05 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "fs-h0405-ok\n");
}

#[test]
fn host_fs_open_handle_rw_seek_close() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_fs_h0406.c");
    let bin = dir.join("rt_host_fs_h0406");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    let path = dir
        .join("h0406.txt")
        .to_string_lossy()
        .replace('\\', "\\\\");

    std::fs::write(
        &main_c,
        format!(
            r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <stdlib.h>
        #include <string.h>

        int main(void) {{
            DraconicHostError err;
            DraconicHostHandle h = DRACONIC_HOST_HANDLE_INVALID;
            uint8_t *data = NULL;
            size_t len = 0;
            int64_t pos = -1;

            err = draconic_rt_host_fs_open("{path}", "w+", &h);
            if (err != DRACONIC_HOST_OK) return 1;
            if (!draconic_rt_host_handle_is_valid(h)) return 2;

            err = draconic_rt_host_fs_handle_write(h, (const uint8_t *)"hello-h0406", 11);
            if (err != DRACONIC_HOST_OK) return 3;

            err = draconic_rt_host_fs_handle_seek(h, 0, 0, &pos);
            if (err != DRACONIC_HOST_OK) return 4;
            if (pos != 0) return 5;

            err = draconic_rt_host_fs_handle_read(h, 64, &data, &len);
            if (err != DRACONIC_HOST_OK) return 6;
            if (len != 11 || !data || memcmp(data, "hello-h0406", 11) != 0) return 7;
            free(data);
            data = NULL;

            err = draconic_rt_host_fs_handle_seek(h, 6, 0, &pos);
            if (err != DRACONIC_HOST_OK) return 8;
            if (pos != 6) return 9;
            err = draconic_rt_host_fs_handle_read(h, 64, &data, &len);
            if (err != DRACONIC_HOST_OK) return 10;
            if (len != 5 || !data || memcmp(data, "h0406", 5) != 0) return 11;
            free(data);

            err = draconic_rt_host_handle_close(h);
            if (err != DRACONIC_HOST_OK) return 12;
            if (draconic_rt_host_handle_is_valid(h)) return 13;
            err = draconic_rt_host_handle_close(h);
            if (err != DRACONIC_HOST_E_BADF) return 14;

            err = draconic_rt_host_fs_open("{path}/__missing_parent__/x", "r", &h);
            if (err != DRACONIC_HOST_E_NOENT) return 15;

            err = draconic_rt_host_fs_open(NULL, "r", &h);
            if (err != DRACONIC_HOST_E_INVAL) return 16;
            err = draconic_rt_host_fs_open("{path}", "zz", &h);
            if (err != DRACONIC_HOST_E_INVAL) return 17;

            puts("fs-h0406-ok");
            return 0;
        }}
        "#
        ),
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for fs H04.06 smoke");

    let output = Command::new(&bin).output().expect("run fs h0406");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "fs H04.06 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "fs-h0406-ok\n");
}

#[test]
fn host_tcp_listen_ephemeral_local_port_close() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_tcp_h0601.c");
    let bin = dir.join("rt_host_tcp_h0601");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>

        int main(void) {
            DraconicHostError err;
            DraconicHostHandle h = DRACONIC_HOST_HANDLE_INVALID;
            int32_t port = 0;

            err = draconic_rt_host_tcp_listen(0, 0, &h);
            if (err != DRACONIC_HOST_OK) return 1;
            if (!draconic_rt_host_handle_is_valid(h)) return 2;

            err = draconic_rt_host_tcp_local_port(h, &port);
            if (err != DRACONIC_HOST_OK) return 3;
            if (port <= 0 || port > 65535) return 4;

            err = draconic_rt_host_handle_close(h);
            if (err != DRACONIC_HOST_OK) return 5;
            if (draconic_rt_host_handle_is_valid(h)) return 6;
            err = draconic_rt_host_handle_close(h);
            if (err != DRACONIC_HOST_E_BADF) return 7;

            err = draconic_rt_host_tcp_listen(-1, 8, &h);
            if (err != DRACONIC_HOST_E_INVAL) return 8;
            err = draconic_rt_host_tcp_listen(70000, 8, &h);
            if (err != DRACONIC_HOST_E_INVAL) return 9;
            err = draconic_rt_host_tcp_local_port(DRACONIC_HOST_HANDLE_INVALID, &port);
            if (err != DRACONIC_HOST_E_BADF) return 10;

            puts("tcp-h0601-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for tcp H06.01 smoke");

    let output = Command::new(&bin).output().expect("run tcp h0601");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "tcp H06.01 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "tcp-h0601-ok\n");
}

#[test]
fn host_tcp_accept_peer_loopback() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_tcp_h0602.c");
    let bin = dir.join("rt_host_tcp_h0602");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <string.h>

        int main(void) {
            DraconicHostError err;
            DraconicHostHandle listen_h = DRACONIC_HOST_HANDLE_INVALID;
            DraconicHostHandle client_h = DRACONIC_HOST_HANDLE_INVALID;
            DraconicHostHandle accept_h = DRACONIC_HOST_HANDLE_INVALID;
            int32_t port = 0;
            int32_t peer_port = 0;
            char *peer_addr = NULL;

            err = draconic_rt_host_tcp_listen(0, 8, &listen_h);
            if (err != DRACONIC_HOST_OK) return 1;
            err = draconic_rt_host_tcp_local_port(listen_h, &port);
            if (err != DRACONIC_HOST_OK) return 2;
            if (port <= 0 || port > 65535) return 3;

            err = draconic_rt_host_tcp_connect("127.0.0.1", port, &client_h);
            if (err != DRACONIC_HOST_OK) return 4;
            if (!draconic_rt_host_handle_is_valid(client_h)) return 5;

            err = draconic_rt_host_tcp_accept(listen_h, &accept_h);
            if (err != DRACONIC_HOST_OK) return 6;
            if (!draconic_rt_host_handle_is_valid(accept_h)) return 7;

            err = draconic_rt_host_tcp_peer_address(accept_h, &peer_addr);
            if (err != DRACONIC_HOST_OK) return 8;
            if (!peer_addr || strcmp(peer_addr, "127.0.0.1") != 0) return 9;

            err = draconic_rt_host_tcp_peer_port(accept_h, &peer_port);
            if (err != DRACONIC_HOST_OK) return 10;
            if (peer_port <= 0 || peer_port > 65535) return 11;

            draconic_rt_host_path_free(peer_addr);
            err = draconic_rt_host_handle_close(accept_h);
            if (err != DRACONIC_HOST_OK) return 12;
            err = draconic_rt_host_handle_close(client_h);
            if (err != DRACONIC_HOST_OK) return 13;
            err = draconic_rt_host_handle_close(listen_h);
            if (err != DRACONIC_HOST_OK) return 14;

            err = draconic_rt_host_tcp_accept(DRACONIC_HOST_HANDLE_INVALID, &accept_h);
            if (err != DRACONIC_HOST_E_BADF) return 15;
            err = draconic_rt_host_tcp_peer_port(DRACONIC_HOST_HANDLE_INVALID, &peer_port);
            if (err != DRACONIC_HOST_E_BADF) return 16;
            err = draconic_rt_host_tcp_connect(NULL, 1, &client_h);
            if (err != DRACONIC_HOST_E_INVAL) return 17;
            err = draconic_rt_host_tcp_connect("127.0.0.1", 0, &client_h);
            if (err != DRACONIC_HOST_E_INVAL) return 18;

            puts("tcp-h0602-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for tcp H06.02 smoke");

    let output = Command::new(&bin).output().expect("run tcp h0602");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "tcp H06.02 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "tcp-h0602-ok\n");
}

#[test]
fn host_tcp_connect_dial_and_refused() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_tcp_h0603.c");
    let bin = dir.join("rt_host_tcp_h0603");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>

        int main(void) {
            DraconicHostError err;
            DraconicHostHandle listen_h = DRACONIC_HOST_HANDLE_INVALID;
            DraconicHostHandle client_h = DRACONIC_HOST_HANDLE_INVALID;
            int32_t port = 0;
            int32_t closed_port = 0;

            /* Dial success: listen + connect to bound port. */
            err = draconic_rt_host_tcp_listen(0, 8, &listen_h);
            if (err != DRACONIC_HOST_OK) return 1;
            err = draconic_rt_host_tcp_local_port(listen_h, &port);
            if (err != DRACONIC_HOST_OK) return 2;
            if (port <= 0 || port > 65535) return 3;

            err = draconic_rt_host_tcp_connect("127.0.0.1", port, &client_h);
            if (err != DRACONIC_HOST_OK) return 4;
            if (!draconic_rt_host_handle_is_valid(client_h)) return 5;
            err = draconic_rt_host_handle_close(client_h);
            if (err != DRACONIC_HOST_OK) return 6;
            client_h = DRACONIC_HOST_HANDLE_INVALID;

            /* Refused: close listener then dial same port → E_CONN. */
            closed_port = port;
            err = draconic_rt_host_handle_close(listen_h);
            if (err != DRACONIC_HOST_OK) return 7;
            listen_h = DRACONIC_HOST_HANDLE_INVALID;

            err = draconic_rt_host_tcp_connect("127.0.0.1", closed_port, &client_h);
            if (err != DRACONIC_HOST_E_CONN) return 8;
            if (draconic_rt_host_handle_is_valid(client_h)) return 9;

            /* Bad port stays E_INVAL; unknown name → E_ADDR (H09.02 resolve). */
            err = draconic_rt_host_tcp_connect(
                "this-host-definitely-does-not-exist.invalid", 80, &client_h);
            if (err != DRACONIC_HOST_E_ADDR) return 10;
            err = draconic_rt_host_tcp_connect("127.0.0.1", 70000, &client_h);
            if (err != DRACONIC_HOST_E_INVAL) return 11;

            puts("tcp-h0603-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for tcp H06.03 smoke");

    let output = Command::new(&bin).output().expect("run tcp h0603");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "tcp H06.03 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "tcp-h0603-ok\n");
}

#[test]
fn host_tcp_read_write_partial_shutdown() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_tcp_h0604.c");
    let bin = dir.join("rt_host_tcp_h0604");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <stdlib.h>
        #include <string.h>

        int main(void) {
            DraconicHostError err;
            DraconicHostHandle listen_h = DRACONIC_HOST_HANDLE_INVALID;
            DraconicHostHandle client_h = DRACONIC_HOST_HANDLE_INVALID;
            DraconicHostHandle accept_h = DRACONIC_HOST_HANDLE_INVALID;
            int32_t port = 0;
            uint8_t *data = NULL;
            size_t len = 0;

            err = draconic_rt_host_tcp_listen(0, 8, &listen_h);
            if (err != DRACONIC_HOST_OK) return 1;
            err = draconic_rt_host_tcp_local_port(listen_h, &port);
            if (err != DRACONIC_HOST_OK) return 2;
            err = draconic_rt_host_tcp_connect("127.0.0.1", port, &client_h);
            if (err != DRACONIC_HOST_OK) return 3;
            err = draconic_rt_host_tcp_accept(listen_h, &accept_h);
            if (err != DRACONIC_HOST_OK) return 4;

            err = draconic_rt_host_tcp_write(client_h, (const uint8_t *)"hello-tcp", 9);
            if (err != DRACONIC_HOST_OK) return 5;
            err = draconic_rt_host_tcp_read(accept_h, 64, &data, &len);
            if (err != DRACONIC_HOST_OK) return 6;
            if (len != 9 || !data || memcmp(data, "hello-tcp", 9) != 0) return 7;
            free(data);
            data = NULL;

            /* Partial read: write 6, read max 3 twice. */
            err = draconic_rt_host_tcp_write(client_h, (const uint8_t *)"abcdef", 6);
            if (err != DRACONIC_HOST_OK) return 8;
            err = draconic_rt_host_tcp_read(accept_h, 3, &data, &len);
            if (err != DRACONIC_HOST_OK) return 9;
            if (len != 3 || !data || memcmp(data, "abc", 3) != 0) return 10;
            free(data);
            data = NULL;
            err = draconic_rt_host_tcp_read(accept_h, 64, &data, &len);
            if (err != DRACONIC_HOST_OK) return 11;
            if (len != 3 || !data || memcmp(data, "def", 3) != 0) return 12;
            free(data);
            data = NULL;

            /* Shutdown write → peer read returns empty EOF. */
            err = draconic_rt_host_tcp_shutdown(client_h, 1);
            if (err != DRACONIC_HOST_OK) return 13;
            err = draconic_rt_host_tcp_read(accept_h, 64, &data, &len);
            if (err != DRACONIC_HOST_OK) return 14;
            if (len != 0 || data != NULL) return 15;

            err = draconic_rt_host_tcp_write(DRACONIC_HOST_HANDLE_INVALID, (const uint8_t *)"x", 1);
            if (err != DRACONIC_HOST_E_BADF) return 16;
            err = draconic_rt_host_tcp_read(DRACONIC_HOST_HANDLE_INVALID, 8, &data, &len);
            if (err != DRACONIC_HOST_E_BADF) return 17;
            err = draconic_rt_host_tcp_shutdown(client_h, 99);
            if (err != DRACONIC_HOST_E_INVAL) return 18;

            err = draconic_rt_host_handle_close(accept_h);
            if (err != DRACONIC_HOST_OK) return 19;
            err = draconic_rt_host_handle_close(client_h);
            if (err != DRACONIC_HOST_OK) return 20;
            err = draconic_rt_host_handle_close(listen_h);
            if (err != DRACONIC_HOST_OK) return 21;

            puts("tcp-h0604-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for tcp H06.04 smoke");

    let output = Command::new(&bin).output().expect("run tcp h0604");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "tcp H06.04 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "tcp-h0604-ok\n");
}

/// H07.01: non-blocking TCP + readiness wait completes via job_drain.
#[test]
fn host_tcp_nonblocking_io_wait_via_job_drain() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_tcp_h0701.c");
    let bin = dir.join("rt_host_tcp_h0701");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <stdlib.h>
        #include <string.h>

        static DraconicHostHandle g_listen;
        static DraconicHostHandle g_accepted;
        static int g_ready_fired;
        static int g_read_fired;
        static uint8_t *g_read_data;
        static size_t g_read_len;
        static DraconicHostError g_read_err;

        static void on_accept_ready(void *data) {
            DraconicHostError err;
            (void)data;
            g_ready_fired = 1;
            err = draconic_rt_host_tcp_accept(g_listen, &g_accepted);
            if (err != DRACONIC_HOST_OK) {
                g_accepted = DRACONIC_HOST_HANDLE_INVALID;
            }
        }

        static void on_read_ready(void *data) {
            (void)data;
            g_read_fired = 1;
            g_read_err = draconic_rt_host_tcp_read(
                g_accepted, 8, &g_read_data, &g_read_len);
        }

        int main(void) {
            DraconicHostError err;
            DraconicHostHandle client_h = DRACONIC_HOST_HANDLE_INVALID;
            int32_t port = 0;
            int64_t wait_id = 0;

            g_listen = DRACONIC_HOST_HANDLE_INVALID;
            g_accepted = DRACONIC_HOST_HANDLE_INVALID;
            g_ready_fired = 0;
            g_read_fired = 0;
            g_read_data = NULL;
            g_read_len = 0;
            g_read_err = DRACONIC_HOST_OK;

            err = draconic_rt_host_tcp_listen(0, 8, &g_listen);
            if (err != DRACONIC_HOST_OK) return 1;
            err = draconic_rt_host_tcp_set_nonblocking(g_listen, 1);
            if (err != DRACONIC_HOST_OK) return 2;
            err = draconic_rt_host_tcp_local_port(g_listen, &port);
            if (err != DRACONIC_HOST_OK) return 3;

            /* Non-blocking accept with no client → E_AGAIN. */
            err = draconic_rt_host_tcp_accept(g_listen, &g_accepted);
            if (err != DRACONIC_HOST_E_AGAIN) return 4;
            if (draconic_rt_host_handle_is_valid(g_accepted)) return 5;

            /* Register readiness before peer connects. */
            err = draconic_rt_host_io_wait(
                g_listen, DRACONIC_HOST_IO_READ, on_accept_ready, NULL, &wait_id);
            if (err != DRACONIC_HOST_OK) return 6;
            if (wait_id <= 0) return 7;
            if (!draconic_rt_host_io_pending()) return 8;

            err = draconic_rt_host_tcp_connect("127.0.0.1", port, &client_h);
            if (err != DRACONIC_HOST_OK) return 9;

            /* Drain promotes readiness → job → accept callback. */
            draconic_rt_job_drain();
            if (g_ready_fired != 1) return 10;
            if (!draconic_rt_host_handle_is_valid(g_accepted)) return 11;
            if (draconic_rt_host_io_pending()) return 12;

            err = draconic_rt_host_tcp_set_nonblocking(g_accepted, 1);
            if (err != DRACONIC_HOST_OK) return 13;
            /* Empty nonblocking read → E_AGAIN. */
            {
                uint8_t *tmp = NULL;
                size_t tlen = 0;
                err = draconic_rt_host_tcp_read(g_accepted, 8, &tmp, &tlen);
                if (err != DRACONIC_HOST_E_AGAIN) return 14;
            }

            err = draconic_rt_host_io_wait(
                g_accepted, DRACONIC_HOST_IO_READ, on_read_ready, NULL, &wait_id);
            if (err != DRACONIC_HOST_OK) return 15;
            err = draconic_rt_host_tcp_write(client_h, (const uint8_t *)"hi", 2);
            if (err != DRACONIC_HOST_OK) return 16;
            draconic_rt_job_drain();
            if (g_read_fired != 1) return 17;
            if (g_read_err != DRACONIC_HOST_OK) return 18;
            if (g_read_len != 2 || !g_read_data || memcmp(g_read_data, "hi", 2) != 0) {
                return 19;
            }
            free(g_read_data);
            g_read_data = NULL;

            /* Cancel path: wait then cancel before ready. */
            {
                DraconicHostHandle listen2 = DRACONIC_HOST_HANDLE_INVALID;
                int64_t id2 = 0;
                err = draconic_rt_host_tcp_listen(0, 4, &listen2);
                if (err != DRACONIC_HOST_OK) return 20;
                err = draconic_rt_host_tcp_set_nonblocking(listen2, 1);
                if (err != DRACONIC_HOST_OK) return 21;
                err = draconic_rt_host_io_wait(
                    listen2, DRACONIC_HOST_IO_READ, on_accept_ready, NULL, &id2);
                if (err != DRACONIC_HOST_OK) return 22;
                draconic_rt_host_io_cancel(id2);
                if (draconic_rt_host_io_pending()) return 23;
                err = draconic_rt_host_handle_close(listen2);
                if (err != DRACONIC_HOST_OK) return 24;
            }

            err = draconic_rt_host_tcp_set_nonblocking(DRACONIC_HOST_HANDLE_INVALID, 1);
            if (err != DRACONIC_HOST_E_BADF) return 25;
            err = draconic_rt_host_io_wait(
                DRACONIC_HOST_HANDLE_INVALID, DRACONIC_HOST_IO_READ,
                on_accept_ready, NULL, &wait_id);
            if (err != DRACONIC_HOST_E_BADF) return 26;
            err = draconic_rt_host_io_wait(g_listen, 0, on_accept_ready, NULL, &wait_id);
            if (err != DRACONIC_HOST_E_INVAL) return 27;

            err = draconic_rt_host_handle_close(g_accepted);
            if (err != DRACONIC_HOST_OK) return 28;
            err = draconic_rt_host_handle_close(client_h);
            if (err != DRACONIC_HOST_OK) return 29;
            err = draconic_rt_host_handle_close(g_listen);
            if (err != DRACONIC_HOST_OK) return 30;

            puts("tcp-h0701-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for tcp H07.01 smoke");

    let output = Command::new(&bin).output().expect("run tcp h0701");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "tcp H07.01 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "tcp-h0701-ok\n");
}

/// H07.02: Promise async accept/connect/read/write + cancel on close.
#[test]
fn host_tcp_async_promises_via_job_drain() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_tcp_h0702.c");
    let bin = dir.join("rt_host_tcp_h0702");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <stdlib.h>
        #include <string.h>

        static int g_accepted;
        static int g_connected;
        static int g_nread;
        static int g_nwrite;
        static int g_rejected;
        static DraconicHostHandle g_conn_for_write;
        static DraconicHostHandle g_accepted_conn;

        static void *on_write(void *data, void *value) {
            (void)data;
            g_nwrite = (int)(intptr_t)value;
            if (draconic_rt_host_handle_is_valid(g_conn_for_write)) {
                (void)draconic_rt_host_handle_close(g_conn_for_write);
                g_conn_for_write = DRACONIC_HOST_HANDLE_INVALID;
            }
            return value;
        }
        static void *on_read(void *data, void *value) {
            (void)data;
            g_nread = (int)(intptr_t)value;
            if (draconic_rt_host_handle_is_valid(g_accepted_conn)) {
                (void)draconic_rt_host_handle_close(g_accepted_conn);
                g_accepted_conn = DRACONIC_HOST_HANDLE_INVALID;
            }
            return value;
        }
        static void *on_accept(void *data, void *value) {
            (void)data;
            g_accepted = 1;
            g_accepted_conn = (DraconicHostHandle)(intptr_t)value;
            {
                DraconicValue *rp = draconic_rt_host_tcp_read_async(g_accepted_conn, 8);
                (void)draconic_rt_promise_then(rp, on_read, NULL, NULL, NULL);
            }
            return value;
        }
        static void *on_connect(void *data, void *value) {
            (void)data;
            g_connected = 1;
            g_conn_for_write = (DraconicHostHandle)(intptr_t)value;
            {
                DraconicValue *wp = draconic_rt_host_tcp_write_async(
                    g_conn_for_write, (const uint8_t *)"hi", 2);
                (void)draconic_rt_promise_then(wp, on_write, NULL, NULL, NULL);
            }
            return value;
        }
        static void *on_reject(void *data, void *reason) {
            (void)data;
            (void)reason;
            g_rejected = 1;
            return NULL;
        }

        int main(void) {
            DraconicHostError err;
            DraconicHostHandle listen = DRACONIC_HOST_HANDLE_INVALID;
            DraconicHostHandle listen2 = DRACONIC_HOST_HANDLE_INVALID;
            int32_t port = 0;
            DraconicValue *pa;
            DraconicValue *pc;

            g_accepted = 0;
            g_connected = 0;
            g_nread = 0;
            g_nwrite = 0;
            g_rejected = 0;
            g_conn_for_write = DRACONIC_HOST_HANDLE_INVALID;
            g_accepted_conn = DRACONIC_HOST_HANDLE_INVALID;

            err = draconic_rt_host_tcp_listen(0, 8, &listen);
            if (err != DRACONIC_HOST_OK) return 1;
            err = draconic_rt_host_tcp_local_port(listen, &port);
            if (err != DRACONIC_HOST_OK) return 2;

            pa = draconic_rt_host_tcp_accept_async(listen);
            if (!pa) return 3;
            (void)draconic_rt_promise_then(pa, on_accept, NULL, NULL, NULL);

            pc = draconic_rt_host_tcp_connect_async("127.0.0.1", port);
            if (!pc) return 4;
            (void)draconic_rt_promise_then(pc, on_connect, NULL, NULL, NULL);

            draconic_rt_job_drain();
            if (g_accepted != 1) return 5;
            if (g_connected != 1) return 6;
            if (g_nwrite != 2) return 7;
            if (g_nread != 2) return 8;

            (void)draconic_rt_host_handle_close(listen);

            /* Cancel: pending accept rejected on close */
            err = draconic_rt_host_tcp_listen(0, 2, &listen2);
            if (err != DRACONIC_HOST_OK) return 9;
            pa = draconic_rt_host_tcp_accept_async(listen2);
            (void)draconic_rt_promise_then(pa, NULL, NULL, on_reject, NULL);
            err = draconic_rt_host_handle_close(listen2);
            if (err != DRACONIC_HOST_OK) return 10;
            draconic_rt_job_drain();
            if (g_rejected != 1) return 11;

            puts("tcp-h0702-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for tcp H07.02 smoke");

    let output = Command::new(&bin).output().expect("run tcp h0702");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "tcp H07.02 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "tcp-h0702-ok\n");
}

/// H07.03: concurrent connections + timer; job queue not starved by multi I/O.
#[test]
fn host_tcp_async_concurrent_does_not_starve_job_queue() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_tcp_h0703.c");
    let bin = dir.join("rt_host_tcp_h0703");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <stdlib.h>
        #include <string.h>

        #define NCONN 4

        static int g_nwrite[NCONN];
        static int g_nread[NCONN];
        static int g_timer_fired;
        static int g_writes_done;
        static int g_reads_done;
        static DraconicHostHandle g_clients[NCONN];
        static DraconicHostHandle g_accepted[NCONN];

        static void on_timer(void *data) {
            (void)data;
            g_timer_fired = 1;
        }

        static void *on_write(void *data, void *value) {
            int i = (int)(intptr_t)data;
            g_nwrite[i] = (int)(intptr_t)value;
            g_writes_done++;
            return value;
        }

        static void *on_read(void *data, void *value) {
            int i = (int)(intptr_t)data;
            g_nread[i] = (int)(intptr_t)value;
            g_reads_done++;
            return value;
        }

        int main(void) {
            DraconicHostError err;
            DraconicHostHandle listen = DRACONIC_HOST_HANDLE_INVALID;
            int32_t port = 0;
            const char *payloads[NCONN] = { "aa", "bb", "cc", "dd" };

            g_timer_fired = 0;
            g_writes_done = 0;
            g_reads_done = 0;
            for (int i = 0; i < NCONN; i++) {
                g_nwrite[i] = 0;
                g_nread[i] = 0;
                g_clients[i] = DRACONIC_HOST_HANDLE_INVALID;
                g_accepted[i] = DRACONIC_HOST_HANDLE_INVALID;
            }

            err = draconic_rt_host_tcp_listen(0, 16, &listen);
            if (err != DRACONIC_HOST_OK) return 1;
            err = draconic_rt_host_tcp_local_port(listen, &port);
            if (err != DRACONIC_HOST_OK) return 2;

            for (int i = 0; i < NCONN; i++) {
                err = draconic_rt_host_tcp_connect("127.0.0.1", port, &g_clients[i]);
                if (err != DRACONIC_HOST_OK) return 10 + i;
                err = draconic_rt_host_tcp_accept(listen, &g_accepted[i]);
                if (err != DRACONIC_HOST_OK) return 20 + i;
            }

            /* Timer scheduled before async I/O; must fire during drain. */
            if (draconic_rt_timer_set(on_timer, NULL, 5.0) <= 0) return 3;

            for (int i = 0; i < NCONN; i++) {
                DraconicValue *wp = draconic_rt_host_tcp_write_async(
                    g_clients[i],
                    (const uint8_t *)payloads[i],
                    2);
                DraconicValue *rp = draconic_rt_host_tcp_read_async(g_accepted[i], 8);
                if (!wp || !rp) return 30 + i;
                (void)draconic_rt_promise_then(
                    wp, on_write, (void *)(intptr_t)i, NULL, NULL);
                (void)draconic_rt_promise_then(
                    rp, on_read, (void *)(intptr_t)i, NULL, NULL);
            }

            draconic_rt_job_drain();

            if (g_timer_fired != 1) return 4;
            if (g_writes_done != NCONN) return 5;
            if (g_reads_done != NCONN) return 6;
            for (int i = 0; i < NCONN; i++) {
                if (g_nwrite[i] != 2) return 40 + i;
                if (g_nread[i] != 2) return 50 + i;
            }

            for (int i = 0; i < NCONN; i++) {
                (void)draconic_rt_host_handle_close(g_clients[i]);
                (void)draconic_rt_host_handle_close(g_accepted[i]);
            }
            (void)draconic_rt_host_handle_close(listen);

            puts("tcp-h0703-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for tcp H07.03 smoke");

    let output = Command::new(&bin).output().expect("run tcp h0703");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "tcp H07.03 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "tcp-h0703-ok\n");
}

#[test]
fn host_udp_bind_sendto_recvfrom_close() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_udp_h0801.c");
    let bin = dir.join("rt_host_udp_h0801");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <string.h>
        #include <stdlib.h>

        int main(void) {
            DraconicHostError err;
            DraconicHostHandle a = DRACONIC_HOST_HANDLE_INVALID;
            DraconicHostHandle b = DRACONIC_HOST_HANDLE_INVALID;
            int32_t port = 0;
            uint8_t *data = NULL;
            size_t len = 0;
            char *peer = NULL;
            int32_t peer_port = 0;

            err = draconic_rt_host_udp_bind(0, &a);
            if (err != DRACONIC_HOST_OK) return 1;
            if (!draconic_rt_host_handle_is_valid(a)) return 2;

            err = draconic_rt_host_udp_local_port(a, &port);
            if (err != DRACONIC_HOST_OK) return 3;
            if (port <= 0 || port > 65535) return 4;

            err = draconic_rt_host_udp_bind(0, &b);
            if (err != DRACONIC_HOST_OK) return 5;

            err = draconic_rt_host_udp_sendto(
                b, (const uint8_t *)"udp-hi", 6, "127.0.0.1", port);
            if (err != DRACONIC_HOST_OK) return 6;

            err = draconic_rt_host_udp_recvfrom(
                a, 64, &data, &len, &peer, &peer_port);
            if (err != DRACONIC_HOST_OK) return 7;
            if (len != 6 || data == NULL) return 8;
            if (memcmp(data, "udp-hi", 6) != 0) return 9;
            if (peer == NULL || strcmp(peer, "127.0.0.1") != 0) return 10;
            if (peer_port <= 0) return 11;
            free(data);
            free(peer);

            err = draconic_rt_host_handle_close(a);
            if (err != DRACONIC_HOST_OK) return 12;
            err = draconic_rt_host_handle_close(b);
            if (err != DRACONIC_HOST_OK) return 13;

            err = draconic_rt_host_udp_bind(-1, &a);
            if (err != DRACONIC_HOST_E_INVAL) return 14;
            err = draconic_rt_host_udp_bind(70000, &a);
            if (err != DRACONIC_HOST_E_INVAL) return 15;
            err = draconic_rt_host_udp_local_port(DRACONIC_HOST_HANDLE_INVALID, &port);
            if (err != DRACONIC_HOST_E_BADF) return 16;

            puts("udp-h0801-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for udp H08.01 smoke");

    let output = Command::new(&bin).output().expect("run udp h0801");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "udp H08.01 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "udp-h0801-ok\n");
}

#[test]
fn host_udp_loopback_echo_e2e() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_udp_h0802.c");
    let bin = dir.join("rt_host_udp_h0802");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <string.h>
        #include <stdlib.h>

        int main(void) {
            DraconicHostError err;
            DraconicHostHandle a = DRACONIC_HOST_HANDLE_INVALID;
            DraconicHostHandle b = DRACONIC_HOST_HANDLE_INVALID;
            int32_t pa = 0;
            int32_t pb = 0;
            uint8_t *req = NULL;
            size_t req_len = 0;
            uint8_t *res = NULL;
            size_t res_len = 0;
            char *peer = NULL;
            int32_t peer_port = 0;

            err = draconic_rt_host_udp_bind(0, &a);
            if (err != DRACONIC_HOST_OK) return 1;
            err = draconic_rt_host_udp_local_port(a, &pa);
            if (err != DRACONIC_HOST_OK) return 2;
            err = draconic_rt_host_udp_bind(0, &b);
            if (err != DRACONIC_HOST_OK) return 3;
            err = draconic_rt_host_udp_local_port(b, &pb);
            if (err != DRACONIC_HOST_OK) return 4;

            err = draconic_rt_host_udp_sendto(
                a, (const uint8_t *)"echo-me", 7, "127.0.0.1", pb);
            if (err != DRACONIC_HOST_OK) return 5;

            err = draconic_rt_host_udp_recvfrom(
                b, 64, &req, &req_len, &peer, &peer_port);
            if (err != DRACONIC_HOST_OK) return 6;
            if (req_len != 7 || req == NULL || memcmp(req, "echo-me", 7) != 0) return 7;
            free(peer);
            peer = NULL;

            err = draconic_rt_host_udp_sendto(
                b, req, req_len, "127.0.0.1", pa);
            if (err != DRACONIC_HOST_OK) return 8;
            free(req);
            req = NULL;

            err = draconic_rt_host_udp_recvfrom(
                a, 64, &res, &res_len, &peer, &peer_port);
            if (err != DRACONIC_HOST_OK) return 9;
            if (res_len != 7 || res == NULL || memcmp(res, "echo-me", 7) != 0) return 10;
            free(res);
            free(peer);

            err = draconic_rt_host_handle_close(a);
            if (err != DRACONIC_HOST_OK) return 11;
            err = draconic_rt_host_handle_close(b);
            if (err != DRACONIC_HOST_OK) return 12;

            puts("udp-h0802-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for udp H08.02 smoke");

    let output = Command::new(&bin).output().expect("run udp h0802");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "udp H08.02 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "udp-h0802-ok\n");
}

#[test]
fn host_dns_lookup_loopback_and_failure() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_dns_h0901.c");
    let bin = dir.join("rt_host_dns_h0901");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <string.h>
        #include <stdlib.h>

        int main(void) {
            DraconicHostError err;
            char **addrs = NULL;
            int64_t count = 0;
            int found = 0;
            int64_t i;

            err = draconic_rt_host_dns_lookup("127.0.0.1", &addrs, &count);
            if (err != DRACONIC_HOST_OK) return 1;
            if (count < 1 || addrs == NULL) return 2;
            for (i = 0; i < count; i++) {
                if (addrs[i] && strcmp(addrs[i], "127.0.0.1") == 0) found = 1;
                free(addrs[i]);
            }
            free(addrs);
            addrs = NULL;
            if (!found) return 3;

            err = draconic_rt_host_dns_lookup(
                "this-host-definitely-does-not-exist.invalid", &addrs, &count);
            if (err != DRACONIC_HOST_E_ADDR) return 4;
            if (addrs != NULL || count != 0) return 5;

            err = draconic_rt_host_dns_lookup("", &addrs, &count);
            if (err != DRACONIC_HOST_E_INVAL) return 6;
            err = draconic_rt_host_dns_lookup(NULL, &addrs, &count);
            if (err != DRACONIC_HOST_E_INVAL) return 7;
            err = draconic_rt_host_dns_lookup("127.0.0.1", NULL, &count);
            if (err != DRACONIC_HOST_E_INVAL) return 8;

            puts("dns-h0901-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for dns H09.01 smoke");

    let output = Command::new(&bin).output().expect("run dns h0901");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "dns H09.01 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "dns-h0901-ok\n");
}

#[test]
fn host_tcp_connect_by_name_localhost() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_tcp_h0902.c");
    let bin = dir.join("rt_host_tcp_h0902");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>

        int main(void) {
            DraconicHostError err;
            DraconicHostHandle listen_h = DRACONIC_HOST_HANDLE_INVALID;
            DraconicHostHandle client_h = DRACONIC_HOST_HANDLE_INVALID;
            int32_t port = 0;

            err = draconic_rt_host_tcp_listen(0, 8, &listen_h);
            if (err != DRACONIC_HOST_OK) return 1;
            err = draconic_rt_host_tcp_local_port(listen_h, &port);
            if (err != DRACONIC_HOST_OK) return 2;

            /* H09.02: dial by hostname, not dotted IPv4. */
            err = draconic_rt_host_tcp_connect("localhost", port, &client_h);
            if (err != DRACONIC_HOST_OK) return 3;
            if (!draconic_rt_host_handle_is_valid(client_h)) return 4;
            err = draconic_rt_host_handle_close(client_h);
            if (err != DRACONIC_HOST_OK) return 5;
            err = draconic_rt_host_handle_close(listen_h);
            if (err != DRACONIC_HOST_OK) return 6;

            err = draconic_rt_host_tcp_connect(
                "this-host-definitely-does-not-exist.invalid", 80, &client_h);
            if (err != DRACONIC_HOST_E_ADDR) return 7;

            puts("tcp-h0902-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for tcp H09.02 smoke");

    let output = Command::new(&bin).output().expect("run tcp h0902");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "tcp H09.02 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "tcp-h0902-ok\n");
}

#[test]
fn host_http_parse_request_line_headers_body() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_http_h1001.c");
    let bin = dir.join("rt_host_http_h1001");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <string.h>
        #include <stdlib.h>

        int main(void) {
            const char *raw =
                "POST /echo HTTP/1.1\r\n"
                "Host: example.com\r\n"
                "Content-Length: 5\r\n"
                "\r\n"
                "helloEXTRA";
            DraconicHostError err;
            char *method = NULL;
            char *path = NULL;
            char *version = NULL;
            char *body = NULL;
            char *host = NULL;
            char *missing = NULL;
            char *cl = NULL;

            err = draconic_rt_host_http_parse_request(
                (const uint8_t *)raw, strlen(raw),
                &method, &path, &version, &body);
            if (err != DRACONIC_HOST_OK) return 1;
            if (!method || strcmp(method, "POST") != 0) return 2;
            if (!path || strcmp(path, "/echo") != 0) return 3;
            if (!version || strcmp(version, "HTTP/1.1") != 0) return 4;
            if (!body || strcmp(body, "hello") != 0) return 5;

            err = draconic_rt_host_http_request_header(
                (const uint8_t *)raw, strlen(raw), "Host", &host);
            if (err != DRACONIC_HOST_OK) return 6;
            if (!host || strcmp(host, "example.com") != 0) return 7;

            err = draconic_rt_host_http_request_header(
                (const uint8_t *)raw, strlen(raw), "content-length", &cl);
            if (err != DRACONIC_HOST_OK) return 8;
            if (!cl || strcmp(cl, "5") != 0) return 9;

            err = draconic_rt_host_http_request_header(
                (const uint8_t *)raw, strlen(raw), "X-Missing", &missing);
            if (err != DRACONIC_HOST_OK) return 10;
            if (!missing || missing[0] != '\0') return 11;

            free(method); free(path); free(version); free(body);
            free(host); free(cl); free(missing);

            /* GET no body */
            {
                const char *get =
                    "GET / HTTP/1.1\r\nHost: x\r\n\r\n";
                method = path = version = body = NULL;
                err = draconic_rt_host_http_parse_request(
                    (const uint8_t *)get, strlen(get),
                    &method, &path, &version, &body);
                if (err != DRACONIC_HOST_OK) return 12;
                if (strcmp(method, "GET") != 0) return 13;
                if (strcmp(path, "/") != 0) return 14;
                if (strcmp(body, "") != 0) return 15;
                free(method); free(path); free(version); free(body);
            }

            /* malformed */
            err = draconic_rt_host_http_parse_request(
                (const uint8_t *)"not-http", 8,
                &method, &path, &version, &body);
            if (err != DRACONIC_HOST_E_INVAL) return 16;

            puts("http-h1001-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for http H10.01 smoke");

    let output = Command::new(&bin).output().expect("run http h1001");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "http H10.01 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "http-h1001-ok\n");
}

#[test]
fn host_http_write_response_status_headers_body() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_http_h1002.c");
    let bin = dir.join("rt_host_http_h1002");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <string.h>
        #include <stdlib.h>

        int main(void) {
            DraconicHostError err;
            char *msg = NULL;
            const char *want =
                "HTTP/1.1 200 OK\r\n"
                "Content-Type: text/plain\r\n"
                "Content-Length: 5\r\n"
                "\r\n"
                "hello";

            err = draconic_rt_host_http_write_response(
                200, "OK", "Content-Type: text/plain\r\n",
                (const uint8_t *)"hello", 5, &msg);
            if (err != DRACONIC_HOST_OK) return 1;
            if (!msg || strcmp(msg, want) != 0) return 2;
            free(msg);
            msg = NULL;

            /* default reason + empty body */
            err = draconic_rt_host_http_write_response(
                404, "", "", NULL, 0, &msg);
            if (err != DRACONIC_HOST_OK) return 3;
            want = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
            if (!msg || strcmp(msg, want) != 0) return 4;
            free(msg);
            msg = NULL;

            /* existing Content-Length not duplicated */
            err = draconic_rt_host_http_write_response(
                200, "OK", "Content-Length: 3\r\n",
                (const uint8_t *)"abc", 3, &msg);
            if (err != DRACONIC_HOST_OK) return 5;
            want = "HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc";
            if (!msg || strcmp(msg, want) != 0) return 6;
            free(msg);
            msg = NULL;

            /* bad status */
            err = draconic_rt_host_http_write_response(
                99, "X", "", NULL, 0, &msg);
            if (err != DRACONIC_HOST_E_INVAL) return 7;
            if (msg) return 8;

            puts("http-h1002-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for http H10.02 smoke");

    let output = Command::new(&bin).output().expect("run http h1002");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "http H10.02 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "http-h1002-ok\n");
}

#[test]
fn host_http_client_write_request_parse_response() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_http_h1005.c");
    let bin = dir.join("rt_host_http_h1005");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <string.h>
        #include <stdlib.h>

        int main(void) {
            DraconicHostError err;
            char *msg = NULL;
            char *version = NULL;
            char *reason = NULL;
            char *body = NULL;
            char *ct = NULL;
            int32_t status = 0;
            const char *want_req =
                "GET /hello HTTP/1.1\r\n"
                "Host: x\r\n"
                "Content-Length: 0\r\n"
                "\r\n";
            const char *raw_res =
                "HTTP/1.1 200 OK\r\n"
                "Content-Type: text/plain\r\n"
                "Content-Length: 5\r\n"
                "\r\n"
                "hello";

            err = draconic_rt_host_http_write_request(
                "GET", "/hello", "Host: x\r\n", NULL, 0, &msg);
            if (err != DRACONIC_HOST_OK) return 1;
            if (!msg || strcmp(msg, want_req) != 0) return 2;
            free(msg);
            msg = NULL;

            err = draconic_rt_host_http_write_request(
                "POST", "/echo", "Host: x\r\n",
                (const uint8_t *)"hi", 2, &msg);
            if (err != DRACONIC_HOST_OK) return 3;
            want_req =
                "POST /echo HTTP/1.1\r\n"
                "Host: x\r\n"
                "Content-Length: 2\r\n"
                "\r\n"
                "hi";
            if (!msg || strcmp(msg, want_req) != 0) return 4;
            free(msg);
            msg = NULL;

            /* empty method → INVAL */
            err = draconic_rt_host_http_write_request(
                "", "/x", "", NULL, 0, &msg);
            if (err != DRACONIC_HOST_E_INVAL) return 5;

            err = draconic_rt_host_http_parse_response(
                (const uint8_t *)raw_res, strlen(raw_res),
                &version, &status, &reason, &body);
            if (err != DRACONIC_HOST_OK) return 6;
            if (!version || strcmp(version, "HTTP/1.1") != 0) return 7;
            if (status != 200) return 8;
            if (!reason || strcmp(reason, "OK") != 0) return 9;
            if (!body || strcmp(body, "hello") != 0) return 10;

            err = draconic_rt_host_http_response_header(
                (const uint8_t *)raw_res, strlen(raw_res), "content-type", &ct);
            if (err != DRACONIC_HOST_OK) return 11;
            if (!ct || strcmp(ct, "text/plain") != 0) return 12;

            free(version); free(reason); free(body); free(ct);
            version = reason = body = ct = NULL;

            /* malformed */
            err = draconic_rt_host_http_parse_response(
                (const uint8_t *)"nope", 4,
                &version, &status, &reason, &body);
            if (err != DRACONIC_HOST_E_INVAL) return 13;

            puts("http-h1005-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
        };
    assert!(status.success(), "clang failed for http H10.05 smoke");

    let output = Command::new(&bin).output().expect("run http h1005");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "http H10.05 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "http-h1005-ok\n");
}

#[test]
fn host_tls_client_wrap_insecure_against_openssl_s_server() {
    // H11.01: insecure TLS client wrap + read/write against openssl s_server.
    if !cfg!(target_os = "macos") {
        return;
    }
    let openssl = ["openssl", "/usr/bin/openssl", "/opt/homebrew/bin/openssl"]
        .iter()
        .map(std::path::PathBuf::from)
        .find(|p| {
            p.is_file()
                || Command::new(p)
                    .arg("version")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
        });
    let openssl = match openssl {
        Some(p) => p,
        None => return, // skip when openssl missing
    };

    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let cert = dir.join("cert.pem");
    let key = dir.join("key.pem");
    let gen = Command::new(&openssl)
        .args([
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-keyout",
        ])
        .arg(&key)
        .arg("-out")
        .arg(&cert)
        .args(["-days", "1", "-nodes", "-subj", "/CN=localhost"])
        .output()
        .expect("openssl req");
    assert!(
        gen.status.success(),
        "openssl req failed: {}",
        String::from_utf8_lossy(&gen.stderr)
    );

    // Ephemeral port via bind port 0 helper: pick free port in Rust.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let mut server = Command::new(&openssl)
        .args(["s_server", "-accept"])
        .arg(port.to_string())
        .arg("-cert")
        .arg(&cert)
        .arg("-key")
        .arg(&key)
        .arg("-www")
        .arg("-quiet")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn openssl s_server");

    // Wait until port accepts.
    for _ in 0..50 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let main_c = dir.join("main_tls.c");
    let bin = dir.join("rt_host_tls");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    std::fs::write(
        &main_c,
        format!(
            r#"
#include "draconic_rt_host.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
int main(void) {{
    DraconicHostHandle tcp = DRACONIC_HOST_HANDLE_INVALID;
    DraconicHostHandle tls = DRACONIC_HOST_HANDLE_INVALID;
    DraconicHostError err;
    uint8_t *data = NULL;
    size_t len = 0;
    const char *req = "GET / HTTP/1.0\r\nHost: localhost\r\n\r\n";
    err = draconic_rt_host_tcp_connect("127.0.0.1", {port}, &tcp);
    if (err != DRACONIC_HOST_OK) {{ fprintf(stderr, "connect %d\n", err); return 1; }}
    err = draconic_rt_host_tls_client_wrap(tcp, "localhost", 1, &tls);
    if (err != DRACONIC_HOST_OK) {{ fprintf(stderr, "wrap %d\n", err); return 2; }}
    err = draconic_rt_host_tls_write(tls, (const uint8_t *)req, strlen(req));
    if (err != DRACONIC_HOST_OK) {{ fprintf(stderr, "write %d\n", err); return 3; }}
    err = draconic_rt_host_tls_read(tls, 4096, &data, &len);
    if (err != DRACONIC_HOST_OK) {{ fprintf(stderr, "read %d\n", err); return 4; }}
    if (!data || len < 4) {{ fprintf(stderr, "empty\n"); return 5; }}
    fwrite(data, 1, len, stdout);
    free(data);
    (void)draconic_rt_host_handle_close(tls);
    printf("\nTLS-OK\n");
    return 0;
}}
"#,
            port = port
        ),
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg(format!("-I{}", header_dir.display()))
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("clang link tls")
    };
    assert!(status.success(), "link tls client failed");

    let out = Command::new(&bin).output().expect("run tls client");
    let _ = server.kill();
    let _ = server.wait();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "tls client failed: {:?}\nstdout={stdout}\nstderr={stderr}",
        out.status
    );
    assert!(
        stdout.contains("HTTP") || stdout.contains("TLS-OK"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("TLS-OK"), "stdout={stdout}");
}

#[test]
fn host_tls_server_wrap_loopback_echo() {
    // H11.02: TLS server wrap + client wrap insecure on loopback echo.
    if !cfg!(target_os = "macos") {
        return;
    }
    let openssl = ["openssl", "/usr/bin/openssl", "/opt/homebrew/bin/openssl"]
        .iter()
        .map(std::path::PathBuf::from)
        .find(|p| {
            p.is_file()
                || Command::new(p)
                    .arg("version")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
        });
    let openssl = match openssl {
        Some(p) => p,
        None => return,
    };

    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let cert = dir.join("cert.pem");
    let key = dir.join("key.pem");
    let gen = Command::new(&openssl)
        .args(["req", "-x509", "-newkey", "rsa:2048", "-keyout"])
        .arg(&key)
        .arg("-out")
        .arg(&cert)
        .args(["-days", "1", "-nodes", "-subj", "/CN=localhost"])
        .output()
        .expect("openssl req");
    assert!(
        gen.status.success(),
        "openssl req failed: {}",
        String::from_utf8_lossy(&gen.stderr)
    );

    let main_c = dir.join("main_tls_server.c");
    let bin = dir.join("rt_host_tls_server");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    let cert_c = cert.display().to_string().replace('\\', "\\\\");
    let key_c = key.display().to_string().replace('\\', "\\\\");
    std::fs::write(
        &main_c,
        format!(
            r#"
#include "draconic_rt_host.h"
#include <pthread.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <unistd.h>

typedef struct {{
    int port;
    const char *cert;
    const char *key;
    int ready;
    int ok;
}} ServerArgs;

static void *server_thread(void *arg) {{
    ServerArgs *sa = (ServerArgs *)arg;
    DraconicHostHandle listen = DRACONIC_HOST_HANDLE_INVALID;
    DraconicHostHandle acc = DRACONIC_HOST_HANDLE_INVALID;
    DraconicHostHandle tls = DRACONIC_HOST_HANDLE_INVALID;
    DraconicHostError err;
    uint8_t *data = NULL;
    size_t len = 0;
    int32_t port = 0;
    err = draconic_rt_host_tcp_listen(0, 16, &listen);
    if (err != DRACONIC_HOST_OK) {{ sa->ok = 0; return NULL; }}
    err = draconic_rt_host_tcp_local_port(listen, &port);
    if (err != DRACONIC_HOST_OK) {{ sa->ok = 0; return NULL; }}
    sa->port = (int)port;
    __sync_synchronize();
    sa->ready = 1;
    err = draconic_rt_host_tcp_accept(listen, &acc);
    if (err != DRACONIC_HOST_OK) {{ sa->ok = 0; return NULL; }}
    err = draconic_rt_host_tls_server_wrap(acc, sa->cert, sa->key, &tls);
    if (err != DRACONIC_HOST_OK) {{
        fprintf(stderr, "server wrap %d\n", err);
        sa->ok = 0;
        return NULL;
    }}
    err = draconic_rt_host_tls_read(tls, 4096, &data, &len);
    if (err != DRACONIC_HOST_OK || !data || len == 0) {{
        fprintf(stderr, "server read %d len=%zu\n", err, len);
        sa->ok = 0;
        return NULL;
    }}
    err = draconic_rt_host_tls_write(tls, data, len);
    free(data);
    if (err != DRACONIC_HOST_OK) {{ sa->ok = 0; return NULL; }}
    (void)draconic_rt_host_handle_close(tls);
    (void)draconic_rt_host_handle_close(listen);
    sa->ok = 1;
    return NULL;
}}

int main(void) {{
    ServerArgs sa;
    pthread_t th;
    DraconicHostHandle tcp = DRACONIC_HOST_HANDLE_INVALID;
    DraconicHostHandle tls = DRACONIC_HOST_HANDLE_INVALID;
    DraconicHostError err;
    uint8_t *data = NULL;
    size_t len = 0;
    const char *msg = "ping-h1102";
    int i;
    memset(&sa, 0, sizeof(sa));
    sa.cert = "{cert}";
    sa.key = "{key}";
    if (pthread_create(&th, NULL, server_thread, &sa) != 0) {{
        fprintf(stderr, "pthread\n");
        return 1;
    }}
    for (i = 0; i < 100 && !sa.ready; i++) {{
        usleep(10000);
    }}
    if (!sa.ready) {{
        fprintf(stderr, "server not ready\n");
        return 2;
    }}
    err = draconic_rt_host_tcp_connect("127.0.0.1", sa.port, &tcp);
    if (err != DRACONIC_HOST_OK) {{ fprintf(stderr, "connect %d\n", err); return 3; }}
    err = draconic_rt_host_tls_client_wrap(tcp, "localhost", 1, &tls);
    if (err != DRACONIC_HOST_OK) {{ fprintf(stderr, "client wrap %d\n", err); return 4; }}
    err = draconic_rt_host_tls_write(tls, (const uint8_t *)msg, strlen(msg));
    if (err != DRACONIC_HOST_OK) {{ fprintf(stderr, "write %d\n", err); return 5; }}
    err = draconic_rt_host_tls_read(tls, 4096, &data, &len);
    if (err != DRACONIC_HOST_OK) {{ fprintf(stderr, "read %d\n", err); return 6; }}
    if (!data || len != strlen(msg) || memcmp(data, msg, len) != 0) {{
        fprintf(stderr, "echo mismatch len=%zu\n", len);
        return 7;
    }}
    free(data);
    (void)draconic_rt_host_handle_close(tls);
    pthread_join(th, NULL);
    if (!sa.ok) {{
        fprintf(stderr, "server failed\n");
        return 8;
    }}
    printf("TLS-SERVER-OK\n");
    return 0;
}}
"#,
            cert = cert_c,
            key = key_c
        ),
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg(format!("-I{}", header_dir.display()))
            .arg("-o")
            .arg(&bin)
            .arg("-lpthread");
        apply_runtime_link_flags(&mut link);
        link.status().expect("clang link tls server")
    };
    assert!(status.success(), "link tls server failed");

    let out = Command::new(&bin).output().expect("run tls server");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "tls server failed: {:?}\nstdout={stdout}\nstderr={stderr}",
        out.status
    );
    assert!(stdout.contains("TLS-SERVER-OK"), "stdout={stdout}");
}

#[test]
fn host_https_http11_loopback() {
    // H11.03: HTTP/1.1 request/response over TLS on loopback (server + client).
    if !cfg!(target_os = "macos") {
        return;
    }
    let openssl = ["openssl", "/usr/bin/openssl", "/opt/homebrew/bin/openssl"]
        .iter()
        .map(std::path::PathBuf::from)
        .find(|p| {
            p.is_file()
                || Command::new(p)
                    .arg("version")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
        });
    let openssl = match openssl {
        Some(p) => p,
        None => return,
    };

    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let cert = dir.join("cert.pem");
    let key = dir.join("key.pem");
    let gen = Command::new(&openssl)
        .args(["req", "-x509", "-newkey", "rsa:2048", "-keyout"])
        .arg(&key)
        .arg("-out")
        .arg(&cert)
        .args(["-days", "1", "-nodes", "-subj", "/CN=localhost"])
        .output()
        .expect("openssl req");
    assert!(
        gen.status.success(),
        "openssl req failed: {}",
        String::from_utf8_lossy(&gen.stderr)
    );

    let main_c = dir.join("main_https.c");
    let bin = dir.join("rt_host_https");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    let cert_c = cert.display().to_string().replace('\\', "\\\\");
    let key_c = key.display().to_string().replace('\\', "\\\\");
    std::fs::write(
        &main_c,
        format!(
            r#"
#include "draconic_rt_host.h"
#include <pthread.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <unistd.h>

typedef struct {{
    int port;
    const char *cert;
    const char *key;
    int ready;
    int ok;
}} ServerArgs;

static void *server_thread(void *arg) {{
    ServerArgs *sa = (ServerArgs *)arg;
    DraconicHostHandle listen = DRACONIC_HOST_HANDLE_INVALID;
    DraconicHostHandle acc = DRACONIC_HOST_HANDLE_INVALID;
    DraconicHostHandle tls = DRACONIC_HOST_HANDLE_INVALID;
    DraconicHostError err;
    uint8_t *data = NULL;
    size_t len = 0;
    int32_t port = 0;
    char *method = NULL;
    char *path = NULL;
    char *version = NULL;
    char *body = NULL;
    char *resp = NULL;
    err = draconic_rt_host_tcp_listen(0, 16, &listen);
    if (err != DRACONIC_HOST_OK) {{ sa->ok = 0; return NULL; }}
    err = draconic_rt_host_tcp_local_port(listen, &port);
    if (err != DRACONIC_HOST_OK) {{ sa->ok = 0; return NULL; }}
    sa->port = (int)port;
    __sync_synchronize();
    sa->ready = 1;
    err = draconic_rt_host_tcp_accept(listen, &acc);
    if (err != DRACONIC_HOST_OK) {{ sa->ok = 0; return NULL; }}
    err = draconic_rt_host_tls_server_wrap(acc, sa->cert, sa->key, &tls);
    if (err != DRACONIC_HOST_OK) {{
        fprintf(stderr, "server wrap %d\n", err);
        sa->ok = 0;
        return NULL;
    }}
    err = draconic_rt_host_tls_read(tls, 4096, &data, &len);
    if (err != DRACONIC_HOST_OK || !data || len == 0) {{
        fprintf(stderr, "server read %d len=%zu\n", err, len);
        sa->ok = 0;
        return NULL;
    }}
    err = draconic_rt_host_http_parse_request(
        data, len, &method, &path, &version, &body);
    free(data);
    if (err != DRACONIC_HOST_OK) {{
        fprintf(stderr, "parse req %d\n", err);
        sa->ok = 0;
        return NULL;
    }}
    err = draconic_rt_host_http_write_response(
        200, "OK", "Content-Type: text/plain\r\n",
        (const uint8_t *)path, path ? strlen(path) : 0, &resp);
    free(method); free(path); free(version); free(body);
    if (err != DRACONIC_HOST_OK || !resp) {{
        fprintf(stderr, "write resp %d\n", err);
        sa->ok = 0;
        return NULL;
    }}
    err = draconic_rt_host_tls_write(tls, (const uint8_t *)resp, strlen(resp));
    free(resp);
    if (err != DRACONIC_HOST_OK) {{ sa->ok = 0; return NULL; }}
    (void)draconic_rt_host_handle_close(tls);
    (void)draconic_rt_host_handle_close(listen);
    sa->ok = 1;
    return NULL;
}}

int main(void) {{
    ServerArgs sa;
    pthread_t th;
    DraconicHostHandle tcp = DRACONIC_HOST_HANDLE_INVALID;
    DraconicHostHandle tls = DRACONIC_HOST_HANDLE_INVALID;
    DraconicHostError err;
    uint8_t *data = NULL;
    size_t len = 0;
    char *req = NULL;
    char *version = NULL;
    char *reason = NULL;
    char *body = NULL;
    int32_t status = 0;
    int i;
    memset(&sa, 0, sizeof(sa));
    sa.cert = "{cert}";
    sa.key = "{key}";
    if (pthread_create(&th, NULL, server_thread, &sa) != 0) {{
        fprintf(stderr, "pthread\n");
        return 1;
    }}
    for (i = 0; i < 100 && !sa.ready; i++) {{
        usleep(10000);
    }}
    if (!sa.ready) {{
        fprintf(stderr, "server not ready\n");
        return 2;
    }}
    err = draconic_rt_host_tcp_connect("127.0.0.1", sa.port, &tcp);
    if (err != DRACONIC_HOST_OK) {{ fprintf(stderr, "connect %d\n", err); return 3; }}
    err = draconic_rt_host_tls_client_wrap(tcp, "localhost", 1, &tls);
    if (err != DRACONIC_HOST_OK) {{ fprintf(stderr, "client wrap %d\n", err); return 4; }}
    err = draconic_rt_host_http_write_request(
        "GET", "/hello", "Host: localhost\r\n", NULL, 0, &req);
    if (err != DRACONIC_HOST_OK || !req) {{ fprintf(stderr, "write req %d\n", err); return 5; }}
    err = draconic_rt_host_tls_write(tls, (const uint8_t *)req, strlen(req));
    free(req);
    if (err != DRACONIC_HOST_OK) {{ fprintf(stderr, "tls write %d\n", err); return 6; }}
    err = draconic_rt_host_tls_read(tls, 4096, &data, &len);
    if (err != DRACONIC_HOST_OK) {{ fprintf(stderr, "tls read %d\n", err); return 7; }}
    err = draconic_rt_host_http_parse_response(
        data, len, &version, &status, &reason, &body);
    free(data);
    if (err != DRACONIC_HOST_OK) {{ fprintf(stderr, "parse res %d\n", err); return 8; }}
    if (!version || strcmp(version, "HTTP/1.1") != 0) {{
        fprintf(stderr, "version\n");
        return 9;
    }}
    if (status != 200) {{ fprintf(stderr, "status %d\n", status); return 10; }}
    if (!reason || strcmp(reason, "OK") != 0) {{ fprintf(stderr, "reason\n"); return 11; }}
    if (!body || strcmp(body, "/hello") != 0) {{
        fprintf(stderr, "body=%s\n", body ? body : "(null)");
        return 12;
    }}
    free(version); free(reason); free(body);
    (void)draconic_rt_host_handle_close(tls);
    pthread_join(th, NULL);
    if (!sa.ok) {{
        fprintf(stderr, "server failed\n");
        return 13;
    }}
    printf("HTTPS-OK\n");
    return 0;
}}
"#,
            cert = cert_c,
            key = key_c
        ),
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg(format!("-I{}", header_dir.display()))
            .arg("-o")
            .arg(&bin)
            .arg("-lpthread");
        apply_runtime_link_flags(&mut link);
        link.status().expect("clang link https")
    };
    assert!(status.success(), "link https failed");

    let out = Command::new(&bin).output().expect("run https");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "https failed: {:?}\nstdout={stdout}\nstderr={stderr}",
        out.status
    );
    assert!(stdout.contains("HTTPS-OK"), "stdout={stdout}");
}
