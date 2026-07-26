//! ROADMAP B10: end-to-end build pipeline for js + native (CLI driver path).

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
        "draconic-integration-build-{}-{}-{}",
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

fn compile_module(src: &str) -> draconic_ir::Module {
    compile_source(src).expect("compile")
}

/// Same pipeline the CLI `build --target js` uses: source → IR → JS artifact.
#[test]
fn e2e_js_build_artifact_runs() {
    let dir = temp_dir();
    let out = dir.join("out.js");
    let module = compile_module("let x = 1 + 2;");
    let js = emit_js(&module).expect("emit_js");
    fs::write(&out, &js).unwrap();

    let script = format!(
        "{}\nif (x !== 3) {{ console.error(x); process.exit(1); }}",
        fs::read_to_string(&out).unwrap()
    );
    let output = Command::new("node")
        .arg("-e")
        .arg(&script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("node");
    assert!(
        output.status.success(),
        "node failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Same pipeline the CLI `build --target native` uses: source → IR → LLVM → binary.
#[test]
fn e2e_native_build_artifact_runs() {
    let dir = temp_dir();
    let out = dir.join("prog");
    let module = compile_module("let x = 1;");
    let ll = emit_llvm_ir(&module).expect("emit_llvm_ir");
    build_native_binary(&ll, Path::new(&out)).expect("build_native_binary");

    let output = Command::new(&out).output().expect("run");
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n");
}
