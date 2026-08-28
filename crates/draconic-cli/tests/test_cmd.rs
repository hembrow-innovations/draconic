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
targets: js
js.exit: 0
js.check: if (x !== 3) process.exit(1);
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

/// ROADMAP U11: `draconic test --coverage` reports JS line coverage.
#[test]
fn test_coverage_reports_line_hits() {
    let dir = temp_dir();
    write(
        &dir,
        "cov.drac",
        "let a = 1;\nlet b = 2;\nlet c = a + b;\n",
    );
    write(
        &dir,
        "cov.meta",
        "\
id: cov
targets: js
js.exit: 0
js.check: if (c !== 3) process.exit(1);
",
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg("--coverage").arg(&dir));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("coverage"),
        "expected coverage section:\n{stdout}"
    );
    assert!(
        stdout.contains("lines") && (stdout.contains('%') || stdout.contains("/")),
        "expected line counts:\n{stdout}"
    );
    assert!(
        stdout.contains("cov.drac") || stdout.contains("total:"),
        "expected file or total line:\n{stdout}"
    );
    // Fully executed straight-line program should hit at least one line.
    assert!(
        !stdout.contains("0/0 lines") || stdout.contains("total:"),
        "stdout={stdout}"
    );
    let total_ok = stdout.lines().any(|l| {
        l.starts_with("total:")
            && l.contains("lines")
            && !l.contains("0/0")
            && !l.contains("0/")
    }) || stdout.lines().any(|l| {
        l.contains("lines") && l.contains('%') && !l.contains("0%")
    });
    assert!(total_ok, "expected non-zero coverage hits:\n{stdout}");
}

#[test]
fn test_coverage_flag_order_flexible() {
    let dir = temp_dir();
    write(&dir, "x.drac", "let n = 1;\n");
    write(
        &dir,
        "x.meta",
        "\
id: x
targets: js
js.exit: 0
js.check: if (n !== 1) process.exit(1);
",
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&dir).arg("--coverage"));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(stdout.contains("coverage"), "stdout={stdout}");
}

#[test]
fn help_lists_test_coverage() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("--coverage") || stdout.contains("coverage"),
        "help should mention coverage:\n{stdout}"
    );
}

/// ROADMAP L05.01: `describe` / `it` suite that all pass → `draconic test` exit 0.
#[test]
fn test_runs_in_language_describe_it() {
    let dir = temp_dir();
    let src = write(
        &dir,
        "suite.drac",
        r#"
describe("math", () => {
  it("adds", () => {
    if (1 + 1 !== 2) throw 1;
  });
});
"#,
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&src));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("ok") || stdout.contains("passed"),
        "stdout={stdout}"
    );
}

/// ROADMAP L05.01: a throwing `it` fails `draconic test`.
#[test]
fn test_fails_in_language_it_throw() {
    let dir = temp_dir();
    let src = write(
        &dir,
        "suite.drac",
        r#"
describe("math", () => {
  it("adds", () => {
    throw 1;
  });
});
"#,
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&src));
    assert_ne!(code, 0, "expected failure\nstdout={stdout}\nstderr={stderr}");
}
