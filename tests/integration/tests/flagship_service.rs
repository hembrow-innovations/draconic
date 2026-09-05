//! ROADMAP P04: flagship service example — typed HTTP + fs/config + git dep.
//!
//! Native: start `examples/flagship-service/server.drac`, GET, assert config +
//! git-dep values in the body, shutdown.
//! JS: build and run the portable config/git-dep path (HTTP listen stays native-first).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use draconic_backend_js::emit_js;
use draconic_backend_llvm::{build_native_binary, emit_llvm_ir};
use draconic_frontend::compile_path;
use draconic_pkg::{
    default_cache_root, parse_manifest, resolve_direct_deps, write_lock, write_manifest,
    ModuleCache, LOCK_FILE, MANIFEST_FILE,
};

const PKG_LIB_MODULE: &str = "github.com/draconic-lang/pkg-lib";
const SERVICE_MODULE: &str = "github.com/draconic-lang/flagship-service";
const ADDR: &str = "127.0.0.1:18084";
const LISTEN_MSG: &str = "flagship-service listening on 18084";
const CONFIG_NAME: &str = "flagship";

fn temp_dir(label: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-flagship-{}-{}-{}-{}",
        label,
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

fn example_dir() -> PathBuf {
    repo_root().join("examples/flagship-service")
}

fn pkg_lib_dir() -> PathBuf {
    repo_root().join("examples/pkg-lib")
}

fn git_ok(args: &[&str], cwd: &Path) {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "Draconic Test")
        .env("GIT_AUTHOR_EMAIL", "test@draconic.local")
        .env("GIT_COMMITTER_NAME", "Draconic Test")
        .env("GIT_COMMITTER_EMAIL", "test@draconic.local")
        .output()
        .expect("spawn git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn head_oid(repo: &Path) -> String {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .expect("rev-parse");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Copy in-repo pkg-lib into a fresh tagged git upstream.
fn tagged_upstream_from_pkg_lib(root: &Path) -> (PathBuf, String) {
    let src = pkg_lib_dir();
    assert!(
        src.is_dir(),
        "examples/pkg-lib missing at {}",
        src.display()
    );

    let repo = root.join("pkg-lib-upstream");
    fs::create_dir_all(&repo).unwrap();
    for name in ["draconic.toml", "index.drac", "README.md"] {
        let from = src.join(name);
        if from.is_file() {
            fs::copy(&from, repo.join(name)).unwrap();
        }
    }
    git_ok(&["init"], &repo);
    git_ok(&["config", "user.email", "test@draconic.local"], &repo);
    git_ok(&["config", "user.name", "Draconic Test"], &repo);
    git_ok(&["checkout", "-B", "main"], &repo);
    git_ok(&["add", "."], &repo);
    git_ok(&["commit", "-m", "v0.1.0"], &repo);
    let oid = head_oid(&repo);
    git_ok(&["tag", "v0.1.0"], &repo);
    (repo, oid)
}

/// Consumer workspace: in-repo example + local git URL + lock/cache.
fn materialize_workspace(root: &Path) -> PathBuf {
    let src = example_dir();
    let (upstream, oid) = tagged_upstream_from_pkg_lib(root);
    let lib_url = upstream.to_str().expect("utf8 path");

    let ws = root.join("service");
    fs::create_dir_all(&ws).unwrap();

    let base = parse_manifest(&fs::read_to_string(src.join(MANIFEST_FILE)).unwrap())
        .expect("flagship-service manifest");
    let mut manifest = base;
    manifest
        .urls
        .insert(PKG_LIB_MODULE.to_string(), lib_url.to_string());
    fs::write(ws.join(MANIFEST_FILE), write_manifest(&manifest)).unwrap();

    for name in ["server.drac", "portable.drac", "config.txt", "README.md"] {
        let from = src.join(name);
        assert!(from.is_file(), "missing {}", from.display());
        fs::copy(&from, ws.join(name)).unwrap();
    }

    let cache = ModuleCache::new(default_cache_root(&ws));
    let lock = resolve_direct_deps(&manifest, &cache).expect("resolve+fetch pkg-lib");
    fs::write(ws.join(LOCK_FILE), write_lock(&lock)).unwrap();
    assert!(
        cache.has_entry(PKG_LIB_MODULE, &oid).unwrap(),
        "pkg-lib cache miss oid={oid}"
    );
    ws
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
        .expect("spawn flagship-service");
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
                panic!("flagship-service exited before listen banner; stderr={err:?}");
            }
            Ok(_) => {
                if line.contains(LISTEN_MSG) {
                    return;
                }
            }
            Err(e) => panic!("read flagship-service stdout: {e}"),
        }
        if start.elapsed() > timeout {
            panic!("timeout waiting for listen banner ({timeout:?})");
        }
    }
}

fn http_get(path: &str, timeout: Duration) -> String {
    let mut stream = TcpStream::connect(ADDR).expect("connect to flagship-service");
    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).expect("write request");
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}

