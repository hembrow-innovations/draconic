//! ROADMAP U04: `draconic check` — typecheck + bind, no emit.

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
        "draconic-cli-check-{}-{}-{}",
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
fn help_lists_check_command() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("draconic check") || stdout.contains("check "),
        "help should list check:\n{stdout}"
    );
}

#[test]
fn check_missing_path_exits_usage() {
    let (code, _stdout, stderr) = run(draconic().arg("check"));
    assert_eq!(code, 2, "stderr={stderr}");
    assert!(
        stderr.contains("usage") || stderr.contains("check"),
        "stderr={stderr}"
    );
}

#[test]
fn check_ok_source_exits_zero_no_emit() {
    let dir = temp_dir();
    let src = write_program(&dir, "ok.drac", "let x = 1 + 2;\n");

    let (code, _stdout, stderr) = run(draconic().arg("check").arg(&src));
    assert_eq!(code, 0, "stderr={stderr}");

    // No emit: neither default JS nor native sibling artifacts.
    assert!(
        !dir.join("ok.js").exists(),
        "check must not write JS output"
    );
    assert!(
        !dir.join("ok").exists(),
        "check must not write native binary"
    );
    // Directory should only contain the source we wrote.
    let entries: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "check must not create extra files; entries={entries:?}"
    );
}

#[test]
fn check_parse_error_exits_nonzero() {
    let dir = temp_dir();
    let src = write_program(&dir, "bad.drac", "let = ;\n");

    let (code, _stdout, stderr) = run(draconic().arg("check").arg(&src));
    assert_ne!(code, 0, "parse error must fail");
    assert_ne!(code, 2, "parse error is not usage (got exit {code})");
    assert!(
        stderr.contains("error") || !stderr.is_empty(),
        "stderr should report error: {stderr}"
    );
}

#[test]
fn check_type_error_exits_nonzero() {
    let dir = temp_dir();
    // Typed binding with incompatible initializer (native-world type error).
    let src = write_program(&dir, "type_err.drac", "let x: i32 = \"hello\";\n");

    let (code, _stdout, stderr) = run(draconic().arg("check").arg(&src));
    assert_ne!(code, 0, "type error must fail; stderr={stderr}");
    assert_ne!(code, 2, "type error is not usage (got exit {code})");
    assert!(
        stderr.contains("error") || !stderr.is_empty(),
        "stderr should report error: {stderr}"
    );
}

#[test]
fn check_bind_error_exits_nonzero() {
    let dir = temp_dir();
    // Duplicate lexical binding (free identifiers are global-object refs, not bind errors).
    let src = write_program(&dir, "bind_err.drac", "let x = 1;\nlet x = 2;\n");

    let (code, _stdout, stderr) = run(draconic().arg("check").arg(&src));
    assert_ne!(code, 0, "bind error must fail; stderr={stderr}");
    assert_ne!(code, 2, "bind error is not usage (got exit {code})");
    assert!(
        stderr.contains("error") || !stderr.is_empty(),
        "stderr should report error: {stderr}"
    );
}
