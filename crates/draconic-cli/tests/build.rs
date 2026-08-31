//! ROADMAP B10: `draconic build --target js|native` end-to-end.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn draconic() -> Command {
    Command::new(env!("CARGO_BIN_EXE_draconic"))
}

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-cli-build-{}-{}-{}",
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

fn write_program(dir: &Path, name: &str, src: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, src).unwrap();
    path
}

fn run_ok(cmd: &mut Command) -> (String, String) {
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn draconic");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "command failed: {:?}\nstdout={stdout}\nstderr={stderr}",
        cmd
    );
    (stdout, stderr)
}

#[test]
fn build_target_js_writes_runnable_js() {
    let dir = temp_dir();
    let src = write_program(&dir, "prog.drac", "let x = 1 + 2;");
    let out = dir.join("out.js");

    run_ok(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("js")
            .arg(&src)
            .arg("-o")
            .arg(&out),
    );

    let js = fs::read_to_string(&out).expect("js output");
    assert!(js.contains("let x"), "emitted js:\n{js}");

    let node = Command::new("node")
        .arg("-e")
        .arg(format!(
            "{js}\nif (x !== 3) {{ console.error(x); process.exit(1); }}"
        ))
        .output()
        .expect("node");
    assert!(
        node.status.success(),
        "node failed: {}",
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn build_target_native_writes_runnable_binary() {
    let dir = temp_dir();
    // Real native path (N01), not the empty-program hello demo.
    let src = write_program(&dir, "prog.drac", "let x: i32 = 42;");
    let out = dir.join("prog");

    run_ok(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("native")
            .arg(&src)
            .arg("-o")
            .arg(&out),
    );

    assert!(out.is_file(), "native binary missing at {}", out.display());

    let output = Command::new(&out).output().expect("run native binary");
    assert!(
        output.status.success(),
        "native exit {:?}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "42\n", "stdout={stdout:?}");
}

#[test]
fn build_target_native_rejects_unsupported_js() {
    let dir = temp_dir();
    let src = write_program(&dir, "prog.drac", "let x = {};");
    let out = dir.join("prog");

    let output = draconic()
        .arg("build")
        .arg("--target")
        .arg("native")
        .arg(&src)
        .arg("-o")
        .arg(&out)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn draconic");
    assert!(
        !output.status.success(),
        "unsupported JS must fail native emit"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unsupported") || stderr.contains("native target"),
        "stderr={stderr}"
    );
}

#[test]
fn build_js_default_output_next_to_source() {
    let dir = temp_dir();
    let src = write_program(&dir, "hello.drac", "let n = 0;");

    run_ok(draconic().arg("build").arg("--target").arg("js").arg(&src));

    let default_out = dir.join("hello.js");
    assert!(
        default_out.is_file(),
        "expected default JS output {}",
        default_out.display()
    );
}

#[test]
fn build_native_default_output_next_to_source() {
    let dir = temp_dir();
    let src = write_program(&dir, "hello.drac", "let n: i32 = 0;");

    run_ok(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("native")
            .arg(&src),
    );

    let default_out = dir.join("hello");
    assert!(
        default_out.is_file(),
        "expected default native output {}",
        default_out.display()
    );
}

#[test]
fn build_rejects_missing_target() {
    let dir = temp_dir();
    let src = write_program(&dir, "p.drac", "let x = 1;");
    let output = draconic().arg("build").arg(&src).output().expect("spawn");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("target") || stderr.contains("usage"),
        "stderr={stderr}"
    );
}

#[test]
fn build_rejects_unknown_target() {
    let dir = temp_dir();
    let src = write_program(&dir, "p.drac", "let x = 1;");
    let output = draconic()
        .arg("build")
        .arg("--target")
        .arg("wasm")
        .arg(&src)
        .output()
        .expect("spawn");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("wasm") || stderr.contains("target"),
        "stderr={stderr}"
    );
}

#[test]
fn build_reports_parse_error() {
    let dir = temp_dir();
    let src = write_program(&dir, "bad.drac", "let = ;");
    let output = draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg(&src)
        .output()
        .expect("spawn");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error"), "stderr={stderr}");
}

/// F04.01: `build --target native --link lib.a` resolves one C symbol from the archive.
#[test]
fn build_native_link_static_lib_resolves_symbol() {
    use draconic_backend_llvm::build_c_static_lib;

    let dir = temp_dir();
    let c_src = dir.join("touch.c");
    fs::write(&c_src, "void draconic_link_static_touch(void) {}\n").unwrap();
    let archive = dir.join("libtouch.a");
    build_c_static_lib(&c_src, &archive).expect("build .a");

    let src = write_program(
        &dir,
        "prog.drac",
        "extern \"C\" function draconic_link_static_touch(): void;\ndraconic_link_static_touch();\nlet x: i32 = 1;\n",
    );
    let out_fail = dir.join("fail");
    let failed = draconic()
        .arg("build")
        .arg("--target")
        .arg("native")
        .arg(&src)
        .arg("-o")
        .arg(&out_fail)
        .output()
        .expect("spawn");
    assert!(
        !failed.status.success(),
        "build without --link must fail to resolve the C symbol"
    );
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(
        stderr.contains("draconic_link_static_touch")
            || stderr.contains("undefined")
            || stderr.contains("Unresolved"),
        "stderr={stderr}"
    );

    let out = dir.join("prog");
    run_ok(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("native")
            .arg("--link")
            .arg(&archive)
            .arg(&src)
            .arg("-o")
            .arg(&out),
    );
    assert!(out.is_file(), "native binary missing at {}", out.display());
}

/// F05.01: `build --target native --link lib.dylib` resolves one C symbol from the shared lib.
#[test]
fn build_native_link_dynamic_lib_resolves_symbol() {
    use draconic_backend_llvm::{build_c_dynamic_lib, dynamic_lib_file_name};

    let dir = temp_dir();
    let c_src = dir.join("touch.c");
    fs::write(&c_src, "void draconic_link_dynamic_touch(void) {}\n").unwrap();
    let dylib = dir.join(dynamic_lib_file_name("touch"));
    build_c_dynamic_lib(&c_src, &dylib).expect("build shared lib");

    let src = write_program(
        &dir,
        "prog.drac",
        "extern \"C\" function draconic_link_dynamic_touch(): void;\ndraconic_link_dynamic_touch();\nlet x: i32 = 1;\n",
    );
    let out_fail = dir.join("fail");
    let failed = draconic()
        .arg("build")
        .arg("--target")
        .arg("native")
        .arg(&src)
        .arg("-o")
        .arg(&out_fail)
        .output()
        .expect("spawn");
    assert!(
        !failed.status.success(),
        "build without --link must fail to resolve the C symbol"
    );
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(
        stderr.contains("draconic_link_dynamic_touch")
            || stderr.contains("undefined")
            || stderr.contains("Unresolved"),
        "stderr={stderr}"
    );

    let out = dir.join("prog");
    run_ok(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("native")
            .arg("--link")
            .arg(&dylib)
            .arg(&src)
            .arg("-o")
            .arg(&out),
    );
    assert!(out.is_file(), "native binary missing at {}", out.display());
}

/// F05.02: `build --target native --link lib.dylib` then run: stdout is the C return value.
#[test]
fn build_native_link_dynamic_lib_call_end_to_end() {
    use draconic_backend_llvm::{build_c_dynamic_lib, dynamic_lib_file_name};

    let dir = temp_dir();
    let c_src = dir.join("add.c");
    fs::write(
        &c_src,
        "int draconic_link_dynamic_add(int a, int b) { return a + b; }\n",
    )
    .unwrap();
    let dylib = dir.join(dynamic_lib_file_name("add"));
    build_c_dynamic_lib(&c_src, &dylib).expect("build shared lib");

    let src = write_program(
        &dir,
        "prog.drac",
        "extern \"C\" function draconic_link_dynamic_add(a: i32, b: i32): i32;\nlet s: i32 = draconic_link_dynamic_add(20, 22);\nlet t: i32 = draconic_link_dynamic_add(-5, 12);\n",
    );
    let out = dir.join("prog");
    run_ok(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("native")
            .arg("--link")
            .arg(&dylib)
            .arg(&src)
            .arg("-o")
            .arg(&out),
    );
    let output = Command::new(&out).output().expect("run");
    assert!(
        output.status.success(),
        "exit {:?}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "42\n7\n",
        "stdout must be C-computed returns"
    );
}

/// F05.02: `--link` of a missing shared lib is E0402, not a raw linker dump.
#[test]
fn build_native_link_dynamic_lib_missing_is_typed_error() {
    use draconic_backend_llvm::dynamic_lib_file_name;

    let dir = temp_dir();
    let src = write_program(
        &dir,
        "prog.drac",
        "extern \"C\" function draconic_link_dynamic_add(a: i32, b: i32): i32;\nlet s: i32 = draconic_link_dynamic_add(20, 22);\n",
    );
    let missing = dir.join(dynamic_lib_file_name("no_such"));
    let out_fail = dir.join("fail");
    let failed = draconic()
        .arg("build")
        .arg("--target")
        .arg("native")
        .arg("--link")
        .arg(&missing)
        .arg(&src)
        .arg("-o")
        .arg(&out_fail)
        .output()
        .expect("spawn");
    assert!(
        !failed.status.success(),
        "build with missing --link dylib must fail"
    );
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(
        stderr.contains("E0402"),
        "missing dylib must be typed E0402, stderr={stderr}"
    );
    assert!(stderr.contains("dynamic lib not found"), "stderr={stderr}");
}

/// F04.02: `build --target native --link lib.a` then run: stdout is the C return value.
#[test]
fn build_native_link_static_lib_call_end_to_end() {
    use draconic_backend_llvm::build_c_static_lib;

    let dir = temp_dir();
    let c_src = dir.join("add.c");
    fs::write(
        &c_src,
        "int draconic_link_static_add(int a, int b) { return a + b; }\n",
    )
    .unwrap();
    let archive = dir.join("libadd.a");
    build_c_static_lib(&c_src, &archive).expect("build .a");

    let src = write_program(
        &dir,
        "prog.drac",
        "extern \"C\" function draconic_link_static_add(a: i32, b: i32): i32;\nlet s: i32 = draconic_link_static_add(20, 22);\nlet t: i32 = draconic_link_static_add(-5, 12);\n",
    );
    let out = dir.join("prog");
    run_ok(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("native")
            .arg("--link")
            .arg(&archive)
            .arg(&src)
            .arg("-o")
            .arg(&out),
    );
    let output = Command::new(&out).output().expect("run");
    assert!(
        output.status.success(),
        "exit {:?}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "42\n7\n",
        "stdout must be C-computed returns"
    );
}

/// H17.01: `examples/http-echo` builds pure native (no C host).
#[test]
fn build_examples_http_echo_native() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest_dir
        .join("../..")
        .join("examples/http-echo/main.drac");
    let src = src.canonicalize().expect("examples/http-echo/main.drac");
    let dir = temp_dir();
    let out = dir.join("http-echo");

    run_ok(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("native")
            .arg(&src)
            .arg("-o")
            .arg(&out),
    );

    assert!(out.is_file(), "native binary missing at {}", out.display());
    let meta = fs::metadata(&out).expect("metadata");
    assert!(meta.len() > 0, "empty binary");
}

