//! ROADMAP U05: `draconic fmt` — idempotent format; optional `--check`.

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
        "draconic-cli-fmt-{}-{}-{}",
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
fn help_lists_fmt_command() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("draconic fmt") || stdout.contains("fmt "),
        "help should list fmt:\n{stdout}"
    );
}

#[test]
fn fmt_missing_path_exits_usage() {
    let (code, _stdout, stderr) = run(draconic().arg("fmt"));
    assert_eq!(code, 2, "stderr={stderr}");
    assert!(
        stderr.contains("usage") || stderr.contains("fmt"),
        "stderr={stderr}"
    );
}

#[test]
fn fmt_rewrites_messy_source_in_place() {
    let dir = temp_dir();
    let src = write_program(&dir, "messy.drac", "let   x=1+2;\n");

    let (code, _stdout, stderr) = run(draconic().arg("fmt").arg(&src));
    assert_eq!(code, 0, "stderr={stderr}");

    let after = fs::read_to_string(&src).unwrap();
    assert_eq!(after, "let x = 1 + 2;\n");
}

#[test]
fn fmt_is_idempotent() {
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "idemp.drac",
        "function add(a,b){return a+b;}\nlet x=add(1,2);\n",
    );

    let (code, _, stderr) = run(draconic().arg("fmt").arg(&src));
    assert_eq!(code, 0, "first fmt stderr={stderr}");
    let once = fs::read_to_string(&src).unwrap();

    let (code, _, stderr) = run(draconic().arg("fmt").arg(&src));
    assert_eq!(code, 0, "second fmt stderr={stderr}");
    let twice = fs::read_to_string(&src).unwrap();

    assert_eq!(once, twice, "fmt(fmt(s)) must equal fmt(s)");
    assert!(once.contains("function add"), "{once}");
    assert!(once.contains("let x"), "{once}");
}

#[test]
fn fmt_check_fails_when_unformatted() {
    let dir = temp_dir();
    let src = write_program(&dir, "check_bad.drac", "let   x=1;\n");
    let before = fs::read_to_string(&src).unwrap();

    let (code, _stdout, stderr) = run(draconic().arg("fmt").arg("--check").arg(&src));
    assert_ne!(code, 0, "unformatted --check must fail");
    assert_ne!(code, 2, "not a usage error");
    assert!(
        stderr.contains("reformat") || stderr.contains("check_bad"),
        "stderr={stderr}"
    );
    // --check must not rewrite
    assert_eq!(fs::read_to_string(&src).unwrap(), before);
}

#[test]
fn fmt_check_ok_when_already_formatted() {
    let dir = temp_dir();
    let src = write_program(&dir, "check_ok.drac", "let x = 1;\n");

    // Format once to canonical form, then --check.
    let (code, _, stderr) = run(draconic().arg("fmt").arg(&src));
    assert_eq!(code, 0, "stderr={stderr}");
    let formatted = fs::read_to_string(&src).unwrap();

    let (code, _, stderr) = run(draconic().arg("fmt").arg("--check").arg(&src));
    assert_eq!(code, 0, "already formatted; stderr={stderr}");
    assert_eq!(fs::read_to_string(&src).unwrap(), formatted);
}

#[test]
fn fmt_parse_error_exits_nonzero() {
    let dir = temp_dir();
    let src = write_program(&dir, "bad.drac", "let = ;\n");

    let (code, _stdout, stderr) = run(draconic().arg("fmt").arg(&src));
    assert_ne!(code, 0, "parse error must fail");
    assert_ne!(code, 2, "parse error is not usage");
    assert!(
        stderr.contains("error") || !stderr.is_empty(),
        "stderr={stderr}"
    );
}
