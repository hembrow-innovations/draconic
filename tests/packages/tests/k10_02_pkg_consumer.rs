//! K10.02: `examples/pkg-consumer` depends on pkg-lib; documented build path.
//!
//! In-repo consumer demo:
//! - `draconic.toml` depends on `github.com/draconic-lang/pkg-lib`
//! - `main.drac` imports named exports from that module path
//! - README documents the local-git URL + tidy/get + build path
//! - End-to-end: resolve/fetch pkg-lib → compile consumer → Node observes values

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

const PKG_LIB_MODULE: &str = "github.com/draconic-lang/pkg-lib";
const PKG_CONSUMER_MODULE: &str = "github.com/draconic-lang/pkg-consumer";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn pkg_lib_dir() -> PathBuf {
    repo_root().join("examples/pkg-lib")
}

fn pkg_consumer_dir() -> PathBuf {
    repo_root().join("examples/pkg-consumer")
}

fn uniq_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "draconic-pkg-k10_02-{label}-{}-{nanos}",
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

/// Copy in-repo pkg-lib into a fresh tagged git upstream.
fn tagged_upstream_from_pkg_lib(root: &Path) -> (PathBuf, String) {
    let src = pkg_lib_dir();
    assert!(src.is_dir(), "examples/pkg-lib missing at {}", src.display());

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

/// K10.02: on-disk layout — manifest depends on pkg-lib; main imports; README build path.
#[test]
fn k10_02_pkg_consumer_layout_and_manifest() {
    let dir = pkg_consumer_dir();
    assert!(
        dir.is_dir(),
        "examples/pkg-consumer must exist at {}",
        dir.display()
    );

    let manifest_path = dir.join(MANIFEST_FILE);
    assert!(
        manifest_path.is_file(),
        "examples/pkg-consumer/{MANIFEST_FILE} required"
    );
    let manifest =
        parse_manifest(&fs::read_to_string(&manifest_path).unwrap()).expect("valid manifest");
    assert_eq!(
        manifest.module, PKG_CONSUMER_MODULE,
        "consumer module path"
    );
    let dep = manifest
        .dependencies
        .get(PKG_LIB_MODULE)
        .unwrap_or_else(|| panic!("must depend on {PKG_LIB_MODULE}"));
    assert!(
        dep.contains("0.1"),
        "version req should target 0.1.x, got {dep}"
    );

    let main = dir.join("main.drac");
    assert!(main.is_file(), "main.drac required");
    let body = fs::read_to_string(&main).unwrap();
    assert!(
        body.contains(PKG_LIB_MODULE),
        "main.drac must import module path {PKG_LIB_MODULE}:\n{body}"
    );
    assert!(
        body.contains("greet") && body.contains("VERSION"),
        "main.drac must use pkg-lib exports:\n{body}"
    );

    let readme = dir.join("README.md");
    assert!(readme.is_file(), "README.md required (documented build path)");
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

/// K10.02: documented path — local git URL for pkg-lib, resolve, compile consumer main, run.
#[test]
fn k10_02_pkg_consumer_documented_build_path() {
    let root = uniq_dir("build");
    let (upstream, oid) = tagged_upstream_from_pkg_lib(&root);
    let lib_url = upstream.to_str().expect("utf8 path");

    let src = pkg_consumer_dir();
    let ws = root.join("consumer");
    fs::create_dir_all(&ws).unwrap();

    // Start from in-repo consumer manifest; inject [urls] for local upstream.
    let base = parse_manifest(&fs::read_to_string(src.join(MANIFEST_FILE)).unwrap())
        .expect("consumer manifest");
    let mut manifest = base;
    manifest
        .urls
        .insert(PKG_LIB_MODULE.to_string(), lib_url.to_string());
    fs::write(ws.join(MANIFEST_FILE), write_manifest(&manifest)).unwrap();
    fs::copy(src.join("main.drac"), ws.join("main.drac")).unwrap();

    let cache = ModuleCache::new(default_cache_root(&ws));
    let lock = resolve_direct_deps(&manifest, &cache).expect("resolve+fetch pkg-lib");
    fs::write(ws.join(LOCK_FILE), write_lock(&lock)).unwrap();
    assert!(cache.has_entry(PKG_LIB_MODULE, &oid).unwrap());

    let main = ws.join("main.drac");
    let ir = compile_path(&main).expect("compile pkg-consumer + pkg-lib");
    let js = emit_js(&ir).expect("emit js");

    let node = Command::new("node")
        .arg("-e")
        .arg(js)
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
        stdout.contains("0.1.0"),
        "expected VERSION on stdout, got:\n{stdout}"
    );
    assert!(
        stdout.contains("hello, pkg-consumer"),
        "expected greet on stdout, got:\n{stdout}"
    );

    let _ = fs::remove_dir_all(&root);
}
