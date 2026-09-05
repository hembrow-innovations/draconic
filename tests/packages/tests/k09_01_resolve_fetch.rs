//! K09.01: fixture temp git lib (tagged) + consumer manifest+lock; resolve+fetch.
//!
//! End-to-end package path (no build/import yet — that is K09.02):
//! 1. Temp git upstream with exportable module source + semver tag
//! 2. Consumer workspace with `draconic.toml` (deps + local URL map)
//! 3. `resolve_direct_deps` → lock pins + cache checkout
//! 4. Lock written to disk; pin OID/hash match checkout tree

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_pkg::{
    content_hash_tree, default_cache_root, parse_lock, parse_manifest, resolve_direct_deps,
    write_lock, write_manifest, ModuleCache, LOCK_FILE, MANIFEST_FILE,
};

fn uniq_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "draconic-pkg-k09_01-{label}-{}-{nanos}",
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

/// Temp git lib: exportable `index.drac` + `draconic.toml`, tagged `v1.0.0`.
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
    fs::write(
        repo.join("index.drac"),
        "export let answer = 42;\nexport function add(a, b) { return a + b; }\n",
    )
    .unwrap();
    git_ok(&["add", "."], &repo);
    git_ok(&["commit", "-m", "v1.0.0"], &repo);
    let oid = head_oid(&repo);
    git_ok(&["tag", "v1.0.0"], &repo);
    (repo, oid)
}

/// Consumer workspace: manifest depends on fixture lib via local URL map.
fn write_consumer_manifest(ws: &Path, lib_path: &str, lib_url: &str) {
    fs::create_dir_all(ws).unwrap();
    let src = format!(
        r#"module = "github.com/fixture/consumer"

[dependencies]
"{lib_path}" = "1.0.0"

[urls]
"{lib_path}" = "{lib_url}"
"#
    );
    let m = parse_manifest(&src).expect("consumer manifest schema");
    fs::write(ws.join(MANIFEST_FILE), write_manifest(&m)).unwrap();
}

/// K09.01 happy path: tagged temp lib → consumer manifest → resolve+fetch → lock+cache.
#[test]
fn k09_01_tagged_lib_consumer_resolve_fetch() {
    let root = uniq_dir("happy");
    let (upstream, oid) = tagged_lib_fixture(&root);
    let lib_path = "github.com/fixture/lib";
    let lib_url = upstream.to_str().expect("utf8 path");

    let ws = root.join("consumer");
    write_consumer_manifest(&ws, lib_path, lib_url);

    let cache_root = default_cache_root(&ws);
    let cache = ModuleCache::new(&cache_root);
    let manifest_src = fs::read_to_string(ws.join(MANIFEST_FILE)).unwrap();
    let manifest = parse_manifest(&manifest_src).expect("parse consumer manifest");

    // Cold cache: resolve tags + fetch checkout.
    assert!(!cache.has_entry(lib_path, &oid).unwrap());
    let lock = resolve_direct_deps(&manifest, &cache).expect("resolve+fetch");

    assert_eq!(lock.version, 1);
    assert_eq!(lock.packages.len(), 1);
    let pin = lock.packages.get(lib_path).expect("lock pin for lib");
    assert_eq!(pin.path, lib_path);
    assert_eq!(pin.version, "1.0.0");
    assert_eq!(pin.commit_oid, oid);
    assert_eq!(pin.git_url, lib_url);
    assert_eq!(pin.content_hash.len(), 64);

    // Cache checkout materialised with lib sources.
    assert!(cache.has_entry(lib_path, &oid).unwrap());
    let checkout = cache.entry_dir(lib_path, &oid).unwrap();
    assert!(checkout.join("index.drac").is_file());
    assert!(checkout.join("draconic.toml").is_file());
    let body = fs::read_to_string(checkout.join("index.drac")).unwrap();
    assert!(body.contains("answer"), "{body}");
    assert_eq!(
        pin.content_hash,
        content_hash_tree(&checkout).expect("hash checkout")
    );

    // Persist lock on disk (consumer manifest+lock pair).
    let lock_text = write_lock(&lock);
    fs::write(ws.join(LOCK_FILE), &lock_text).unwrap();
    let lock_round = parse_lock(&fs::read_to_string(ws.join(LOCK_FILE)).unwrap()).unwrap();
    assert_eq!(lock_round.packages[lib_path].commit_oid, oid);
    assert_eq!(lock_round.packages[lib_path].content_hash, pin.content_hash);

    // Second resolve is cache-hit stable (same pin).
    let lock2 = resolve_direct_deps(&manifest, &cache).expect("resolve again");
    assert_eq!(
        lock2.packages[lib_path].commit_oid,
        lock.packages[lib_path].commit_oid
    );
    assert_eq!(
        lock2.packages[lib_path].content_hash,
        lock.packages[lib_path].content_hash
    );

    let _ = fs::remove_dir_all(&root);
}

