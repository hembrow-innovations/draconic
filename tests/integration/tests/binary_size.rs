//! ROADMAP D05: strip / LTO flags documented and testable.
//!
//! Combined surface for the parent row: docs name both native size opts, and
//! `draconic build` accepts `--strip` and `--lto` together. Child D05.01 locks
//! strip-symbols size; child D05.02 locks the LTO size-delta smoke.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-binary-size-{}-{}-{}",
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

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn draconic_bin() -> PathBuf {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let bin = repo_root().join("target").join(profile).join("draconic");
    assert!(
        bin.is_file(),
        "missing {} (build draconic-cli first)",
        bin.display()
    );
    bin
}

fn read(path: &str) -> String {
    let full = repo_root().join(path);
    assert!(full.is_file(), "missing {} (D05)", full.display());
    fs::read_to_string(&full).unwrap_or_else(|e| panic!("read {}: {e}", full.display()))
}

fn run(cmd: &mut Command) -> (i32, String, String) {
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn");
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

/// Docs name both native size opts on the public CLI reference page.
#[test]
fn cli_docs_name_strip_and_lto_flags() {
    let text = read("website/cli.md");
    assert!(
        text.contains("--strip"),
        "CLI docs must name --strip:\n{text}"
    );
    assert!(text.contains("--lto"), "CLI docs must name --lto:\n{text}");
    let lower = text.to_ascii_lowercase();
    assert!(
        lower.contains("native") && (lower.contains("size") || lower.contains("strip")),
        "CLI docs must describe --strip / --lto as native size opts:\n{text}"
    );
}

/// `draconic help` documents both flags on one usage surface.
#[test]
fn help_documents_strip_and_lto_flags() {
    let (code, stdout, stderr) = run(Command::new(draconic_bin()).arg("help"));
    assert_eq!(code, 0, "help failed\nstdout={stdout}\nstderr={stderr}");
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("--strip"),
        "draconic help must document --strip:\n{combined}"
    );
    assert!(
        combined.contains("--lto"),
        "draconic help must document --lto:\n{combined}"
    );
}

/// Combined sitting: `--strip --lto` is invokable on native and writes a binary.
#[test]
fn strip_and_lto_together_are_invokable_on_native() {
    let dir = temp_dir();
    let src = dir.join("prog.drac");
    fs::write(&src, "let x: i32 = 42;\n").unwrap();
    let out = dir.join("prog");

    let (code, stdout, stderr) = run(Command::new(draconic_bin())
        .arg("build")
        .arg("--target")
        .arg("native")
        .arg("--strip")
        .arg("--lto")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(
        code, 0,
        "--strip --lto must be accepted on native build\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        out.is_file(),
        "native binary missing at {}",
        out.display()
    );

    let _ = fs::remove_dir_all(&dir);
}
