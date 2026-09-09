//! Host WebSocket handshake and frame ABI tests.

use super::*;
use std::process::Command;

#[test]
fn host_ws_handshake_response_rfc6455_sample() {
    // H12.01: RFC 6455 §1.3 sample key → Sec-WebSocket-Accept.
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_ws_h1201.c");
    let bin = dir.join("rt_host_ws_h1201");
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
            char *msg = NULL;
            const char *want =
                "HTTP/1.1 101 Switching Protocols\r\n"
                "Upgrade: websocket\r\n"
                "Connection: Upgrade\r\n"
                "Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n"
                "\r\n";

            err = draconic_rt_host_ws_handshake_response(
                "dGhlIHNhbXBsZSBub25jZQ==", &msg);
            if (err != DRACONIC_HOST_OK) return 1;
            if (!msg || strcmp(msg, want) != 0) {
                fprintf(stderr, "got=[%s]\n", msg ? msg : "(null)");
                return 2;
            }
            free(msg);
            msg = NULL;

            err = draconic_rt_host_ws_handshake_response("", &msg);
            if (err != DRACONIC_HOST_E_INVAL) return 3;
            if (msg) return 4;

            err = draconic_rt_host_ws_handshake_response(NULL, &msg);
            if (err != DRACONIC_HOST_E_INVAL) return 5;

            puts("ws-h1201-ok");
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
    assert!(status.success(), "clang failed for ws H12.01 smoke");

    let output = Command::new(&bin).output().expect("run ws h1201");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "ws H12.01 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "ws-h1201-ok\n");
}

