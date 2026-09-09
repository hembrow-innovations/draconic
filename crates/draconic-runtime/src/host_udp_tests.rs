//! Host UDP bind, send, and recv ABI tests.

use super::*;
use std::process::Command;

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
