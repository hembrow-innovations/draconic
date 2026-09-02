//! ROADMAP D05.01: `draconic build --strip` is invokable on the native target.

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
        "draconic-cli-strip-{}-{}-{}",
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

fn run(cmd: &mut Command) -> (i32, String, String) {
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

#[test]
fn build_native_strip_flag_is_invokable() {
    let dir = temp_dir();
    let src = write_program(&dir, "prog.drac", "let x: i32 = 42;");
    let out = dir.join("prog");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("native")
        .arg("--strip")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(
        code, 0,
        "--strip must be accepted on native build\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        out.is_file(),
        "stripped native binary missing at {}",
        out.display()
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn build_js_strip_flag_is_native_only() {
    let dir = temp_dir();
    let src = write_program(&dir, "prog.drac", "let x = 1;");
    let out = dir.join("out.js");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg("--strip")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_ne!(code, 0, "js --strip must fail\nstdout={stdout}\nstderr={stderr}");
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("strip") && combined.contains("native"),
        "js --strip must say the flag is native-only:\n{combined}"
    );
    assert!(
        !combined.contains("unknown option"),
        "js --strip must be recognized, not unknown:\n{combined}"
    );

    let _ = fs::remove_dir_all(&dir);
}
