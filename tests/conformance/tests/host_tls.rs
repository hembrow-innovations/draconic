//! ROADMAP H11 / H11.01–H11.03: TLS client/server wrap + HTTPS loopback.
//! H11 parent locks the combined TLS surface in one Program.

use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use draconic_backend_llvm::{build_native_binary, emit_llvm_ir};
use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};
use draconic_frontend::compile_source;

fn assert_fixture_present(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == id),
        "missing {id} fixture, got {ids:?}"
    );
}

fn assert_fixture_runs(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
    for r in run_fixture(fixture) {
        assert!(
            r.ok,
            "{} @ {}: {}",
            r.fixture_id,
            r.target.as_str(),
            r.message
        );
    }
}

fn find_openssl() -> Option<PathBuf> {
    ["openssl", "/usr/bin/openssl", "/opt/homebrew/bin/openssl"]
        .iter()
        .map(PathBuf::from)
        .find(|p| {
            p.is_file()
                || Command::new(p)
                    .arg("version")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
        })
}

fn gen_self_signed(dir: &Path, openssl: &Path) -> (PathBuf, PathBuf) {
    let cert = dir.join("cert.pem");
    let key = dir.join("key.pem");
    let gen = Command::new(openssl)
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
    (cert, key)
}

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    listener.local_addr().unwrap().port()
}

fn compile_native(src: &str, out_bin: &Path) {
    let module = compile_source(src).expect("compile");
    let ll = emit_llvm_ir(&module).expect("emit_llvm_ir");
    build_native_binary(&ll, out_bin).expect("build_native_binary");
}

#[test]
fn tls_client_plain_peer_fails_fixture_present() {
    assert_fixture_present("host/net/tls/tls_client_plain_peer_fails");
}

#[test]
fn tls_client_plain_peer_fails_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/tls/tls_client_plain_peer_fails")
        .expect("host/net/tls/tls_client_plain_peer_fails");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/net/tls/tls_client_plain_peer_fails");
}

#[test]
fn tls_server_missing_cert_fixture_present() {
    assert_fixture_present("host/net/tls/tls_server_missing_cert");
}

#[test]
fn tls_server_missing_cert_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/tls/tls_server_missing_cert")
        .expect("host/net/tls/tls_server_missing_cert");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/net/tls/tls_server_missing_cert");
}

#[test]
fn https_loopback_fixture_present() {
    assert_fixture_present("host/net/tls/https_loopback");
    assert_fixture_present("host/net/tls/https_server_oneshot");
}

#[test]
fn https_loopback_client_no_peer_fails_native() {
    // Standalone client with port 0 → connect fails (ECONN).
    assert_fixture_runs("host/net/tls/https_loopback");
}

#[test]
fn https_server_oneshot_missing_cert_fails_native() {
    assert_fixture_runs("host/net/tls/https_server_oneshot");
}

#[test]
fn https_loopback_runs_native() {
    // H11.03: dual-process HTTPS loopback — Draconic TLS server + client.
    if !cfg!(target_os = "macos") {
        return;
    }
    let Some(openssl) = find_openssl() else {
        return;
    };

    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-https-h1103-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");
    let (cert, key) = gen_self_signed(&dir, &openssl);
    let port = free_port();
    let cert_s = cert.display().to_string().replace('\\', "\\\\");
    let key_s = key.display().to_string().replace('\\', "\\\\");

    let server_src = format!(
        r#"
let s = tcpListen({port});
let a = tcpAccept(s);
let t = tlsServerWrap(a, "{cert}", "{key}");
let raw = tlsRead(t, 4096);
let req = httpParseRequest(raw);
let path = req.path;
let resp = httpWriteResponse(200, "OK", "Content-Type: text/plain\r\n", path);
tlsWrite(t, resp);
closeTls(t);
closeTcp(s);
"#,
        port = port,
        cert = cert_s,
        key = key_s
    );
    let client_src = format!(
        r#"
let c = tcpConnect("127.0.0.1", {port});
let t = tlsClientWrap(c, "localhost", 1);
let reqMsg = httpWriteRequest("GET", "/hello", "Host: localhost\r\n", "");
tlsWrite(t, reqMsg);
let out = tlsRead(t, 4096);
let res = httpParseResponse(out);
let v = res.version;
let st = res.status;
let r = res.reason;
let b = res.body;
closeTls(t);
"#,
        port = port
    );

    let server_bin = dir.join("https_server");
    let client_bin = dir.join("https_client");
    compile_native(&server_src, &server_bin);
    compile_native(&client_src, &client_bin);

    let mut server = Command::new(&server_bin)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn server");

    // Give the server time to bind + block in accept. Do not TCP-probe: that
    // would consume the single accept slot before the client connects.
    thread::sleep(Duration::from_millis(150));

    let client_out = Command::new(&client_bin).output().expect("run client");
    let _ = server.kill();
    let server_out = server.wait_with_output().expect("wait server");

    let stdout = String::from_utf8_lossy(&client_out.stdout);
    let stderr = String::from_utf8_lossy(&client_out.stderr);
    let sstderr = String::from_utf8_lossy(&server_out.stderr);
    assert!(
        client_out.status.success(),
        "https client failed: {:?}\nstdout={stdout}\nstderr={stderr}\nserver_stderr={sstderr}",
        client_out.status
    );
    assert_eq!(
        stdout.as_ref(),
        "HTTP/1.1\n200\nOK\n/hello\n",
        "stdout={stdout:?}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn surface_fixture_present() {
    assert_fixture_present("host/net/tls/surface");
}

#[test]
fn surface_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/tls/surface")
        .expect("host/net/tls/surface");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    for name in [
        "tlsClientWrap",
        "tlsServerWrap",
        "tlsRead",
        "tlsWrite",
        "closeTls",
        "httpWriteRequest",
        "httpParseRequest",
        "httpWriteResponse",
        "httpParseResponse",
    ] {
        assert!(
            fixture.source.contains(name),
            "H11 surface must use {name} in one Program"
        );
    }
    assert_eq!(
        fixture.expect_native.exit, 1,
        "H11 surface handshake needs two processes; missing PEM fails closed"
    );
    assert_eq!(
        fixture.expect_native.stderr.as_deref(),
        Some("EIO\n"),
        "H11 surface must fail closed on missing cert/key"
    );
    assert_fixture_runs("host/net/tls/surface");
}