/// ROADMAP K07.01: `draconic build` auto-fetches missing locked cache entries.
#[test]
fn build_auto_fetches_missing_locked_cache() {
    let root = temp_dir();

    // Upstream fixture package (tagged).
    let upstream = root.join("upstream");
    fs::create_dir_all(&upstream).unwrap();
    git_ok(&["init"], &upstream);
    git_ok(&["config", "user.email", "test@draconic.local"], &upstream);
    git_ok(&["config", "user.name", "Draconic Test"], &upstream);
    git_ok(&["checkout", "-B", "main"], &upstream);
    fs::write(
        upstream.join("index.drac"),
        "export let value = 41;\nexport function inc(x) { return x + 1; }\n",
    )
    .unwrap();
    git_ok(&["add", "."], &upstream);
    git_ok(&["commit", "-m", "v1.0.0"], &upstream);
    git_ok(&["tag", "v1.0.0"], &upstream);
    let oid = git_stdout(&["rev-parse", "HEAD"], &upstream);

    // Populate cache once to compute content hash, then wipe cache.
    let seed_cache = root.join("seed-cache");
    let (code, _stdout, stderr) = run_code(
        draconic()
            .arg("get")
            .arg("github.com/org/lib@1.0.0")
            .arg("--url")
            .arg(upstream.to_str().unwrap())
            .arg("--dir")
            .arg({
                let ws = root.join("seed-ws");
                fs::create_dir_all(&ws).unwrap();
                fs::write(
                    ws.join("draconic.toml"),
                    "module = \"github.com/acme/seed\"\n",
                )
                .unwrap();
                ws
            })
            .arg("--cache-dir")
            .arg(&seed_cache),
    );
    assert_eq!(code, 0, "seed get failed: {stderr}");
    let lock_src = fs::read_to_string(root.join("seed-ws/draconic.lock")).unwrap();
    let content_hash = lock_src
        .lines()
        .find_map(|l| l.trim().strip_prefix("content_hash = \""))
        .and_then(|s| s.strip_suffix('"'))
        .expect("content_hash in seed lock")
        .to_string();

    // Consumer workspace: lock present, default cache empty.
    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    fs::write(
        ws.join("draconic.toml"),
        format!(
            "module = \"github.com/acme/app\"\n\n[dependencies]\n\"github.com/org/lib\" = \"1.0.0\"\n\n[urls]\n\"github.com/org/lib\" = \"{}\"\n",
            upstream.display()
        ),
    )
    .unwrap();
    fs::write(
        ws.join("draconic.lock"),
        format!(
            r#"version = 1

[[package]]
path = "github.com/org/lib"
version = "1.0.0"
git_url = "{}"
commit_oid = "{oid}"
content_hash = "{content_hash}"
"#,
            upstream.display()
        ),
    )
    .unwrap();
    let main = ws.join("main.drac");
    fs::write(
        &main,
        "import { value, inc } from \"github.com/org/lib\";\nlet a = value;\nlet b = inc(value);\n",
    )
    .unwrap();

    let cache_mod = ws
        .join(".draconic/mod-cache/mod/github.com/org/lib")
        .join(&oid);
    assert!(
        !cache_mod.is_dir(),
        "cache must be empty before build auto-fetch"
    );

    let out = ws.join("out.js");
    run_ok(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("js")
            .arg(&main)
            .arg("-o")
            .arg(&out),
    );

    assert!(
        cache_mod.is_dir(),
        "build should materialize locked checkout at {}",
        cache_mod.display()
    );
    let js = fs::read_to_string(&out).expect("js");
    assert!(
        js.contains("41") || js.contains("value") || js.contains("inc"),
        "{js}"
    );

    let _ = fs::remove_dir_all(&root);
}

