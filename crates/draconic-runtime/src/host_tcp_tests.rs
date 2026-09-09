//! Host TCP listen, accept, connect, and async I/O ABI tests.

use super::*;
use std::process::Command;

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
