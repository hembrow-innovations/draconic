//! ROADMAP R03.02: compiler integration hard-fails a lock hash mismatch.
//!
//! K08 already verifies lock hashes in `draconic-pkg`. This test proves the
//! compiler surface (`compile_path`) does not silently emit a wrong tree when
//! the lock pin `content_hash` does not match the resolved checkout.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_frontend::compile_path;
use draconic_pkg::{
    default_cache_root, parse_lock, parse_manifest, resolve_direct_deps, write_lock,
    write_manifest, ModuleCache, LOCK_FILE, MANIFEST_FILE,
};

fn temp_dir(label: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-r03-02-{}-{}-{}-{}",
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

/// Temp git lib: exportable `index.drac` + tag `v1.0.0`.
fn tagged_lib_fixture(root: &Path) -> (PathBuf, String) {
    let repo = root.join("lib-upstream");
    fs::create_dir_all(&repo).unwrap();
    git_ok(&["init"], &repo);
    git_ok(&["config", "user.email", "test@draconic.local"], &repo);
    git_ok(&["config", "user.name", "Draconic Test"], &repo);
    git_ok(&["checkout", "-B", "main"], &repo);

    fs::write(
        repo.join("draconic.toml"),
        "module = \"github.com/fixture/lib\"\n",
    )
    .unwrap();
    fs::write(repo.join("index.drac"), "export let answer = 42;\n").unwrap();
    git_ok(&["add", "."], &repo);
    git_ok(&["commit", "-m", "v1.0.0"], &repo);
    let oid = head_oid(&repo);
    git_ok(&["tag", "v1.0.0"], &repo);
    (repo, oid)
}

fn consumer_workspace(root: &Path, lib_url: &str) -> PathBuf {
    let ws = root.join("consumer");
    fs::create_dir_all(&ws).unwrap();
    let lib_path = "github.com/fixture/lib";
    let manifest_src = format!(
        r#"module = "github.com/fixture/consumer"

[dependencies]
"{lib_path}" = "1.0.0"

[urls]
"{lib_path}" = "{lib_url}"
"#
    );
    let manifest = parse_manifest(&manifest_src).expect("manifest");
    fs::write(ws.join(MANIFEST_FILE), write_manifest(&manifest)).unwrap();
    ws
}

/// R03.02: after a matching lock pin, rewriting `content_hash` must hard-fail
/// compile. Matching-hash compile is the control that the fixture is valid.
#[test]
fn compile_path_hard_fails_lock_hash_mismatch() {
    let root = temp_dir("lock-hash");
    let (upstream, oid) = tagged_lib_fixture(&root);
    let lib_path = "github.com/fixture/lib";
    let lib_url = upstream.to_str().expect("utf8 path");

    let ws = consumer_workspace(&root, lib_url);
    let cache = ModuleCache::new(default_cache_root(&ws));
    let lock = resolve_direct_deps(
        &parse_manifest(&fs::read_to_string(ws.join(MANIFEST_FILE)).unwrap()).unwrap(),
        &cache,
    )
    .expect("resolve+fetch");
    fs::write(ws.join(LOCK_FILE), write_lock(&lock)).unwrap();
    assert!(cache.has_entry(lib_path, &oid).unwrap());

    let main = ws.join("main.drac");
    fs::write(
        &main,
        "import { answer } from \"github.com/fixture/lib\";\nlet a = answer;\n",
    )
    .unwrap();

    compile_path(&main).expect("matching lock hash must compile");

    let mut lock = parse_lock(&fs::read_to_string(ws.join(LOCK_FILE)).unwrap()).expect("lock");
    let entry = lock
        .packages
        .get_mut(lib_path)
        .expect("locked fixture package");
    let original = entry.content_hash.clone();
    let bogus = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    assert_ne!(
        original, bogus,
        "fixture hash must not already be the bogus pin"
    );
    entry.content_hash = bogus.to_string();
    fs::write(ws.join(LOCK_FILE), write_lock(&lock)).unwrap();

    let err = compile_path(&main).expect_err("lock hash mismatch must hard-fail");
    let msg = err.to_string();
    assert!(
        msg.contains("content hash mismatch"),
        "expected content hash mismatch, got: {msg}"
    );
    assert!(
        msg.contains("refuse") || msg.contains("wrong tree") || msg.contains("tampered"),
        "expected refuse/wrong-tree diagnostic, got: {msg}"
    );
    assert!(
        msg.contains(lib_path),
        "diagnostic should name package: {msg}"
    );
    assert!(
        msg.contains(bogus),
        "diagnostic should name the lock pin: {msg}"
    );

    let _ = fs::remove_dir_all(&root);
}