/// ROADMAP K07.02: `draconic build --offline` uses cache only; missing pin → fixit, no fetch.
#[test]
fn build_offline_fails_when_cache_missing() {
    let root = temp_dir();

    let upstream = root.join("upstream");
    fs::create_dir_all(&upstream).unwrap();
    git_ok(&["init"], &upstream);
    git_ok(&["config", "user.email", "test@draconic.local"], &upstream);
    git_ok(&["config", "user.name", "Draconic Test"], &upstream);
    git_ok(&["checkout", "-B", "main"], &upstream);
    fs::write(
        upstream.join("index.drac"),
        "export let value = 41;\nexport function inc(x) { return x + 1; }\n",
    )
    .unwrap();
    git_ok(&["add", "."], &upstream);
    git_ok(&["commit", "-m", "v1.0.0"], &upstream);
    git_ok(&["tag", "v1.0.0"], &upstream);
    let oid = git_stdout(&["rev-parse", "HEAD"], &upstream);

    let seed_cache = root.join("seed-cache");
    let (code, _stdout, stderr) = run_code(
        draconic()
            .arg("get")
            .arg("github.com/org/lib@1.0.0")
            .arg("--url")
            .arg(upstream.to_str().unwrap())
            .arg("--dir")
            .arg({
                let ws = root.join("seed-ws");
                fs::create_dir_all(&ws).unwrap();
                fs::write(
                    ws.join("draconic.toml"),
                    "module = \"github.com/acme/seed\"\n",
                )
                .unwrap();
                ws
            })
            .arg("--cache-dir")
            .arg(&seed_cache),
    );
    assert_eq!(code, 0, "seed get failed: {stderr}");
    let lock_src = fs::read_to_string(root.join("seed-ws/draconic.lock")).unwrap();
    let content_hash = lock_src
        .lines()
        .find_map(|l| l.trim().strip_prefix("content_hash = \""))
        .and_then(|s| s.strip_suffix('"'))
        .expect("content_hash in seed lock")
        .to_string();

    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    fs::write(
        ws.join("draconic.toml"),
        format!(
            "module = \"github.com/acme/app\"\n\n[dependencies]\n\"github.com/org/lib\" = \"1.0.0\"\n\n[urls]\n\"github.com/org/lib\" = \"{}\"\n",
            upstream.display()
        ),
    )
    .unwrap();
    fs::write(
        ws.join("draconic.lock"),
        format!(
            r#"version = 1

[[package]]
path = "github.com/org/lib"
version = "1.0.0"
git_url = "{}"
commit_oid = "{oid}"
content_hash = "{content_hash}"
"#,
            upstream.display()
        ),
    )
    .unwrap();
    let main = ws.join("main.drac");
    fs::write(
        &main,
        "import { value, inc } from \"github.com/org/lib\";\nlet a = value;\nlet b = inc(value);\n",
    )
    .unwrap();

    let cache_mod = ws
        .join(".draconic/mod-cache/mod/github.com/org/lib")
        .join(&oid);
    assert!(
        !cache_mod.is_dir(),
        "cache must be empty before offline build"
    );

    let out = ws.join("out.js");
    let (code, _stdout, stderr) = run_code(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("js")
            .arg("--offline")
            .arg(&main)
            .arg("-o")
            .arg(&out),
    );
    assert_ne!(code, 0, "offline build must fail when cache missing");
    assert!(
        stderr.contains("offline") || stderr.contains("--offline"),
        "stderr should mention offline: {stderr}"
    );
    assert!(
        stderr.contains("github.com/org/lib") || stderr.contains("cache"),
        "stderr should name missing package or cache: {stderr}"
    );
    assert!(
        stderr.contains("draconic get") || stderr.contains("without --offline"),
        "stderr should include fixit: {stderr}"
    );
    assert!(!cache_mod.is_dir(), "offline must not fetch into cache");
    assert!(!out.is_file(), "offline miss must not write output");

    let _ = fs::remove_dir_all(&root);
}

