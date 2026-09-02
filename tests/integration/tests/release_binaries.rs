//! ROADMAP D01: release binaries + install script; one-line install to PATH.
//!
//! Combined surface for the parent row: CI/release stages a host-triple
//! artifact, the install script places `draconic` on PATH, and a fresh PATH
//! can run `draconic -V` and parse a hello Program. Child rows D01.01–D01.03
//! lock each step; this file locks the pipeline as one sitting.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-d01-{}-{}-{}",
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

fn installed_name() -> &'static str {
    if cfg!(windows) {
        "draconic.exe"
    } else {
        "draconic"
    }
}

fn run_bash(script: &str, args: &[&str]) -> (i32, String, String) {
    let path = repo_root().join(script);
    assert!(
        path.is_file(),
        "missing {} (D01 parent surface)",
        path.display()
    );
    let output = Command::new("bash")
        .arg(&path)
        .args(args)
        .current_dir(repo_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("spawn {script}: {e}"));
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

/// `env -i PATH=<install-dir>` so lookup cannot see cargo `target/` or the ambient PATH.
fn fresh_path_draconic(install_dir: &str, args: &[&str]) -> (i32, String, String) {
    let output = Command::new("env")
        .arg("-i")
        .arg(format!("PATH={install_dir}"))
        .arg(installed_name())
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn env -i draconic");
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

/// Host-triple artifact → install dir on PATH → `draconic -V` and parse hello.
#[test]
fn release_then_install_then_fresh_path_dash_v_and_parse_hello() {
    let dist = temp_dir();
    let dest = temp_dir();
    let bin = draconic_bin();

    let (code, stdout, stderr) = run_bash(
        "scripts/release-artifact.sh",
        &[
            "--bin",
            bin.to_str().unwrap(),
            "--out",
            dist.to_str().unwrap(),
        ],
    );
    assert_eq!(
        code, 0,
        "release-artifact.sh failed\nstdout={stdout}\nstderr={stderr}"
    );

    let (code, stdout, stderr) = run_bash(
        "scripts/install.sh",
        &[
            "--from",
            dist.to_str().unwrap(),
            "--dir",
            dest.to_str().unwrap(),
        ],
    );
    assert_eq!(
        code, 0,
        "install.sh failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        dest.join(installed_name()).is_file(),
        "install should place {} on PATH dir\nstdout={stdout}\nstderr={stderr}",
        dest.join(installed_name()).display()
    );

    let dest_s = dest.to_str().unwrap();
    let (code, stdout, stderr) = fresh_path_draconic(dest_s, &["-V"]);
    assert_eq!(
        code, 0,
        "fresh PATH draconic -V failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("draconic"),
        "fresh PATH -V should print version:\n{stdout}"
    );

    let hello = dest.join("hello.drac");
    fs::write(
        &hello,
        "let console = globalThis.console;\nconsole.log(\"hello\");\n",
    )
    .expect("write hello.drac");
    let (code, stdout, stderr) = fresh_path_draconic(dest_s, &["parse", hello.to_str().unwrap()]);
    assert_eq!(
        code, 0,
        "fresh PATH draconic parse hello failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("Program"),
        "parse hello should dump AST starting with Program:\n{stdout}"
    );
    assert!(
        stdout.contains("hello"),
        "parse hello dump should include the string literal:\n{stdout}"
    );

    let _ = fs::remove_dir_all(&dist);
    let _ = fs::remove_dir_all(&dest);
}
