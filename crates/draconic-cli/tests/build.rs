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
    let output = draconic()
        .arg("build")
        .arg(&src)
        .output()
        .expect("spawn");
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
