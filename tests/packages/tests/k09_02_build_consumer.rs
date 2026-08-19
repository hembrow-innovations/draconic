//! K09.02: E2E build consumer importing module path from fixture.
//!
//! Full chain on top of K09.01 resolve+fetch:
//! 1. Temp git upstream lib (tagged)
//! 2. Consumer manifest + `resolve_direct_deps` → lock + cache
//! 3. Consumer Program `import { … } from "github.com/…"`
//! 4. Frontend compile → JS emit; Node observes imported values

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

fn uniq_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "draconic-pkg-k09_02-{label}-{}-{nanos}",
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

/// K09.02: resolve+fetch fixture lib, then compile consumer that imports it.
#[test]
fn k09_02_build_consumer_importing_module_path() {
    let root = uniq_dir("build");
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
    let manifest = parse_manifest(&manifest_src).expect("manifest");
    fs::write(ws.join(MANIFEST_FILE), write_manifest(&manifest)).unwrap();

    let cache = ModuleCache::new(default_cache_root(&ws));
    let lock = resolve_direct_deps(&manifest, &cache).expect("resolve+fetch");
    fs::write(ws.join(LOCK_FILE), write_lock(&lock)).unwrap();

    assert!(cache.has_entry(lib_path, &oid).unwrap());
    let pin = lock.packages.get(lib_path).expect("pin");
    assert_eq!(pin.commit_oid, oid);

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
    assert!(
        js.contains("42") || js.contains("answer") || js.contains("add") || js.contains("sum"),
        "emitted js missing package surface:\n{js}"
    );

    // Node observes linked package values (not just string presence).
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

/// K09.02: cold cache — lock on disk only; compile still needs checkout present
/// (caller/ensure path). Here resolve already filled cache; second compile is stable.
#[test]
fn k09_02_rebuild_with_warm_cache_is_stable() {
    let root = uniq_dir("rebuild");
    let (upstream, oid) = tagged_lib_fixture(&root);
    let lib_path = "github.com/fixture/lib";
    let lib_url = upstream.to_str().unwrap();

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
    let manifest = parse_manifest(&manifest_src).unwrap();
    fs::write(ws.join(MANIFEST_FILE), write_manifest(&manifest)).unwrap();
    let cache = ModuleCache::new(default_cache_root(&ws));
    let lock = resolve_direct_deps(&manifest, &cache).unwrap();
    fs::write(ws.join(LOCK_FILE), write_lock(&lock)).unwrap();

    let main = ws.join("main.drac");
    fs::write(
        &main,
        "import { answer } from \"github.com/fixture/lib\";\nlet a = answer;\n",
    )
    .unwrap();

    let js1 = emit_js(&compile_path(&main).expect("compile 1")).expect("emit 1");
    let js2 = emit_js(&compile_path(&main).expect("compile 2")).expect("emit 2");
    assert_eq!(js1, js2, "warm-cache rebuild must be byte-stable");
    assert!(cache.has_entry(lib_path, &oid).unwrap());

    let node = Command::new("node")
        .arg("-e")
        .arg(format!(
            "{js1}\nif (a !== 42) {{ console.error(a); process.exit(1); }}"
        ))
        .output()
        .expect("node");
    assert!(
        node.status.success(),
        "node: {}",
        String::from_utf8_lossy(&node.stderr)
    );

    let _ = fs::remove_dir_all(&root);
}
