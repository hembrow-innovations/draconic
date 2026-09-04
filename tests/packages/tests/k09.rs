//! K09: E2E temp git dep + consumer Program (parent of K09.01–K09.02).
//!
//! One honest package path on the compiler target:
//! 1. Temp git upstream lib with exportable module source + semver tag
//! 2. Consumer workspace with `draconic.toml` (deps + local URL map)
//! 3. `resolve_direct_deps` → lock pins + cache checkout (K09.01)
//! 4. Consumer Program `import { … } from "github.com/…"` compiles and Node
//!    observes imported values (K09.02)

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_js::emit_js;
use draconic_frontend::compile_path;
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
        "draconic-pkg-k09-{label}-{}-{nanos}",
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

/// K09 parent: tagged temp lib → resolve+fetch lock/cache → consumer Program import.
#[test]
fn k09_e2e_temp_git_dep_consumer_program() {
    let root = uniq_dir("e2e");
    let (upstream, oid) = tagged_lib_fixture(&root);
    let lib_path = "github.com/fixture/lib";
    let lib_url = upstream.to_str().expect("utf8 path");

    let ws = root.join("consumer");
    fs::create_dir_all(&ws).unwrap();
    let manifest_src = format!(
        r#"module = "github.com/fixture/consumer"

[dependencies]
"{lib_path}" = "1.0.0"

[urls]
"{lib_path}" = "{lib_url}"
"#
    );
    let manifest = parse_manifest(&manifest_src).expect("consumer manifest");
    fs::write(ws.join(MANIFEST_FILE), write_manifest(&manifest)).unwrap();

    let cache_root = default_cache_root(&ws);
    let cache = ModuleCache::new(&cache_root);
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

    fs::write(ws.join(LOCK_FILE), write_lock(&lock)).unwrap();
    let lock_round = parse_lock(&fs::read_to_string(ws.join(LOCK_FILE)).unwrap()).unwrap();
    assert_eq!(lock_round.packages[lib_path].commit_oid, oid);
    assert_eq!(lock_round.packages[lib_path].content_hash, pin.content_hash);

    let main = ws.join("main.drac");
    fs::write(
        &main,
        r#"import { answer, add } from "github.com/fixture/lib";
let sum = add(answer, 8);
let a = answer;
"#,
    )
    .unwrap();

    let ir = compile_path(&main).expect("frontend compile consumer+package");
    let js = emit_js(&ir).expect("emit js");

    let node = Command::new("node")
        .arg("-e")
        .arg(format!(
            "{js}\nif (typeof a !== 'number' || a !== 42) {{ console.error('a', a); process.exit(1); }}\nif (typeof sum !== 'number' || sum !== 50) {{ console.error('sum', sum); process.exit(1); }}"
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
