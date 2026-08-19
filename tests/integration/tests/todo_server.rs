//! ROADMAP H17.03: build `examples/todo` native static host, GET index, assert body, shutdown.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use draconic_backend_llvm::{build_native_binary, emit_llvm_ir};
use draconic_frontend::compile_path;

const ADDR: &str = "127.0.0.1:18083";
const LISTEN_MSG: &str = "Draconic todo server listening on http://127.0.0.1:18083";

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-todo-server-{}-{}-{}",
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

fn build_todo_server() -> PathBuf {
    let src = repo_root().join("examples/todo/server.drac");
    assert!(src.is_file(), "missing {}", src.display());
    let dir = temp_dir();
    let out = dir.join("todo-server");
    let module = compile_path(&src).expect("compile examples/todo/server.drac");
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

fn spawn_server(bin: &Path, cwd: &Path) -> ChildGuard {
    let child = Command::new(bin)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn todo-server");
    ChildGuard(child)
}

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
                panic!("todo-server exited before listen banner; stderr={err:?}");
            }
            Ok(_) => {
                if line.contains(LISTEN_MSG) {
                    return;
                }
            }
            Err(e) => panic!("read todo-server stdout: {e}"),
        }
        if start.elapsed() > timeout {
            panic!("timeout waiting for listen banner ({timeout:?})");
        }
    }
}

fn http_get(path: &str, timeout: Duration) -> String {
    let mut stream = TcpStream::connect(ADDR).expect("connect to todo-server");
    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).expect("write request");
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}

/// H17.03: build server.drac → start in examples/todo → GET / → 200 + index → kill.
#[test]
fn todo_server_serves_index_and_shutdown() {
    let todo_dir = repo_root().join("examples/todo");
    let public = todo_dir.join("public");
    assert!(public.join("index.html").is_file(), "missing index.html");

    let bin = build_todo_server();
    let mut guard = spawn_server(&bin, &todo_dir);

    wait_for_listen_banner(&mut guard.0, Duration::from_secs(10));

    let resp = http_get("/", Duration::from_secs(5));
    assert!(
        resp.contains("HTTP/1.1 200"),
        "expected HTTP/1.1 200, got:\n{resp}"
    );
    assert!(
        resp.contains("text/html"),
        "expected text/html content-type, got:\n{resp}"
    );
    assert!(
        resp.contains("todo") || resp.contains("<!DOCTYPE") || resp.contains("<html"),
        "expected index.html body, got:\n{resp}"
    );

    let js_resp = http_get("/todo.js", Duration::from_secs(5));
    assert!(
        js_resp.contains("HTTP/1.1 200"),
        "expected todo.js 200, got:\n{js_resp}"
    );

    let missing = http_get("/no-such-file-h1703", Duration::from_secs(5));
    assert!(
        missing.contains("HTTP/1.1 404"),
        "expected 404, got:\n{missing}"
    );

    let traversal = http_get("/../Cargo.toml", Duration::from_secs(5));
    assert!(
        traversal.contains("HTTP/1.1 404"),
        "expected traversal 404, got:\n{traversal}"
    );

    guard.0.kill().expect("kill todo-server");
    let status = guard.0.wait().expect("wait todo-server");
    assert!(
        !status.success(),
        "killed accept-loop should not exit 0, status={status:?}"
    );
}
