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

    // H00: language HostError.code / native stderr token for each ABI failure.
    assert_eq!(host_error_name(HOST_OK), None);
    assert_eq!(host_error_name(HOST_E_INVAL), Some("EINVAL"));
    assert_eq!(host_error_name(HOST_E_NOENT), Some("ENOENT"));
    assert_eq!(host_error_name(HOST_E_NOSYS), Some("ENOSYS"));
    assert_eq!(host_error_name(HOST_E_BADF), Some("EBADF"));
    assert_eq!(host_error_name(HOST_E_EXIST), Some("EEXIST"));
    assert_eq!(host_error_name(HOST_E_PERM), Some("EPERM"));
    assert_eq!(host_error_name(HOST_E_IO), Some("EIO"));
    assert_eq!(host_error_name(HOST_E_NOMEM), Some("ENOMEM"));
    assert_eq!(host_error_name(HOST_E_AGAIN), Some("EAGAIN"));
    assert_eq!(host_error_name(HOST_E_CONN), Some("ECONN"));
    assert_eq!(host_error_name(HOST_E_ADDR), Some("EADDR"));
    assert_eq!(host_error_name(99), None);

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
        HOST_WS_HANDSHAKE_RESPONSE.declare(),
        "declare i32 @draconic_rt_host_ws_handshake_response(ptr, ptr)"
    );
    assert_eq!(
        HOST_WS_ENCODE_TEXT.declare(),
        "declare i32 @draconic_rt_host_ws_encode_text(ptr, ptr, ptr)"
    );
    assert_eq!(
        HOST_WS_ENCODE_BINARY.declare(),
        "declare i32 @draconic_rt_host_ws_encode_binary(ptr, i64, ptr, ptr)"
    );
    assert_eq!(
        HOST_WS_ENCODE_CLOSE.declare(),
        "declare i32 @draconic_rt_host_ws_encode_close(i32, ptr, ptr, ptr)"
    );
    assert_eq!(
        HOST_WS_ENCODE_PING.declare(),
        "declare i32 @draconic_rt_host_ws_encode_ping(ptr, ptr, ptr)"
    );
    assert_eq!(
        HOST_WS_ENCODE_PONG.declare(),
        "declare i32 @draconic_rt_host_ws_encode_pong(ptr, ptr, ptr)"
    );
    assert_eq!(
        HOST_WS_DECODE_FRAME.declare(),
        "declare i32 @draconic_rt_host_ws_decode_frame(ptr, i64, ptr, ptr, ptr, ptr, ptr)"
    );
    assert_eq!(
        HOST_WS_CLIENT_HANDSHAKE_REQUEST.declare(),
        "declare i32 @draconic_rt_host_ws_client_handshake_request(ptr, ptr, ptr, ptr)"
    );
    assert_eq!(
        HOST_WS_CLIENT_CHECK_ACCEPT.declare(),
        "declare i32 @draconic_rt_host_ws_client_check_accept(ptr, i64, ptr)"
    );
    assert_eq!(
        HOST_WS_ENCODE_TEXT_CLIENT.declare(),
        "declare i32 @draconic_rt_host_ws_encode_text_client(ptr, ptr, ptr)"
    );
    assert_eq!(
        HOST_HTTP2_CLIENT_PREFACE.declare(),
        "declare i32 @draconic_rt_host_http2_client_preface(ptr, ptr)"
    );
    assert_eq!(
        HOST_HTTP2_SERVER_PREFACE.declare(),
        "declare i32 @draconic_rt_host_http2_server_preface(ptr, ptr)"
    );
    assert_eq!(
        HOST_HTTP2_SETTINGS_ACK.declare(),
        "declare i32 @draconic_rt_host_http2_settings_ack(ptr, ptr)"
    );
    assert_eq!(
        HOST_HTTP2_ENCODE_REQUEST.declare(),
        "declare i32 @draconic_rt_host_http2_encode_request(ptr, ptr, ptr, i64, ptr, ptr)"
    );
    assert_eq!(
        HOST_HTTP2_ENCODE_RESPONSE.declare(),
        "declare i32 @draconic_rt_host_http2_encode_response(i32, ptr, i64, ptr, ptr)"
    );
    assert_eq!(
        HOST_HTTP2_PARSE_REQUEST.declare(),
        "declare i32 @draconic_rt_host_http2_parse_request(ptr, i64, ptr, ptr, ptr, ptr, ptr)"
    );
    assert_eq!(
        HOST_HTTP2_PARSE_RESPONSE.declare(),
        "declare i32 @draconic_rt_host_http2_parse_response(ptr, i64, ptr, ptr, ptr, ptr)"
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
    assert_eq!(
        HOST_WORKER_SPAWN.declare(),
        "declare i32 @draconic_rt_host_worker_spawn(i32, ptr)"
    );
    assert_eq!(
        HOST_WORKER_JOIN.declare(),
        "declare i32 @draconic_rt_host_worker_join(i32)"
    );
    assert_eq!(
        HOST_WORKER_TERMINATE.declare(),
        "declare i32 @draconic_rt_host_worker_terminate(i32)"
    );
    assert_eq!(
        HOST_WORKER_OS_THREAD.declare(),
        "declare i32 @draconic_rt_host_worker_os_thread(i32)"
    );
    assert_eq!(
        HOST_CHANNEL_MAKE.declare(),
        "declare i32 @draconic_rt_host_channel_make(i32)"
    );
    assert_eq!(
        HOST_CHANNEL_SEND_F64.declare(),
        "declare i32 @draconic_rt_host_channel_send_f64(i32, double)"
    );
    assert_eq!(
        HOST_CHANNEL_SEND_STR.declare(),
        "declare i32 @draconic_rt_host_channel_send_str(i32, ptr)"
    );
    assert_eq!(
        HOST_ONCE_MAKE.declare(),
        "declare i32 @draconic_rt_host_once_make()"
    );
    assert_eq!(
        HOST_ONCE_RUN.declare(),
        "declare i32 @draconic_rt_host_once_run(i32, ptr)"
    );
    assert_eq!(
        HOST_INTERNAL_MUTEX_MAKE.declare(),
        "declare i32 @draconic_rt_host_internal_mutex_make()"
    );
    assert_eq!(
        HOST_INTERNAL_MUTEX_LOCK.declare(),
        "declare i32 @draconic_rt_host_internal_mutex_lock(i32)"
    );
    assert_eq!(
        HOST_INTERNAL_MUTEX_UNLOCK.declare(),
        "declare i32 @draconic_rt_host_internal_mutex_unlock(i32)"
    );
    assert_eq!(
        HOST_CANCEL_MAKE.declare(),
        "declare i32 @draconic_rt_host_cancel_make()"
    );
    assert_eq!(
        HOST_CANCEL_ABORT.declare(),
        "declare i32 @draconic_rt_host_cancel_abort(i32)"
    );
    assert_eq!(
        HOST_CANCEL_ABORTED.declare(),
        "declare i32 @draconic_rt_host_cancel_aborted(i32)"
    );
    assert_eq!(
        HOST_CANCEL_LINK.declare(),
        "declare i32 @draconic_rt_host_cancel_link(i32, i32)"
    );
    assert_eq!(
        HOST_CANCEL_TIMEOUT.declare(),
        "declare i32 @draconic_rt_host_cancel_timeout(double)"
    );
    assert_eq!(
        HOST_CANCEL_CLEAR_TIMEOUT.declare(),
        "declare i32 @draconic_rt_host_cancel_clear_timeout(i32)"
    );
    assert_eq!(
        HOST_CHANNEL_SEND_BOOL.declare(),
        "declare i32 @draconic_rt_host_channel_send_bool(i32, i32)"
    );
    assert_eq!(
        HOST_CHANNEL_RECV_F64.declare(),
        "declare i32 @draconic_rt_host_channel_recv_f64(i32, ptr)"
    );
    assert_eq!(
        HOST_CHANNEL_RECV_STR.declare(),
        "declare i32 @draconic_rt_host_channel_recv_str(i32, ptr)"
    );
    assert_eq!(
        HOST_CHANNEL_RECV_BOOL.declare(),
        "declare i32 @draconic_rt_host_channel_recv_bool(i32, ptr)"
    );
    assert_eq!(
        HOST_CHANNEL_SEND_OBJ.declare(),
        "declare i32 @draconic_rt_host_channel_send_obj(i32, ptr)"
    );
    assert_eq!(
        HOST_CHANNEL_RECV_OBJ.declare(),
        "declare i32 @draconic_rt_host_channel_recv_obj(i32, ptr)"
    );
    assert_eq!(
        HOST_SHARED_MAKE.declare(),
        "declare i32 @draconic_rt_host_shared_make(i32)"
    );
    assert_eq!(
        HOST_SHARED_LOAD.declare(),
        "declare i32 @draconic_rt_host_shared_load(i32, i32)"
    );
    assert_eq!(
        HOST_SHARED_STORE.declare(),
        "declare i32 @draconic_rt_host_shared_store(i32, i32, i32)"
    );
    assert_eq!(
        HOST_SHARED_ADD.declare(),
        "declare i32 @draconic_rt_host_shared_add(i32, i32, i32)"
    );
    assert_eq!(
        HOST_SHARED_CMPXCHG.declare(),
        "declare i32 @draconic_rt_host_shared_cmpxchg(i32, i32, i32, i32)"
    );
    assert_eq!(
        HOST_SHARED_WAIT.declare(),
        "declare i32 @draconic_rt_host_shared_wait(i32, i32, i32, double)"
    );
    assert_eq!(
        HOST_SHARED_NOTIFY.declare(),
        "declare i32 @draconic_rt_host_shared_notify(i32, i32)"
    );
    assert_eq!(
        HOST_SIGNAL_WATCH.declare(),
        "declare i32 @draconic_rt_host_signal_watch(i32, ptr, ptr)"
    );
    assert_eq!(
        HOST_SIGNAL_IGNORE.declare(),
        "declare i32 @draconic_rt_host_signal_ignore(i32)"
    );
    assert_eq!(
        HOST_SIGNAL_RESTORE.declare(),
        "declare i32 @draconic_rt_host_signal_restore(i32)"
    );
    assert_eq!(
        HOST_SIGNAL_RAISE.declare(),
        "declare i32 @draconic_rt_host_signal_raise(i32)"
    );
    assert_eq!(
        HOST_SIGNAL_POLL.declare(),
        "declare i32 @draconic_rt_host_signal_poll()"
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
        "draconic_rt_host_cwd",
        "draconic_rt_host_chdir",
        "draconic_rt_host_hostname",
        "draconic_rt_host_os_type",
        "draconic_rt_host_os_arch",
        "draconic_rt_host_temp_dir",
        "draconic_rt_host_home_dir",
        "draconic_rt_host_process_run",
        "draconic_rt_host_process_spawn",
        "draconic_rt_host_process_stdin_write",
        "draconic_rt_host_process_wait",
        "draconic_rt_host_process_wait_async",
        "draconic_rt_host_process_pending",
        "draconic_rt_host_process_poll",
        "draconic_rt_host_process_stdout",
        "draconic_rt_host_process_stderr",
        "draconic_rt_host_process_kill",
        "draconic_rt_host_process_close",
        "draconic_rt_host_signal_watch",
        "draconic_rt_host_signal_ignore",
        "draconic_rt_host_signal_restore",
        "draconic_rt_host_signal_raise",
        "draconic_rt_host_signal_poll",
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
        "draconic_rt_host_path_resolve",
    ] {
        assert!(
            out.contains(sym),
            "archive must contain host symbol {sym}\nnm out={out}"
        );
    }
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

            {
                const char *parts1[] = {"/foo", "bar"};
                s = draconic_rt_host_path_resolve(2, parts1);
                if (!expect_str(s, "/foo/bar", "resolve abs+rel")) return 13;
                free(s);
            }
            {
                const char *parts2[] = {"/foo", "/bar"};
                s = draconic_rt_host_path_resolve(2, parts2);
                if (!expect_str(s, "/bar", "resolve abs wins")) return 14;
                free(s);
            }
            {
                const char *parts3[] = {"/foo/bar", "/tmp/file/"};
                s = draconic_rt_host_path_resolve(2, parts3);
                if (!expect_str(s, "/tmp/file", "resolve strip trail")) return 15;
                free(s);
            }
            {
                s = draconic_rt_host_path_resolve(0, NULL);
                if (!s || s[0] != '/') return 16;
                if (draconic_rt_host_path_is_absolute(s) != 1) return 17;
                free(s);
            }

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
    assert_eq!(String::from_utf8_lossy(&output.stdout), "path-h0302-ok\n");
}
