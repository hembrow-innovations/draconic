//! Host HTTP/1.1 parse, write, chunked, and client ABI tests.

use super::*;
use std::process::Command;

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
fn host_http_chunked_transfer_encoding() {
    // H10.06: parse chunked request/response bodies; write chunked when TE present.
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_http_h1006.c");
    let bin = dir.join("rt_host_http_h1006");
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
            char *method = NULL;
            char *path = NULL;
            char *version = NULL;
            char *body = NULL;
            char *reason = NULL;
            char *msg = NULL;
            int32_t status = 0;
            const char *raw_req =
                "POST /up HTTP/1.1\r\n"
                "Host: x\r\n"
                "Transfer-Encoding: chunked\r\n"
                "\r\n"
                "5\r\n"
                "hello\r\n"
                "6\r\n"
                " world\r\n"
                "0\r\n"
                "\r\n";
            const char *raw_res =
                "HTTP/1.1 200 OK\r\n"
                "Transfer-Encoding: chunked\r\n"
                "\r\n"
                "3\r\n"
                "foo\r\n"
                "0\r\n"
                "\r\n";
            const char *want_write =
                "HTTP/1.1 200 OK\r\n"
                "Transfer-Encoding: chunked\r\n"
                "\r\n"
                "5\r\n"
                "hello\r\n"
                "0\r\n"
                "\r\n";
            const char *want_req =
                "POST /c HTTP/1.1\r\n"
                "Transfer-Encoding: chunked\r\n"
                "\r\n"
                "2\r\n"
                "hi\r\n"
                "0\r\n"
                "\r\n";

            err = draconic_rt_host_http_parse_request(
                (const uint8_t *)raw_req, strlen(raw_req),
                &method, &path, &version, &body);
            if (err != DRACONIC_HOST_OK) return 1;
            if (!body || strcmp(body, "hello world") != 0) return 2;
            free(method); free(path); free(version); free(body);
            method = path = version = body = NULL;

            err = draconic_rt_host_http_parse_response(
                (const uint8_t *)raw_res, strlen(raw_res),
                &version, &status, &reason, &body);
            if (err != DRACONIC_HOST_OK) return 3;
            if (status != 200) return 4;
            if (!body || strcmp(body, "foo") != 0) return 5;
            free(version); free(reason); free(body);
            version = reason = body = NULL;

            err = draconic_rt_host_http_write_response(
                200, "OK", "Transfer-Encoding: chunked\r\n",
                (const uint8_t *)"hello", 5, &msg);
            if (err != DRACONIC_HOST_OK) return 6;
            if (!msg || strcmp(msg, want_write) != 0) return 7;
            free(msg);
            msg = NULL;

            err = draconic_rt_host_http_write_request(
                "POST", "/c", "Transfer-Encoding: chunked\r\n",
                (const uint8_t *)"hi", 2, &msg);
            if (err != DRACONIC_HOST_OK) return 8;
            if (!msg || strcmp(msg, want_req) != 0) return 9;
            free(msg);
            msg = NULL;

            /* empty chunked body */
            err = draconic_rt_host_http_write_response(
                204, "", "Transfer-Encoding: chunked\r\n",
                NULL, 0, &msg);
            if (err != DRACONIC_HOST_OK) return 10;
            want_write =
                "HTTP/1.1 204 No Content\r\n"
                "Transfer-Encoding: chunked\r\n"
                "\r\n"
                "0\r\n"
                "\r\n";
            if (!msg || strcmp(msg, want_write) != 0) return 11;
            free(msg);

            /* malformed chunk size */
            {
                const char *bad =
                    "POST / HTTP/1.1\r\n"
                    "Transfer-Encoding: chunked\r\n"
                    "\r\n"
                    "ZZ\r\n";
                err = draconic_rt_host_http_parse_request(
                    (const uint8_t *)bad, strlen(bad),
                    &method, &path, &version, &body);
                if (err != DRACONIC_HOST_E_INVAL) return 12;
            }

            puts("http-h1006-ok");
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
    assert!(status.success(), "clang failed for http H10.06 smoke");

    let output = Command::new(&bin).output().expect("run http h1006");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "http H10.06 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "http-h1006-ok\n");
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