#[test]
fn host_ws_frames_text_binary_close_ping_pong() {
    // H12.02: RFC 6455 frames — text wire sample, roundtrips, masked client decode.
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_ws_h1202.c");
    let bin = dir.join("rt_host_ws_h1202");
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
        #include <stdint.h>

        int main(void) {
            DraconicHostError err;
            uint8_t *frame = NULL;
            size_t flen = 0;
            int32_t fin = 0, opcode = 0, close_code = -1;
            uint8_t *payload = NULL;
            size_t plen = 0;
            /* RFC 6455 unmasked text "Hello" */
            static const uint8_t want_hello[] = {0x81, 0x05, 'H', 'e', 'l', 'l', 'o'};
            /* RFC 6455 masked client text "Hello" with mask 0x37 0xfa 0x21 0x3d */
            static const uint8_t masked_hello[] = {
                0x81, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58
            };

            err = draconic_rt_host_ws_encode_text("Hello", &frame, &flen);
            if (err != DRACONIC_HOST_OK) return 1;
            if (flen != sizeof(want_hello) || memcmp(frame, want_hello, flen) != 0) {
                fprintf(stderr, "text wire mismatch len=%zu\n", flen);
                return 2;
            }
            free(frame); frame = NULL; flen = 0;

            err = draconic_rt_host_ws_decode_frame(
                want_hello, sizeof(want_hello),
                &fin, &opcode, &payload, &plen, &close_code);
            if (err != DRACONIC_HOST_OK) return 3;
            if (fin != 1 || opcode != 1 || close_code != -1) return 4;
            if (plen != 5 || memcmp(payload, "Hello", 5) != 0) return 5;
            free(payload); payload = NULL; plen = 0;

            err = draconic_rt_host_ws_decode_frame(
                masked_hello, sizeof(masked_hello),
                &fin, &opcode, &payload, &plen, &close_code);
            if (err != DRACONIC_HOST_OK) return 6;
            if (fin != 1 || opcode != 1 || plen != 5 || memcmp(payload, "Hello", 5) != 0) return 7;
            free(payload); payload = NULL;

            err = draconic_rt_host_ws_encode_binary((const uint8_t *)"Hi", 2, &frame, &flen);
            if (err != DRACONIC_HOST_OK) return 8;
            err = draconic_rt_host_ws_decode_frame(frame, flen, &fin, &opcode, &payload, &plen, &close_code);
            free(frame); frame = NULL;
            if (err != DRACONIC_HOST_OK || opcode != 2 || plen != 2 || memcmp(payload, "Hi", 2) != 0)
                return 9;
            free(payload); payload = NULL;

            err = draconic_rt_host_ws_encode_close(1000, "bye", &frame, &flen);
            if (err != DRACONIC_HOST_OK) return 10;
            err = draconic_rt_host_ws_decode_frame(frame, flen, &fin, &opcode, &payload, &plen, &close_code);
            free(frame); frame = NULL;
            if (err != DRACONIC_HOST_OK || opcode != 8 || close_code != 1000) return 11;
            if (plen != 3 || memcmp(payload, "bye", 3) != 0) return 12;
            free(payload); payload = NULL;

            err = draconic_rt_host_ws_encode_ping("x", &frame, &flen);
            if (err != DRACONIC_HOST_OK) return 13;
            err = draconic_rt_host_ws_decode_frame(frame, flen, &fin, &opcode, &payload, &plen, &close_code);
            free(frame); frame = NULL;
            if (err != DRACONIC_HOST_OK || opcode != 9 || plen != 1 || payload[0] != 'x') return 14;
            free(payload); payload = NULL;

            err = draconic_rt_host_ws_encode_pong("x", &frame, &flen);
            if (err != DRACONIC_HOST_OK) return 15;
            err = draconic_rt_host_ws_decode_frame(frame, flen, &fin, &opcode, &payload, &plen, &close_code);
            free(frame); frame = NULL;
            if (err != DRACONIC_HOST_OK || opcode != 10 || plen != 1 || payload[0] != 'x') return 16;
            free(payload); payload = NULL;

            err = draconic_rt_host_ws_decode_frame((const uint8_t *)"\x81", 1,
                &fin, &opcode, &payload, &plen, &close_code);
            if (err != DRACONIC_HOST_E_INVAL) return 17;

            puts("ws-h1202-ok");
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
    assert!(status.success(), "clang failed for ws H12.02 smoke");

    let output = Command::new(&bin).output().expect("run ws h1202");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "ws H12.02 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "ws-h1202-ok\n");
}

#[test]
fn host_ws_client_handshake_accept_masked_text() {
    // H12.03: client handshake request, Accept check, masked text encode/decode.
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_ws_h1203.c");
    let bin = dir.join("rt_host_ws_h1203");
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
        #include <stdint.h>

        int main(void) {
            DraconicHostError err;
            char *req = NULL;
            char *resp = NULL;
            uint8_t *frame = NULL;
            size_t flen = 0;
            int32_t fin = 0, opcode = 0, close_code = -1;
            uint8_t *payload = NULL;
            size_t plen = 0;
            static const char key[] = "dGhlIHNhbXBsZSBub25jZQ==";

            err = draconic_rt_host_ws_client_handshake_request("/echo", "127.0.0.1", key, &req);
            if (err != DRACONIC_HOST_OK || !req) return 1;
            if (!strstr(req, "GET /echo HTTP/1.1\r\n")) return 2;
            if (!strstr(req, "Host: 127.0.0.1\r\n")) return 3;
            if (!strstr(req, "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n")) return 4;
            if (!strstr(req, "Upgrade: websocket\r\n")) return 5;
            free(req); req = NULL;

            err = draconic_rt_host_ws_handshake_response(key, &resp);
            if (err != DRACONIC_HOST_OK || !resp) return 6;
            err = draconic_rt_host_ws_client_check_accept(
                (const uint8_t *)resp, strlen(resp), key);
            if (err != DRACONIC_HOST_OK) return 7;
            err = draconic_rt_host_ws_client_check_accept(
                (const uint8_t *)"HTTP/1.1 200 OK\r\n\r\n", 19, key);
            if (err != DRACONIC_HOST_E_INVAL) return 8;
            err = draconic_rt_host_ws_client_check_accept(
                (const uint8_t *)
                    "HTTP/1.1 101 Switching Protocols\r\n"
                    "Upgrade: websocket\r\n"
                    "Connection: Upgrade\r\n"
                    "Sec-WebSocket-Accept: AAAAAAAAAAAAAAAAAAAAAAAAAAA=\r\n\r\n",
                130, key);
            if (err != DRACONIC_HOST_E_INVAL) return 9;
            free(resp); resp = NULL;

            err = draconic_rt_host_ws_encode_text_client("hello", &frame, &flen);
            if (err != DRACONIC_HOST_OK || !frame || flen < 6) return 10;
            if ((frame[0] & 0x8f) != 0x81) return 11; /* FIN + text */
            if ((frame[1] & 0x80) == 0) return 12; /* MASK must be set */
            err = draconic_rt_host_ws_decode_frame(
                frame, flen, &fin, &opcode, &payload, &plen, &close_code);
            free(frame); frame = NULL;
            if (err != DRACONIC_HOST_OK) return 13;
            if (fin != 1 || opcode != 1 || plen != 5 || memcmp(payload, "hello", 5) != 0) return 14;
            free(payload);

            err = draconic_rt_host_ws_client_handshake_request("", "h", key, &req);
            if (err != DRACONIC_HOST_E_INVAL) return 15;

            puts("ws-h1203-ok");
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
    assert!(status.success(), "clang failed for ws H12.03 smoke");

    let output = Command::new(&bin).output().expect("run ws h1203");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "ws H12.03 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "ws-h1203-ok\n");
}
