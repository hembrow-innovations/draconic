//! K10.01: `examples/pkg-lib` is a minimal exportable module.
//!
//! In-repo package demo (not a temp fixture):
//! - `draconic.toml` with Go-like module path
//! - package root entry (`index.drac`) with named exports
//! - Frontend can compile a consumer that imports the module path
//!   when the lib is served as a local git upstream (URL map)

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_js::emit_js;
use draconic_frontend::compile_path;
use draconic_pkg::{
    default_cache_root, parse_manifest, resolve_direct_deps, write_lock, write_manifest,
    ModuleCache, LOCK_FILE, MANIFEST_FILE,
};

fn repo_root() -> PathBuf {
    // tests/packages/tests/k10_01_pkg_lib.rs → repo root is ../../..
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn pkg_lib_dir() -> PathBuf {
    repo_root().join("examples/pkg-lib")
}

fn uniq_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "draconic-pkg-k10_01-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
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

/// Copy in-repo pkg-lib into a fresh tagged git upstream (package cache expects git).
fn tagged_upstream_from_pkg_lib(root: &Path) -> (PathBuf, String, String) {
    let src = pkg_lib_dir();
    assert!(
        src.is_dir(),
        "examples/pkg-lib missing at {}",
        src.display()
    );

    let manifest_src = fs::read_to_string(src.join(MANIFEST_FILE))
        .unwrap_or_else(|e| panic!("read pkg-lib {MANIFEST_FILE}: {e}"));
    let manifest = parse_manifest(&manifest_src).expect("pkg-lib draconic.toml schema");
    let module_path = manifest.module.clone();

    let repo = root.join("pkg-lib-upstream");
    fs::create_dir_all(&repo).unwrap();
    // Copy package tree (not .git).
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
    (repo, module_path, oid)
}

/// K10.01: on-disk layout — manifest + index entry + named exports.
#[test]
fn k10_01_pkg_lib_layout_and_manifest() {
    let dir = pkg_lib_dir();
    assert!(dir.is_dir(), "examples/pkg-lib must exist");

    let manifest_path = dir.join(MANIFEST_FILE);
    assert!(
        manifest_path.is_file(),
        "examples/pkg-lib/{MANIFEST_FILE} required"
    );
    let manifest =
        parse_manifest(&fs::read_to_string(&manifest_path).unwrap()).expect("valid manifest");
    assert!(
        !manifest.module.is_empty(),
        "module path must be non-empty"
    );
    assert!(
        manifest.module.contains('/'),
        "module path should be Go-like (host/path), got {}",
        manifest.module
    );
    assert!(
        manifest.dependencies.is_empty(),
        "minimal lib has no deps"
    );

    let index = dir.join("index.drac");
    assert!(index.is_file(), "package root entry index.drac required");
    let body = fs::read_to_string(&index).unwrap();
    assert!(
        body.contains("export "),
        "index.drac must export symbols:\n{body}"
    );
}

/// K10.01: consumer imports module path of in-repo pkg-lib via local git URL map.
#[test]
fn k10_01_pkg_lib_importable_via_module_path() {
    let root = uniq_dir("import");
    let (upstream, module_path, oid) = tagged_upstream_from_pkg_lib(&root);
    let lib_url = upstream.to_str().expect("utf8 path");

    let ws = root.join("consumer");
    fs::create_dir_all(&ws).unwrap();
    let manifest_src = format!(
        r#"module = "github.com/draconic-lang/pkg-consumer-smoke"

[dependencies]
"{module_path}" = "0.1.0"

[urls]
"{module_path}" = "{lib_url}"
"#
    );
    let manifest = parse_manifest(&manifest_src).expect("consumer manifest");
    fs::write(ws.join(MANIFEST_FILE), write_manifest(&manifest)).unwrap();

    let cache = ModuleCache::new(default_cache_root(&ws));
    let lock = resolve_direct_deps(&manifest, &cache).expect("resolve+fetch pkg-lib");
    fs::write(ws.join(LOCK_FILE), write_lock(&lock)).unwrap();
    assert!(cache.has_entry(&module_path, &oid).unwrap());

    let main = ws.join("main.drac");
    fs::write(
        &main,
        format!(
            r#"import {{ greet, VERSION }} from "{module_path}";
let msg = greet("world");
let v = VERSION;
"#
        ),
    )
    .unwrap();

    let ir = compile_path(&main).expect("compile consumer + pkg-lib");
    let js = emit_js(&ir).expect("emit js");

    let node = Command::new("node")
        .arg("-e")
        .arg(format!(
            r#"{js}
if (typeof v !== "string" || v !== "0.1.0") {{ console.error("VERSION", v); process.exit(1); }}
if (typeof msg !== "string" || msg !== "hello, world") {{ console.error("greet", msg); process.exit(2); }}
"#
        ))
        .output()
        .expect("spawn node");
    assert!(
        node.status.success(),
        "node failed: stdout={} stderr={}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );

    let _ = fs::remove_dir_all(&root);
}
