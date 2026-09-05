//! ROADMAP R03: compiler integration supply-chain policy once K08 lands.
//!
//! K08 verifies lock hashes in `draconic-pkg`. R03.01 and R03.02 already lock
//! the child atoms. This parent harness proves both through `compile_path` so
//! a tampered cache and a lock hash mismatch cannot silently succeed.

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

const LIB_PATH: &str = "github.com/fixture/lib";
const BOGUS_HASH: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn temp_dir(label: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-r03-{}-{}-{}-{}",
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
    let manifest_src = format!(
        r#"module = "github.com/fixture/consumer"

[dependencies]
"{LIB_PATH}" = "1.0.0"

[urls]
"{LIB_PATH}" = "{lib_url}"
"#
    );
    let manifest = parse_manifest(&manifest_src).expect("manifest");
    fs::write(ws.join(MANIFEST_FILE), write_manifest(&manifest)).unwrap();
    ws
}

struct LockedConsumer {
    root: PathBuf,
    oid: String,
    main: PathBuf,
    cache: ModuleCache,
}

fn locked_consumer(label: &str) -> LockedConsumer {
    let root = temp_dir(label);
    let (upstream, oid) = tagged_lib_fixture(&root);
    let lib_url = upstream.to_str().expect("utf8 path");
    let ws = consumer_workspace(&root, lib_url);
    let cache = ModuleCache::new(default_cache_root(&ws));
    let lock = resolve_direct_deps(
        &parse_manifest(&fs::read_to_string(ws.join(MANIFEST_FILE)).unwrap()).unwrap(),
        &cache,
    )
    .expect("resolve+fetch");
    fs::write(ws.join(LOCK_FILE), write_lock(&lock)).unwrap();
    assert!(cache.has_entry(LIB_PATH, &oid).unwrap());

    let main = ws.join("main.drac");
    fs::write(
        &main,
        "import { answer } from \"github.com/fixture/lib\";\nlet a = answer;\n",
    )
    .unwrap();

    LockedConsumer {
        root,
        oid,
        main,
        cache,
    }
}

fn assert_integrity_refuse(msg: &str) {
    assert!(
        msg.contains("content hash mismatch"),
        "expected content hash mismatch, got: {msg}"
    );
    assert!(
        msg.contains("refuse") || msg.contains("wrong tree") || msg.contains("tampered"),
        "expected refuse/wrong-tree diagnostic, got: {msg}"
    );
    assert!(
        msg.contains(LIB_PATH),
        "diagnostic should name package: {msg}"
    );
}

/// R03: after lock pin, altering cache contents must hard-fail compile.
#[test]
fn compile_path_refuses_tampered_module_cache() {
    let fx = locked_consumer("tamper");
    compile_path(&fx.main).expect("untampered cache must compile");

    let checkout = fx.cache.entry_dir(LIB_PATH, &fx.oid).unwrap();
    fs::write(checkout.join("index.drac"), "export let answer = 666;\n").unwrap();

    let err = compile_path(&fx.main).expect_err("tampered cache must be refused");
    assert_integrity_refuse(&err.to_string());

    let _ = fs::remove_dir_all(&fx.root);
}

/// R03: rewriting lock `content_hash` must hard-fail compile.
#[test]
fn compile_path_hard_fails_lock_hash_mismatch() {
    let fx = locked_consumer("lock-hash");
    compile_path(&fx.main).expect("matching lock hash must compile");

    let lock_path = fx.main.parent().unwrap().join(LOCK_FILE);
    let mut lock = parse_lock(&fs::read_to_string(&lock_path).unwrap()).expect("lock");
    let entry = lock
        .packages
        .get_mut(LIB_PATH)
        .expect("locked fixture package");
    let original = entry.content_hash.clone();
    assert_ne!(
        original, BOGUS_HASH,
        "fixture hash must not already be the bogus pin"
    );
    entry.content_hash = BOGUS_HASH.to_string();
    fs::write(&lock_path, write_lock(&lock)).unwrap();

    let err = compile_path(&fx.main).expect_err("lock hash mismatch must hard-fail");
    let msg = err.to_string();
    assert_integrity_refuse(&msg);
    assert!(
        msg.contains(BOGUS_HASH),
        "diagnostic should name the lock pin: {msg}"
    );

    let _ = fs::remove_dir_all(&fx.root);
}

/// R03 parent remainder: one consumer, both attacks. Neither silently succeeds.
#[test]
fn compile_path_refuses_tamper_and_lock_hash_mismatch() {
    let fx = locked_consumer("both");
    compile_path(&fx.main).expect("honest lock and cache must compile");

    let checkout = fx.cache.entry_dir(LIB_PATH, &fx.oid).unwrap();
    let honest = fs::read_to_string(checkout.join("index.drac")).unwrap();
    fs::write(checkout.join("index.drac"), "export let answer = 666;\n").unwrap();
    let tamper_err = compile_path(&fx.main).expect_err("tampered cache must be refused");
    assert_integrity_refuse(&tamper_err.to_string());

    fs::write(checkout.join("index.drac"), honest).unwrap();
    compile_path(&fx.main).expect("restored cache must compile");

    let lock_path = fx.main.parent().unwrap().join(LOCK_FILE);
    let mut lock = parse_lock(&fs::read_to_string(&lock_path).unwrap()).expect("lock");
    lock.packages
        .get_mut(LIB_PATH)
        .expect("locked fixture package")
        .content_hash = BOGUS_HASH.to_string();
    fs::write(&lock_path, write_lock(&lock)).unwrap();

    let mismatch_err = compile_path(&fx.main).expect_err("lock hash mismatch must hard-fail");
    let msg = mismatch_err.to_string();
    assert_integrity_refuse(&msg);
    assert!(
        msg.contains(BOGUS_HASH),
        "diagnostic should name the lock pin: {msg}"
    );

    let _ = fs::remove_dir_all(&fx.root);
}
