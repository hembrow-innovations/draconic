//! Host TLS client and server wrap ABI tests.

use super::*;
use std::process::Command;

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
