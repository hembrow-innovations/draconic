//! Module-path plus subpath after a real tagged fetch (not a seeded cache).
//!
//! `from "github.com/org/pkg/util"` must compile. Root-only resolve is not enough.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_js::emit_js;
use draconic_frontend::compile_path;
use draconic_pkg::{
    content_hash_tree, default_cache_root, parse_manifest, resolve_direct_deps, write_lock,
    write_manifest, ModuleCache, LOCK_FILE, MANIFEST_FILE,
};

fn uniq_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "draconic-pkg-subpath-{label}-{}-{nanos}",
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

fn tagged_multi_file_lib(root: &Path) -> (PathBuf, String) {
    let repo = root.join("lib-upstream");
    fs::create_dir_all(&repo).unwrap();
    git_ok(&["init"], &repo);
    git_ok(&["config", "user.email", "test@draconic.local"], &repo);
    git_ok(&["config", "user.name", "Draconic Test"], &repo);
    git_ok(&["checkout", "-B", "main"], &repo);

    fs::write(
        repo.join("draconic.toml"),
        "module = \"github.com/org/pkg\"\n",
    )
    .unwrap();
    fs::write(
        repo.join("index.drac"),
        "export const ROOT = \"root-only\";\n",
    )
    .unwrap();
    fs::write(
        repo.join("util.drac"),
        "export const TAG = \"util\";\nexport function scale(n) { return n * 2; }\n",
    )
    .unwrap();
    git_ok(&["add", "."], &repo);
    git_ok(&["commit", "-m", "v1.0.0"], &repo);
    let oid = head_oid(&repo);
    git_ok(&["tag", "v1.0.0"], &repo);
    (repo, oid)
}

/// Fetch a tagged multi-file tree, then compile `from "github.com/org/pkg/util"`.
#[test]
fn subpath_tagged_multi_file_fetch_compiles() {
    let root = uniq_dir("e2e");
    let (upstream, oid) = tagged_multi_file_lib(&root);
    let lib_path = "github.com/org/pkg";
    let lib_url = upstream.to_str().expect("utf8 path");

    let ws = root.join("consumer");
    fs::create_dir_all(&ws).unwrap();
    let manifest_src = format!(
        r#"module = "github.com/org/consumer"

[dependencies]
"{lib_path}" = "1.0.0"

[urls]
"{lib_path}" = "{lib_url}"
"#
    );
    let manifest = parse_manifest(&manifest_src).expect("manifest");
    fs::write(ws.join(MANIFEST_FILE), write_manifest(&manifest)).unwrap();

    let cache = ModuleCache::new(default_cache_root(&ws));
    assert!(!cache.has_entry(lib_path, &oid).unwrap());

    let lock = resolve_direct_deps(&manifest, &cache).expect("fetch tagged multi-file tree");
    fs::write(ws.join(LOCK_FILE), write_lock(&lock)).unwrap();
    assert!(cache.has_entry(lib_path, &oid).unwrap());

    let pin = lock.packages.get(lib_path).expect("pin");
    assert_eq!(pin.commit_oid, oid);
    let checkout = cache.entry_dir(lib_path, &oid).unwrap();
    assert!(
        checkout.join("index.drac").is_file() && checkout.join("util.drac").is_file(),
        "fetched tree must contain more than one .drac file"
    );
    assert_eq!(
        pin.content_hash,
        content_hash_tree(&checkout).expect("hash"),
        "integrity hash must match the fetched tree"
    );

    let main = ws.join("main.drac");
    fs::write(
        &main,
        r#"import { scale, TAG } from "github.com/org/pkg/util";
let x = scale(21);
let tag = TAG;
"#,
    )
    .unwrap();

    let ir = compile_path(&main).expect("compile subpath import after fetch");
    let js = emit_js(&ir).expect("emit js");
    assert!(
        !js.contains("root-only"),
        "subpath import must not load only the package root:\n{js}"
    );

    let node = Command::new("node")
        .arg("-e")
        .arg(format!(
            r#"{js}
if (typeof x !== "number" || x !== 42) {{ console.error("scale", x); process.exit(1); }}
if (typeof tag !== "string" || tag !== "util") {{ console.error("TAG", tag); process.exit(2); }}
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
