//! ROADMAP U14: build+execute pipeline used by `draconic run` (library path).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_js::emit_js;
use draconic_backend_llvm::{build_native_binary, emit_llvm_ir};
use draconic_frontend::compile_source;

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-run-{}-{}-{}",
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

/// JS path: compile → emit → node execute (same as CLI `run --target js`).
#[test]
fn e2e_run_js_console_log() {
    let dir = temp_dir();
    let out = dir.join("out.js");
    let module =
        compile_source("let console = globalThis.console;\nconsole.log(\"integration-run-js\");\n")
            .expect("compile");
    let js = emit_js(&module).expect("emit_js");
    fs::write(&out, &js).unwrap();

    let output = Command::new("node")
        .arg(&out)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("node");
    assert!(
        output.status.success(),
        "node failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("integration-run-js"), "stdout={stdout}");
}

/// Native path: compile → LLVM → binary execute (same as CLI `run --target native`).
#[test]
fn e2e_run_native_scalar() {
    let dir = temp_dir();
    let out = dir.join("prog");
    let module = compile_source("let x: i32 = 11;").expect("compile");
    let ll = emit_llvm_ir(&module).expect("emit_llvm_ir");
    build_native_binary(&ll, Path::new(&out)).expect("build_native_binary");

    let output = Command::new(&out).output().expect("run");
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "11\n");
}

/// Hashbang line is ignored by the lexer/parser (shebang-friendly sources).
#[test]
fn e2e_run_js_source_with_hashbang() {
    let dir = temp_dir();
    let out = dir.join("out.js");
    let module = compile_source(
        "#!/usr/bin/env draconic\nlet console = globalThis.console;\nconsole.log(\"hashbang-src\");\n",
    )
    .expect("compile with hashbang");
    let js = emit_js(&module).expect("emit_js");
    fs::write(&out, &js).unwrap();

    let output = Command::new("node")
        .arg(&out)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("node");
    assert!(
        output.status.success(),
        "node failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("hashbang-src"));
}
