//! ROADMAP U01: `draconic test` runner integration.

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
        "draconic-cli-test-{}-{}-{}",
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

fn write(dir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, contents).unwrap();
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
fn help_lists_test_command() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("draconic test") || stdout.contains("test "),
        "help should list test:\n{stdout}"
    );
}

#[test]
fn test_missing_path_exits_usage() {
    let (code, _stdout, stderr) = run(draconic().arg("test"));
    assert_eq!(code, 2, "stderr={stderr}");
    assert!(
        stderr.contains("usage") || stderr.contains("test"),
        "stderr={stderr}"
    );
}

#[test]
fn test_runs_passing_fixture_dir() {
    let dir = temp_dir();
    write(
        &dir,
        "smoke.drac",
        "let x = 1 + 2;\n",
    );
    write(
        &dir,
        "smoke.meta",
        "\
id: smoke
targets: js,native
js.exit: 0
js.check: if (x !== 3) process.exit(1);
native.exit: 0
native.stdout: hello\\n
",
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&dir));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("ok") || stdout.contains("passed"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("smoke") || stdout.contains("js") || stdout.contains("native"),
        "stdout={stdout}"
    );
}

#[test]
fn test_fails_when_js_check_fails() {
    let dir = temp_dir();
    write(&dir, "bad.drac", "let x = 1;\n");
    write(
        &dir,
        "bad.meta",
        "\
id: bad
targets: js
js.exit: 0
js.check: if (x !== 99) process.exit(1);
",
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&dir));
    assert_ne!(code, 0, "expected failure\nstdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("FAIL")
            || stdout.contains("fail")
            || stderr.contains("FAIL")
            || stderr.contains("fail")
            || stdout.contains("bad"),
        "stdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn test_runs_single_fixture_file() {
    let dir = temp_dir();
    let src = write(&dir, "one.drac", "let n = 0;\n");
    write(
        &dir,
        "one.meta",
        "\
id: one
targets: js
js.exit: 0
js.check: if (n !== 0) process.exit(1);
",
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&src));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("ok") || stdout.contains("passed") || stdout.contains("one"),
        "stdout={stdout}"
    );
}

#[test]
fn test_missing_path_reports_error() {
    let missing = temp_dir().join("does-not-exist");
    let (code, _stdout, stderr) = run(draconic().arg("test").arg(&missing));
    assert_ne!(code, 0, "stderr={stderr}");
    assert!(
        stderr.contains("error") || stderr.contains("missing") || stderr.contains("not"),
        "stderr={stderr}"
    );
}
