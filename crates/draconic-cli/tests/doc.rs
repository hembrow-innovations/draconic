//! ROADMAP U12: `draconic doc` — extract doc comments → markdown/HTML.

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
        "draconic-cli-doc-{}-{}-{}",
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
fn help_lists_doc_command() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("draconic doc") || stdout.contains("doc "),
        "help should list doc:\n{stdout}"
    );
}

#[test]
fn doc_missing_path_exits_usage() {
    let (code, _stdout, stderr) = run(draconic().arg("doc"));
    assert_eq!(code, 2, "stderr={stderr}");
    assert!(
        stderr.contains("usage") || stderr.contains("doc"),
        "stderr={stderr}"
    );
}

#[test]
fn doc_emits_markdown_for_jsdoc_function() {
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "lib.drac",
        r#"/**
 * Add two numbers.
 */
function add(a, b) {
  return a + b;
}

/** No star prefix on this line */
function greet(name) {
  return name;
}
"#,
    );

    let (code, stdout, stderr) = run(draconic().arg("doc").arg(&src));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(stdout.contains("# lib.drac") || stdout.contains("# lib"), "{stdout}");
    assert!(stdout.contains("## `add`") || stdout.contains("## add"), "{stdout}");
    assert!(stdout.contains("Add two numbers."), "{stdout}");
    assert!(stdout.contains("## `greet`") || stdout.contains("## greet"), "{stdout}");
    assert!(stdout.contains("No star prefix on this line"), "{stdout}");
}

#[test]
fn doc_associates_with_class_and_const() {
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "types.drac",
        r#"/**
 * A counter.
 */
class Counter {
  constructor() {}
}

/**
 * Default limit.
 */
const LIMIT = 10;
"#,
    );

    let (code, stdout, stderr) = run(draconic().arg("doc").arg(&src));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(stdout.contains("Counter"), "{stdout}");
    assert!(stdout.contains("A counter."), "{stdout}");
    assert!(stdout.contains("LIMIT"), "{stdout}");
    assert!(stdout.contains("Default limit."), "{stdout}");
}

#[test]
fn doc_format_html_writes_file() {
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "html.drac",
        r#"/**
 * Escape me: <script> & "quotes"
 */
function danger() {}
"#,
    );
    let out = dir.join("out.html");

    let (code, _stdout, stderr) = run(
        draconic()
            .arg("doc")
            .arg("--format")
            .arg("html")
            .arg("-o")
            .arg(&out)
            .arg(&src),
    );
    assert_eq!(code, 0, "stderr={stderr}");
    let html = fs::read_to_string(&out).unwrap();
    assert!(html.contains("<!DOCTYPE html>") || html.contains("<html"), "{html}");
    assert!(html.contains("danger"), "{html}");
    assert!(
        html.contains("&lt;script&gt;") || html.contains("&#"),
        "must escape HTML specials:\n{html}"
    );
    assert!(!html.contains("<script>"), "raw script tag must not appear:\n{html}");
}

#[test]
fn doc_export_function() {
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "mod.drac",
        r#"/**
 * Public API.
 */
export function api() {
  return 1;
}
"#,
    );

    let (code, stdout, stderr) = run(draconic().arg("doc").arg(&src));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(stdout.contains("api"), "{stdout}");
    assert!(stdout.contains("Public API."), "{stdout}");
}

#[test]
fn doc_missing_file_exits_nonzero() {
    let dir = temp_dir();
    let missing = dir.join("nope.drac");
    let (code, _stdout, stderr) = run(draconic().arg("doc").arg(&missing));
    assert_ne!(code, 0, "missing file must fail");
    assert_ne!(code, 2, "not a usage error");
    assert!(!stderr.is_empty(), "stderr={stderr}");
}

#[test]
fn doc_no_docs_emits_title_only() {
    let dir = temp_dir();
    let src = write_program(&dir, "empty.drac", "let x = 1;\n");
    let (code, stdout, stderr) = run(draconic().arg("doc").arg(&src));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("# empty.drac") || stdout.contains("# empty"),
        "{stdout}"
    );
}
