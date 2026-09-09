//! Host HTTP/2 preface and single-stream ABI tests.

use super::*;
use std::process::Command;

#[test]
fn host_http2_preface_single_stream_roundtrip() {
    // H13.01: client preface length; encode/parse request+response on stream 1.
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_http2_h1301.c");
    let bin = dir.join("rt_host_http2_h1301");
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
            DraconicHostError err;
            uint8_t *pref = NULL;
            size_t pref_len = 0;
            uint8_t *wire = NULL;
            size_t wire_len = 0;
            char *method = NULL;
            char *path = NULL;
            uint8_t *body = NULL;
            size_t body_len = 0;
            int32_t stream_id = 0;
            int32_t status = 0;

            err = draconic_rt_host_http2_client_preface(&pref, &pref_len);
            if (err != DRACONIC_HOST_OK) return 1;
            if (pref_len != 33) return 2;
            if (memcmp(pref, "PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n", 24) != 0) return 3;
            free(pref);

            err = draconic_rt_host_http2_encode_request(
                "GET", "/hello", NULL, 0, &wire, &wire_len);
            if (err != DRACONIC_HOST_OK) return 4;
            err = draconic_rt_host_http2_parse_request(
                wire, wire_len, &method, &path, &body, &body_len, &stream_id);
            free(wire);
            wire = NULL;
            if (err != DRACONIC_HOST_OK) return 5;
            if (!method || strcmp(method, "GET") != 0) return 6;
            if (!path || strcmp(path, "/hello") != 0) return 7;
            if (stream_id != 1) return 8;
            if (body_len != 0) return 9;
            free(method);
            free(path);
            free(body);
            method = path = NULL;
            body = NULL;

            err = draconic_rt_host_http2_encode_response(
                200, (const uint8_t *)"ok", 2, &wire, &wire_len);
            if (err != DRACONIC_HOST_OK) return 10;
            err = draconic_rt_host_http2_parse_response(
                wire, wire_len, &status, &body, &body_len, &stream_id);
            free(wire);
            if (err != DRACONIC_HOST_OK) return 11;
            if (status != 200) return 12;
            if (stream_id != 1) return 13;
            if (body_len != 2 || memcmp(body, "ok", 2) != 0) return 14;
            free(body);

            puts("http2-h1301-ok");
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
    assert!(status.success(), "clang failed for http2 H13.01 smoke");

    let output = Command::new(&bin).output().expect("run http2 h1301");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "http2 H13.01 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "http2-h1301-ok\n");
}
