//! ROADMAP H17.02: start `examples/http-echo`, client request, assert status/body, shutdown.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use draconic_backend_llvm::{build_native_binary, emit_llvm_ir};
use draconic_frontend::compile_path;

const ECHO_ADDR: &str = "127.0.0.1:8080";
const LISTEN_MSG: &str = "http-echo listening on 8080";

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-http-echo-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn build_http_echo() -> PathBuf {
    let src = repo_root().join("examples/http-echo/main.drac");
    assert!(src.is_file(), "missing {}", src.display());
    let dir = temp_dir();
    let out = dir.join("http-echo");
    let module = compile_path(&src).expect("compile examples/http-echo/main.drac");
    let ll = emit_llvm_ir(&module).expect("emit_llvm_ir");
    build_native_binary(&ll, Path::new(&out)).expect("build_native_binary");
    assert!(out.is_file(), "binary missing at {}", out.display());
    out
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn_echo(bin: &Path) -> ChildGuard {
    let child = Command::new(bin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn http-echo");
    ChildGuard(child)
}

/// Wait for the listen banner on stdout (do not TCP-probe — that steals accept).
fn wait_for_listen_banner(child: &mut Child, timeout: Duration) {
    let stdout = child.stdout.take().expect("stdout piped");
    let start = Instant::now();
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => {
                let err = child.stderr.take().map(|mut e| {
                    let mut s = String::new();
                    let _ = e.read_to_string(&mut s);
                    s
                });
                panic!(
                    "http-echo exited before listen banner; stderr={err:?}"
                );
            }
            Ok(_) => {
                if line.contains(LISTEN_MSG) {
                    // Keep remaining stdout attached via Drop only; banner seen.
                    // Put reader back? Child.stdout already taken — leave closed.
                    return;
                }
            }
            Err(e) => panic!("read http-echo stdout: {e}"),
        }
        if start.elapsed() > timeout {
            panic!("timeout waiting for listen banner ({timeout:?})");
        }
    }
}

fn http_get(path: &str, timeout: Duration) -> String {
    let mut stream = TcpStream::connect(ECHO_ADDR).expect("connect to http-echo");
    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).expect("write request");
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}

/// H17.02: build example → start → GET /hello → 200 + body `/hello` → kill server.
#[test]
fn http_echo_start_request_assert_shutdown() {
    let bin = build_http_echo();
    let mut guard = spawn_echo(&bin);

    wait_for_listen_banner(&mut guard.0, Duration::from_secs(10));

    let resp = http_get("/hello", Duration::from_secs(5));
    assert!(
        resp.contains("HTTP/1.1 200"),
        "expected HTTP/1.1 200, got:\n{resp}"
    );
    assert!(
        resp.contains("\r\n\r\n/hello") || resp.ends_with("/hello"),
        "expected body /hello, got:\n{resp}"
    );

    // Shutdown: kill accept-loop server; process must terminate.
    guard.0.kill().expect("kill http-echo");
    let status = guard.0.wait().expect("wait http-echo");
    assert!(
        !status.success(),
        "killed accept-loop should not exit 0, status={status:?}"
    );
}
