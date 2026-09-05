//! K06.03: module-path imports coexist with E11 static relative imports.
//!
//! One Program graph may mix `./local.drac` relatives and `github.com/…`
//! package paths; package internals may keep relative edges; pure E11
//! relative graphs still link when a workspace lock is present.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_js::emit_js;
use draconic_frontend::compile_path;
use draconic_linker::{link_entry, link_entry_with_packages, PackageLinkContext};
use draconic_pkg::{
    content_hash_tree, default_cache_root, write_lock, LockEntry, LockFile, ModuleCache, LOCK_FILE,
};

fn uniq_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "draconic-pkg-k06_03-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn lock_ctx(root: &Path, module_path: &str, oid: &str, hash: &str) -> PackageLinkContext {
    let cache = ModuleCache::new(root.join("cache"));
    let entry = LockEntry::new(
        module_path,
        "1.0.0",
        &format!("https://{module_path}.git"),
        oid,
        hash,
    )
    .unwrap();
    let mut packages = BTreeMap::new();
    packages.insert(module_path.to_string(), entry);
    PackageLinkContext {
        lock: LockFile {
            version: 1,
            packages,
        },
        cache,
    }
}

/// Seed checkout files + marker; return content hash for the lock pin.
fn seed_pkg(cache: &ModuleCache, module_path: &str, oid: &str, files: &[(&str, &str)]) -> String {
    let pkg_dir = cache.entry_dir(module_path, oid).unwrap();
    fs::create_dir_all(&pkg_dir).unwrap();
    for (rel, src) in files {
        let path = pkg_dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, src).unwrap();
    }
    let hash = content_hash_tree(&pkg_dir).unwrap();
    fs::write(pkg_dir.join(".draconic-checkout-oid"), format!("{oid}\n")).unwrap();
    hash
}

