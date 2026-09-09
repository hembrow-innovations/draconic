//! Host HTTPS HTTP/1.1 over TLS ABI tests.

use super::*;
use std::process::Command;

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