/// ROADMAP K07.03: build prefers lock pins; does not float to newer tags when lock present.
#[test]
fn build_prefers_lock_pins_does_not_float() {
    let root = temp_dir();

    let upstream = root.join("upstream");
    fs::create_dir_all(&upstream).unwrap();
    git_ok(&["init"], &upstream);
    git_ok(&["config", "user.email", "test@draconic.local"], &upstream);
    git_ok(&["config", "user.name", "Draconic Test"], &upstream);
    git_ok(&["checkout", "-B", "main"], &upstream);
    fs::write(
        upstream.join("index.drac"),
        "export let value = 41;\nexport function inc(x) { return x + 1; }\n",
    )
    .unwrap();
    git_ok(&["add", "."], &upstream);
    git_ok(&["commit", "-m", "v1.0.0"], &upstream);
    git_ok(&["tag", "v1.0.0"], &upstream);
    let oid_v1 = git_stdout(&["rev-parse", "HEAD"], &upstream);

    // Publish a newer tag; a floating resolver would pick this.
    fs::write(
        upstream.join("index.drac"),
        "export let value = 99;\nexport function inc(x) { return x + 1; }\n",
    )
    .unwrap();
    git_ok(&["add", "."], &upstream);
    git_ok(&["commit", "-m", "v2.0.0"], &upstream);
    git_ok(&["tag", "v2.0.0"], &upstream);
    let oid_v2 = git_stdout(&["rev-parse", "HEAD"], &upstream);
    assert_ne!(oid_v1, oid_v2);

    // Seed lock content_hash for the pinned v1 tree via a throwaway get.
    let seed_cache = root.join("seed-cache");
    let (code, _stdout, stderr) = run_code(
        draconic()
            .arg("get")
            .arg("github.com/org/lib@1.0.0")
            .arg("--url")
            .arg(upstream.to_str().unwrap())
            .arg("--dir")
            .arg({
                let ws = root.join("seed-ws");
                fs::create_dir_all(&ws).unwrap();
                fs::write(
                    ws.join("draconic.toml"),
                    "module = \"github.com/acme/seed\"\n",
                )
                .unwrap();
                ws
            })
            .arg("--cache-dir")
            .arg(&seed_cache),
    );
    assert_eq!(code, 0, "seed get failed: {stderr}");
    let lock_src = fs::read_to_string(root.join("seed-ws/draconic.lock")).unwrap();
    let content_hash = lock_src
        .lines()
        .find_map(|l| l.trim().strip_prefix("content_hash = \""))
        .and_then(|s| s.strip_suffix('"'))
        .expect("content_hash in seed lock")
        .to_string();

    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    // Manifest req would float to 2.0.0 if build re-resolved tags.
    fs::write(
        ws.join("draconic.toml"),
        format!(
            "module = \"github.com/acme/app\"\n\n[dependencies]\n\"github.com/org/lib\" = \">=1.0.0\"\n\n[urls]\n\"github.com/org/lib\" = \"{}\"\n",
            upstream.display()
        ),
    )
    .unwrap();
    let lock_text = format!(
        r#"version = 1

[[package]]
path = "github.com/org/lib"
version = "1.0.0"
git_url = "{}"
commit_oid = "{oid_v1}"
content_hash = "{content_hash}"
"#,
        upstream.display()
    );
    fs::write(ws.join("draconic.lock"), &lock_text).unwrap();
    let main = ws.join("main.drac");
    fs::write(
        &main,
        "import { value, inc } from \"github.com/org/lib\";\nlet a = value;\nlet b = inc(value);\n",
    )
    .unwrap();

    let out = ws.join("out.js");
    run_ok(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("js")
            .arg(&main)
            .arg("-o")
            .arg(&out),
    );

    let cache_v1 = ws
        .join(".draconic/mod-cache/mod/github.com/org/lib")
        .join(&oid_v1);
    let cache_v2 = ws
        .join(".draconic/mod-cache/mod/github.com/org/lib")
        .join(&oid_v2);
    assert!(cache_v1.is_dir(), "locked v1 checkout must be present");
    assert!(
        !cache_v2.is_dir(),
        "build must not materialize floated v2 OID"
    );

    let js = fs::read_to_string(&out).expect("js");
    assert!(
        js.contains("41"),
        "build must emit locked pin value 41, got:\n{js}"
    );
    assert!(
        !js.contains("99"),
        "build must not float to v2 value 99, got:\n{js}"
    );

    let lock_after = fs::read_to_string(ws.join("draconic.lock")).unwrap();
    assert_eq!(
        lock_after, lock_text,
        "build must not rewrite lock when pins are present"
    );
    assert!(
        lock_after.contains(&oid_v1) && lock_after.contains("1.0.0"),
        "lock pin preserved: {lock_after}"
    );
    assert!(
        !lock_after.contains(&oid_v2) && !lock_after.contains("2.0.0"),
        "lock must not float: {lock_after}"
    );

    let _ = fs::remove_dir_all(&root);
}

