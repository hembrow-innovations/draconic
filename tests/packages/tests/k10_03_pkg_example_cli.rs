//! K10: in-repo pkg-lib / pkg-consumer via `draconic get` and `draconic build`.
//!
//! Layout tests are not enough. This path copies the committed examples into
//! temp dirs, tags a local git upstream, then runs the CLI (not compile_path).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_pkg::{parse_lock, parse_manifest, LOCK_FILE, MANIFEST_FILE};

const PKG_LIB_MODULE: &str = "github.com/draconic-lang/pkg-lib";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn draconic_bin() -> PathBuf {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let bin = repo_root().join("target").join(profile).join("draconic");
    assert!(
        bin.is_file(),
        "missing {} (build draconic-cli first)",
        bin.display()
    );
    bin
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
        "draconic-pkg-k10_03-{label}-{}-{nanos}",
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

fn tagged_upstream_from_pkg_lib(root: &Path) -> PathBuf {
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
    git_ok(&["tag", "v0.1.0"], &repo);
    repo
}

fn run_draconic(args: &[&str], extra_env: &[(&str, &str)]) -> (i32, String, String) {
    let mut cmd = Command::new(draconic_bin());
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let output = cmd.output().expect("spawn draconic");
    (
        output.status.code().unwrap_or(1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Example consumer `draconic get` then `draconic build -o` (not layout-only).
#[test]
fn k10_03_pkg_example_get_and_build() {
    let root = uniq_dir("cli");
    let upstream = tagged_upstream_from_pkg_lib(&root);
    let lib_url = upstream.to_str().expect("utf8 path");

    let src = pkg_consumer_dir();
    let ws = root.join("consumer");
    fs::create_dir_all(&ws).unwrap();
    fs::copy(src.join(MANIFEST_FILE), ws.join(MANIFEST_FILE)).unwrap();
    fs::copy(src.join("main.drac"), ws.join("main.drac")).unwrap();

    let in_repo_lock = src.join(LOCK_FILE);
    assert!(
        !in_repo_lock.exists(),
        "must not emit lock next to examples: {}",
        in_repo_lock.display()
    );

    let (code, stdout, stderr) = run_draconic(
        &[
            "get",
            &format!("{PKG_LIB_MODULE}@0.1.0"),
            "--url",
            lib_url,
            "--dir",
            ws.to_str().unwrap(),
        ],
        &[],
    );
    assert_eq!(code, 0, "get failed: stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains(PKG_LIB_MODULE),
        "get stdout should name the module path:\n{stdout}"
    );

    let manifest = parse_manifest(&fs::read_to_string(ws.join(MANIFEST_FILE)).unwrap())
        .expect("consumer manifest after get");
    assert!(
        manifest.dependencies.contains_key(PKG_LIB_MODULE),
        "get must keep the example library dep"
    );
    let lock =
        parse_lock(&fs::read_to_string(ws.join(LOCK_FILE)).unwrap()).expect("lock after get");
    assert!(
        lock.packages.contains_key(PKG_LIB_MODULE),
        "get must write a lock pin for {PKG_LIB_MODULE}"
    );

    let out = root.join("pkg-consumer.out.js");
    let main = ws.join("main.drac");
    let (code, stdout, stderr) = run_draconic(
        &[
            "build",
            "--target",
            "js",
            main.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ],
        &[],
    );
    assert_eq!(code, 0, "build failed: stdout={stdout}\nstderr={stderr}");
    assert!(out.is_file(), "build -o must write {}", out.display());
    assert!(
        !src.join("main.out.js").exists(),
        "must not emit scratch next to examples"
    );

    let node = Command::new("node").arg(&out).output().expect("spawn node");
    assert!(
        node.status.success(),
        "node failed: stdout={} stderr={}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
    let node_out = String::from_utf8_lossy(&node.stdout);
    assert!(
        node_out.contains("0.1.0"),
        "expected VERSION on stdout, got:\n{node_out}"
    );
    assert!(
        node_out.contains("hello, pkg-consumer"),
        "expected greet on stdout, got:\n{node_out}"
    );

    let _ = fs::remove_dir_all(&root);
}