/// P04 layout: one example Program with typed HTTP, fs config, and git module path.
#[test]
fn flagship_service_layout() {
    let dir = example_dir();
    assert!(
        dir.is_dir(),
        "examples/flagship-service must exist at {}",
        dir.display()
    );

    let manifest_path = dir.join(MANIFEST_FILE);
    assert!(
        manifest_path.is_file(),
        "examples/flagship-service/{MANIFEST_FILE} required"
    );
    let manifest =
        parse_manifest(&fs::read_to_string(&manifest_path).unwrap()).expect("valid manifest");
    assert_eq!(manifest.module, SERVICE_MODULE, "service module path");
    let dep = manifest
        .dependencies
        .get(PKG_LIB_MODULE)
        .unwrap_or_else(|| panic!("must depend on {PKG_LIB_MODULE}"));
    assert!(
        dep.contains("0.1"),
        "version req should target 0.1.x, got {dep}"
    );

    let server = dir.join("server.drac");
    assert!(server.is_file(), "server.drac required");
    let server_src = fs::read_to_string(&server).unwrap();
    assert!(
        server_src.contains(PKG_LIB_MODULE),
        "server.drac must import module path {PKG_LIB_MODULE}:\n{server_src}"
    );
    assert!(
        server_src.contains("readFileText"),
        "server.drac must read config via host fs:\n{server_src}"
    );
    assert!(
        server_src.contains("tcpListen")
            && server_src.contains("httpParseRequest")
            && server_src.contains("httpWriteResponse"),
        "server.drac must use typed HTTP helpers:\n{server_src}"
    );
    assert!(
        server_src.contains("greet") && server_src.contains("VERSION"),
        "server.drac must use pkg-lib exports:\n{server_src}"
    );

    let portable = dir.join("portable.drac");
    assert!(
        portable.is_file(),
        "portable.drac required (JS config/git-dep path)"
    );
    let portable_src = fs::read_to_string(&portable).unwrap();
    assert!(
        portable_src.contains(PKG_LIB_MODULE) && portable_src.contains("readFileText"),
        "portable.drac must use git dep + fs config:\n{portable_src}"
    );
    assert!(
        !portable_src.contains("tcpListen"),
        "portable path must not listen (HTTP listen stays native-first):\n{portable_src}"
    );

    let config = dir.join("config.txt");
    assert!(config.is_file(), "config.txt required");
    let cfg = fs::read_to_string(&config).unwrap();
    assert!(
        cfg.trim() == CONFIG_NAME,
        "config.txt should name {CONFIG_NAME}, got {cfg:?}"
    );

    let readme = dir.join("README.md");
    assert!(readme.is_file(), "README.md required");
    let doc = fs::read_to_string(&readme).unwrap();
    assert!(
        doc.contains("draconic")
            && (doc.contains("mod tidy") || doc.contains("get ") || doc.contains("build")),
        "README must document tidy/get + build path:\n{doc}"
    );
    assert!(
        doc.contains(PKG_LIB_MODULE) || doc.contains("pkg-lib"),
        "README must mention pkg-lib dependency:\n{doc}"
    );
}

/// P04 JS: portable config + git-dep path builds and runs (no HTTP listen).
#[test]
fn flagship_service_js_portable_config_git_dep() {
    let root = temp_dir("js");
    let ws = materialize_workspace(&root);
    let entry = ws.join("portable.drac");
    let ir = compile_path(&entry).expect("compile portable.drac + pkg-lib");
    let js = emit_js(&ir).expect("emit js");

    let node = Command::new("node")
        .arg("-e")
        .arg(&js)
        .current_dir(&ws)
        .output()
        .expect("spawn node");
    assert!(
        node.status.success(),
        "node failed: stdout={} stderr={}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
    let stdout = String::from_utf8_lossy(&node.stdout);
    assert!(
        stdout.contains("hello, flagship"),
        "expected greet(config) on stdout, got:\n{stdout}"
    );
    assert!(
        stdout.contains("0.1.0"),
        "expected VERSION on stdout, got:\n{stdout}"
    );

    let _ = fs::remove_dir_all(&root);
}

/// P04 native: HTTP + fs/config + git-dep — start, GET /hello, assert body, shutdown.
#[test]
fn flagship_service_native_http_config_git_dep() {
    let root = temp_dir("native");
    let ws = materialize_workspace(&root);
    let src = ws.join("server.drac");
    let out = root.join("flagship-service");
    let module = compile_path(&src).expect("compile server.drac + pkg-lib");
    let ll = emit_llvm_ir(&module).expect("emit_llvm_ir");
    build_native_binary(&ll, Path::new(&out)).expect("build_native_binary");
    assert!(out.is_file(), "binary missing at {}", out.display());

    let mut guard = spawn_server(&out, &ws);
    wait_for_listen_banner(&mut guard.0, Duration::from_secs(10));

    let resp = http_get("/hello", Duration::from_secs(5));
    assert!(
        resp.contains("HTTP/1.1 200"),
        "expected HTTP/1.1 200, got:\n{resp}"
    );
    assert!(
        resp.contains("hello, flagship"),
        "expected greet(config) in body, got:\n{resp}"
    );
    assert!(
        resp.contains("0.1.0"),
        "expected git-dep VERSION in body, got:\n{resp}"
    );
    assert!(
        resp.contains("/hello"),
        "expected request path in body, got:\n{resp}"
    );

    guard.0.kill().expect("kill flagship-service");
    let status = guard.0.wait().expect("wait flagship-service");
    assert!(
        !status.success(),
        "killed accept-loop should not exit 0, status={status:?}"
    );

    let _ = fs::remove_dir_all(&root);
}