/// Same entry mixes E11 relative + module-path imports.
#[test]
fn entry_mixes_relative_and_module_path() {
    let root = uniq_dir("mix-entry");
    fs::create_dir_all(&root).unwrap();
    let oid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let module_path = "github.com/org/pkg";
    let cache = ModuleCache::new(root.join("cache"));
    let hash = seed_pkg(
        &cache,
        module_path,
        oid,
        &[("index.drac", "export let fromPkg = 41;\n")],
    );
    let ctx = lock_ctx(&root, module_path, oid, &hash);

    fs::write(root.join("local.drac"), "export let fromLocal = 1;\n").unwrap();
    let main = root.join("main.drac");
    fs::write(
        &main,
        r#"import { fromLocal } from "./local.drac";
import { fromPkg } from "github.com/org/pkg";
let sum = fromLocal + fromPkg;
"#,
    )
    .unwrap();

    let program = link_entry_with_packages(&main, Some(&ctx)).expect("mixed link");
    let dump = draconic_ast::dump_program(&program);
    assert!(dump.contains("sum"), "{dump}");
    assert!(
        dump.contains("41") || dump.contains("fromPkg") || dump.contains("__m"),
        "{dump}"
    );
    assert!(
        dump.contains("1") || dump.contains("fromLocal") || dump.contains("__m"),
        "{dump}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// Local relative module itself imports a package path; entry only uses relative.
#[test]
fn relative_local_imports_module_path() {
    let root = uniq_dir("rel-to-pkg");
    fs::create_dir_all(&root).unwrap();
    let oid = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let module_path = "github.com/acme/lib";
    let cache = ModuleCache::new(root.join("cache"));
    let hash = seed_pkg(
        &cache,
        module_path,
        oid,
        &[("index.drac", "export let answer = 42;\n")],
    );
    let ctx = lock_ctx(&root, module_path, oid, &hash);

    fs::write(
        root.join("bridge.drac"),
        "import { answer } from \"github.com/acme/lib\";\nexport let doubled = answer * 2;\n",
    )
    .unwrap();
    let main = root.join("main.drac");
    fs::write(
        &main,
        "import { doubled } from \"./bridge.drac\";\nlet d = doubled;\n",
    )
    .unwrap();

    let program = link_entry_with_packages(&main, Some(&ctx)).expect("relative→package");
    let dump = draconic_ast::dump_program(&program);
    assert!(dump.contains("d"), "{dump}");
    assert!(
        dump.contains("42") || dump.contains("doubled") || dump.contains("__m"),
        "{dump}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// Package keeps internal relatives; consumer mixes local relative + package import.
#[test]
fn package_relative_internals_plus_consumer_relative() {
    let root = uniq_dir("pkg-internals");
    fs::create_dir_all(&root).unwrap();
    let oid = "cccccccccccccccccccccccccccccccccccccccc";
    let module_path = "github.com/org/math";
    let cache = ModuleCache::new(root.join("cache"));
    let hash = seed_pkg(
        &cache,
        module_path,
        oid,
        &[
            ("index.drac", "export { add } from \"./ops.drac\";\n"),
            ("ops.drac", "export function add(a, b) { return a + b; }\n"),
        ],
    );
    let ctx = lock_ctx(&root, module_path, oid, &hash);

    fs::write(root.join("scale.drac"), "export let scale = 10;\n").unwrap();
    let main = root.join("main.drac");
    fs::write(
        &main,
        r#"import { scale } from "./scale.drac";
import { add } from "github.com/org/math";
let n = add(scale, 2);
"#,
    )
    .unwrap();

    let program = link_entry_with_packages(&main, Some(&ctx)).expect("mixed graph");
    let dump = draconic_ast::dump_program(&program);
    assert!(dump.contains("n"), "{dump}");
    assert!(
        dump.contains("add") || dump.contains("scale") || dump.contains("__m"),
        "{dump}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// Workspace lock present: pure E11 relative graph still links (no package import).
#[test]
fn pure_relative_still_links_with_workspace_lock() {
    let root = uniq_dir("pure-rel");
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    let oid = "dddddddddddddddddddddddddddddddddddddddd";
    let module_path = "github.com/unused/dep";
    let cache = ModuleCache::new(default_cache_root(&root));
    let hash = seed_pkg(
        &cache,
        module_path,
        oid,
        &[("index.drac", "export let unused = 0;\n")],
    );
    let entry = LockEntry::new(
        module_path,
        "1.0.0",
        "https://github.com/unused/dep.git",
        oid,
        hash,
    )
    .unwrap();
    let mut packages = BTreeMap::new();
    packages.insert(module_path.to_string(), entry);
    let lock = LockFile {
        version: 1,
        packages,
    };
    fs::write(root.join(LOCK_FILE), write_lock(&lock)).unwrap();

    fs::write(src.join("lib.drac"), "export let value = 7;\n").unwrap();
    let main = src.join("main.drac");
    fs::write(
        &main,
        "import { value } from \"./lib.drac\";\nlet v = value;\n",
    )
    .unwrap();

    let program = link_entry(&main).expect("E11 relative with lock present");
    let dump = draconic_ast::dump_program(&program);
    assert!(dump.contains("v"), "{dump}");
    assert!(
        dump.contains("7") || dump.contains("value") || dump.contains("__m"),
        "{dump}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// Frontend compile path: mixed relative + module-path → IR → JS emit.
#[test]
fn frontend_compile_mixed_relative_and_module_path() {
    let root = uniq_dir("frontend-mix");
    fs::create_dir_all(&root).unwrap();
    let oid = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    let module_path = "github.com/org/greet";
    let cache = ModuleCache::new(default_cache_root(&root));
    let hash = seed_pkg(
        &cache,
        module_path,
        oid,
        &[(
            "index.drac",
            "export function greet(name) { return name; }\n",
        )],
    );
    let entry = LockEntry::new(
        module_path,
        "1.0.0",
        "https://github.com/org/greet.git",
        oid,
        hash,
    )
    .unwrap();
    let mut packages = BTreeMap::new();
    packages.insert(module_path.to_string(), entry);
    let lock = LockFile {
        version: 1,
        packages,
    };
    fs::write(root.join(LOCK_FILE), write_lock(&lock)).unwrap();

    fs::write(root.join("name.drac"), "export let who = \"world\";\n").unwrap();
    let main = root.join("main.drac");
    fs::write(
        &main,
        r#"import { who } from "./name.drac";
import { greet } from "github.com/org/greet";
let msg = greet(who);
"#,
    )
    .unwrap();

    let ir = compile_path(&main).expect("frontend compile mixed graph");
    let js = emit_js(&ir).expect("emit js");
    assert!(
        js.contains("greet") || js.contains("who") || js.contains("msg") || js.contains("world"),
        "{js}"
    );
    let _ = fs::remove_dir_all(&root);
}