/// ROADMAP K07.02: `draconic build --offline` succeeds when locked checkout is already cached.
#[test]
fn build_offline_succeeds_when_cache_present() {
    let root = temp_dir();

    let upstream = root.join("upstream");
    fs::create_dir_all(&upstream).unwrap();
    git_ok(&["init"], &upstream);
    git_ok(&["config", "user.email", "test@draconic.local"], &upstream);
    git_ok(&["config", "user.name", "Draconic Test"], &upstream);
    git_ok(&["checkout", "-B", "main"], &upstream);
    fs::write(
        upstream.join("index.drac"),
        "export let value = 41;\nexport function inc(x) { return x + 1; }\n",
    )
    .unwrap();
    git_ok(&["add", "."], &upstream);
    git_ok(&["commit", "-m", "v1.0.0"], &upstream);
    git_ok(&["tag", "v1.0.0"], &upstream);

    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    fs::write(
        ws.join("draconic.toml"),
        format!(
            "module = \"github.com/acme/app\"\n\n[dependencies]\n\"github.com/org/lib\" = \"1.0.0\"\n\n[urls]\n\"github.com/org/lib\" = \"{}\"\n",
            upstream.display()
        ),
    )
    .unwrap();
    let (code, _stdout, stderr) = run_code(
        draconic()
            .arg("get")
            .arg("github.com/org/lib@1.0.0")
            .arg("--url")
            .arg(upstream.to_str().unwrap())
            .arg("--dir")
            .arg(&ws),
    );
    assert_eq!(code, 0, "get failed: {stderr}");

    let main = ws.join("main.drac");
    fs::write(
        &main,
        "import { value, inc } from \"github.com/org/lib\";\nlet a = value;\nlet b = inc(value);\n",
    )
    .unwrap();

    let out = ws.join("out.js");
    run_ok(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("js")
            .arg("--offline")
            .arg(&main)
            .arg("-o")
            .arg(&out),
    );

    assert!(out.is_file(), "offline build with warm cache must emit js");
    let js = fs::read_to_string(&out).expect("js");
    assert!(
        js.contains("41") || js.contains("value") || js.contains("inc"),
        "{js}"
    );

    let _ = fs::remove_dir_all(&root);
}