/// K09.01: caret req on multi-tag fixture picks highest matching semver.
#[test]
fn k09_01_resolve_picks_highest_matching_tag() {
    let root = uniq_dir("caret");
    let repo = root.join("lib-upstream");
    fs::create_dir_all(&repo).unwrap();
    git_ok(&["init"], &repo);
    git_ok(&["config", "user.email", "test@draconic.local"], &repo);
    git_ok(&["config", "user.name", "Draconic Test"], &repo);
    git_ok(&["checkout", "-B", "main"], &repo);

    fs::write(repo.join("index.drac"), "export let v = 1;\n").unwrap();
    git_ok(&["add", "."], &repo);
    git_ok(&["commit", "-m", "v1.0.0"], &repo);
    git_ok(&["tag", "v1.0.0"], &repo);

    fs::write(repo.join("index.drac"), "export let v = 2;\n").unwrap();
    git_ok(&["add", "."], &repo);
    git_ok(&["commit", "-m", "v1.2.0"], &repo);
    let oid_120 = head_oid(&repo);
    git_ok(&["tag", "v1.2.0"], &repo);

    fs::write(repo.join("index.drac"), "export let v = 3;\n").unwrap();
    git_ok(&["add", "."], &repo);
    git_ok(&["commit", "-m", "v2.0.0"], &repo);
    git_ok(&["tag", "v2.0.0"], &repo);

    let lib_path = "github.com/fixture/semver-lib";
    let lib_url = repo.to_str().unwrap();
    let ws = root.join("consumer");
    fs::create_dir_all(&ws).unwrap();
    let src = format!(
        r#"module = "github.com/fixture/consumer"

[dependencies]
"{lib_path}" = "^1.0.0"

[urls]
"{lib_path}" = "{lib_url}"
"#
    );
    fs::write(ws.join(MANIFEST_FILE), &src).unwrap();
    let manifest = parse_manifest(&src).unwrap();
    let cache = ModuleCache::new(default_cache_root(&ws));

    let lock = resolve_direct_deps(&manifest, &cache).expect("resolve caret");
    let pin = &lock.packages[lib_path];
    assert_eq!(pin.version, "1.2.0");
    assert_eq!(pin.commit_oid, oid_120);
    assert_ne!(pin.version, "2.0.0");

    let checkout = cache.entry_dir(lib_path, &oid_120).unwrap();
    let body = fs::read_to_string(checkout.join("index.drac")).unwrap();
    assert!(body.contains("v = 2"), "{body}");

    fs::write(ws.join(LOCK_FILE), write_lock(&lock)).unwrap();
    let on_disk = parse_lock(&fs::read_to_string(ws.join(LOCK_FILE)).unwrap()).unwrap();
    assert_eq!(on_disk.packages[lib_path].version, "1.2.0");

    let _ = fs::remove_dir_all(&root);
}
