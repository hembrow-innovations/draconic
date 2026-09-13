//! Relative `[replace]` paths clone from the consumer workspace.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_js::emit_js;
use draconic_frontend::compile_path;
use draconic_pkg::{
    default_cache_root, get_package, mod_tidy, parse_lock, ModuleCache, LOCK_FILE, MANIFEST_FILE,
};

const LIB_PATH: &str = "github.com/org/lib";

fn uniq_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "draconic-pkg-replace-relative-{label}-{}-{nanos}",
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

fn tagged_vendor_lib(root: &Path, index_src: &str) -> (PathBuf, String) {
    let repo = root.join("vendor").join("lib");
    fs::create_dir_all(&repo).unwrap();
    git_ok(&["init"], &repo);
    git_ok(&["config", "user.email", "test@draconic.local"], &repo);
    git_ok(&["config", "user.name", "Draconic Test"], &repo);
    git_ok(&["checkout", "-B", "main"], &repo);
    fs::write(
        repo.join("draconic.toml"),
        "module = \"github.com/org/lib\"\n",
    )
    .unwrap();
    fs::write(repo.join("index.drac"), index_src).unwrap();
    git_ok(&["add", "."], &repo);
    git_ok(&["commit", "-m", "v1.0.0"], &repo);
    git_ok(&["tag", "v1.0.0"], &repo);
    let oid = head_oid(&repo);
    (repo, oid)
}

fn write_consumer_manifest(ws: &Path) {
    fs::create_dir_all(ws).unwrap();
    fs::write(
        ws.join(MANIFEST_FILE),
        r#"module = "github.com/acme/app"

[dependencies]
"github.com/org/lib" = "1.0.0"

[replace]
"github.com/org/lib" = { path = "../vendor/lib" }
"#,
    )
    .unwrap();
}

fn assert_pin_is_vendor(lock_path: &Path, vendor: &Path, oid: &str) {
    let lock = parse_lock(&fs::read_to_string(lock_path).unwrap()).unwrap();
    let e = lock.packages.get(LIB_PATH).expect("pin");
    assert_eq!(e.commit_oid, oid);
    let pin = PathBuf::from(&e.git_url)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(&e.git_url));
    let want = vendor.canonicalize().unwrap();
    assert_eq!(pin, want, "lock git_url should be the vendor tree");
    assert!(
        Path::new(&e.git_url).is_absolute(),
        "clone URL must be absolute after relative replace: {}",
        e.git_url
    );
}

#[test]
fn replace_relative_workspace_get_and_tidy_clone() {
    let root = uniq_dir("get-tidy");
    let (vendor, oid) = tagged_vendor_lib(&root, "export let marker = 7;\n");

    let get_ws = root.join("app-get");
    write_consumer_manifest(&get_ws);
    let get_cache = ModuleCache::new(root.join("cache-get"));
    let got = get_package(&get_ws, LIB_PATH, "1.0.0", None, &get_cache).expect("get relative replace");
    assert_eq!(got.commit_oid, oid);
    assert_eq!(
        fs::read_to_string(got.checkout_dir.join("index.drac")).unwrap(),
        "export let marker = 7;\n"
    );
    assert_pin_is_vendor(&get_ws.join(LOCK_FILE), &vendor, &oid);

    let tidy_ws = root.join("app-tidy");
    write_consumer_manifest(&tidy_ws);
    let tidy_cache = ModuleCache::new(root.join("cache-tidy"));
    let r = mod_tidy(&tidy_ws, &tidy_cache).expect("tidy relative replace");
    assert!(r.fetched.iter().any(|p| p == LIB_PATH), "{r:?}");
    assert_pin_is_vendor(&tidy_ws.join(LOCK_FILE), &vendor, &oid);
    let checkout = tidy_cache.entry_dir(LIB_PATH, &oid).unwrap();
    assert_eq!(
        fs::read_to_string(checkout.join("index.drac")).unwrap(),
        "export let marker = 7;\n"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn replace_relative_workspace_consumer_builds() {
    let root = uniq_dir("build");
    let (_vendor, oid) = tagged_vendor_lib(
        &root,
        "export let answer = 42;\nexport function add(a, b) { return a + b; }\n",
    );
    let ws = root.join("app");
    write_consumer_manifest(&ws);
    let cache = ModuleCache::new(default_cache_root(&ws));
    let got = get_package(&ws, LIB_PATH, "1.0.0", None, &cache).expect("get relative replace");
    assert_eq!(got.commit_oid, oid);

    let main = ws.join("main.drac");
    fs::write(
        &main,
        r#"import { answer, add } from "github.com/org/lib";
let sum = add(answer, 8);
let a = answer;
"#,
    )
    .unwrap();

    let ir = compile_path(&main).expect("frontend compile consumer+replace");
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

#[test]
fn replace_relative_invalid_fails_closed() {
    let root = uniq_dir("missing");
    let ws = root.join("app");
    write_consumer_manifest(&ws);
    let cache = ModuleCache::new(root.join("cache"));
    let err = get_package(&ws, LIB_PATH, "1.0.0", None, &cache)
        .expect_err("missing relative replace must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("github.com/org/lib") || msg.contains("vendor") || msg.contains("clone")
            || msg.contains("git")
            || msg.contains("invalid"),
        "diagnostic should name the failure: {msg}"
    );
    assert!(!ws.join(LOCK_FILE).exists(), "must not write lock on clone failure");

    let _ = fs::remove_dir_all(&root);
}