/// ROADMAP K09.02: E2E CLI build of consumer importing module path from temp git fixture.
/// get → lock+cache → build --target js → Node observes imported values.
#[test]
fn k09_02_build_consumer_importing_module_path_from_fixture() {
    let root = temp_dir();

    let upstream = root.join("lib-upstream");
    fs::create_dir_all(&upstream).unwrap();
    git_ok(&["init"], &upstream);
    git_ok(&["config", "user.email", "test@draconic.local"], &upstream);
    git_ok(&["config", "user.name", "Draconic Test"], &upstream);
    git_ok(&["checkout", "-B", "main"], &upstream);
    fs::write(
        upstream.join("index.drac"),
        "export let answer = 42;\nexport function add(a, b) { return a + b; }\n",
    )
    .unwrap();
    git_ok(&["add", "."], &upstream);
    git_ok(&["commit", "-m", "v1.0.0"], &upstream);
    git_ok(&["tag", "v1.0.0"], &upstream);

    let ws = root.join("consumer");
    fs::create_dir_all(&ws).unwrap();
    fs::write(
        ws.join("draconic.toml"),
        "module = \"github.com/fixture/consumer\"\n",
    )
    .unwrap();
    let (code, _stdout, stderr) = run_code(
        draconic()
            .arg("get")
            .arg("github.com/fixture/lib@1.0.0")
            .arg("--url")
            .arg(upstream.to_str().unwrap())
            .arg("--dir")
            .arg(&ws),
    );
    assert_eq!(code, 0, "get failed: {stderr}");
    assert!(ws.join("draconic.lock").is_file(), "get must write lock");

    let main = ws.join("main.drac");
    fs::write(
        &main,
        r#"import { answer, add } from "github.com/fixture/lib";
let sum = add(answer, 8);
let a = answer;
"#,
    )
    .unwrap();

    let out = ws.join("out.js");
    run_ok(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("js")
            .arg(&main)
            .arg("-o")
            .arg(&out),
    );
    assert!(out.is_file(), "build must emit js");
    let js = fs::read_to_string(&out).expect("js");

    let node = Command::new("node")
        .arg("-e")
        .arg(format!(
            "{js}\nif (a !== 42) {{ console.error('a', a); process.exit(1); }}\nif (sum !== 50) {{ console.error('sum', sum); process.exit(1); }}"
        ))
        .output()
        .expect("node");
    assert!(
        node.status.success(),
        "node failed: stdout={} stderr={}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );

    let _ = fs::remove_dir_all(&root);
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

fn git_stdout(args: &[&str], cwd: &Path) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn git");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn run_code(cmd: &mut Command) -> (i32, String, String) {
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn draconic");
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}
