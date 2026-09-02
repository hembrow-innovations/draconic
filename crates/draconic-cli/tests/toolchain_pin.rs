//! ROADMAP D02: toolchain version pin in `draconic.toml`; CLI enforces or warns.
//! Child D02.02 locks the mismatch path; parent D02 is the combined pin surface.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn draconic() -> Command {
    Command::new(env!("CARGO_BIN_EXE_draconic"))
}

fn running_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-cli-toolchain-pin-{}-{}-{}",
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

fn write_manifest(dir: &Path, body: &str) {
    fs::write(dir.join("draconic.toml"), body).unwrap();
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
fn required_pin_mismatch_check_exits_nonzero() {
    let dir = temp_dir();
    write_manifest(
        &dir,
        "module = \"github.com/acme/app\"\ntoolchain = { version = \"9.9.9\", required = true }\n",
    );
    let src = write_program(&dir, "ok.drac", "let x = 1 + 2;\n");

    let (code, stdout, stderr) = run(draconic().arg("check").arg(&src));
    assert_eq!(code, 1, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stderr.contains("9.9.9") && stderr.contains(running_version()),
        "stderr should name pin and running version:\n{stderr}"
    );
    assert!(
        stderr.to_ascii_lowercase().contains("toolchain"),
        "stderr={stderr}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn optional_pin_mismatch_check_warns_and_succeeds() {
    let dir = temp_dir();
    write_manifest(
        &dir,
        "module = \"github.com/acme/app\"\ntoolchain = \"9.9.9\"\n",
    );
    let src = write_program(&dir, "ok.drac", "let x = 1 + 2;\n");

    let (code, stdout, stderr) = run(draconic().arg("check").arg(&src));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stderr.to_ascii_lowercase().contains("warning"),
        "optional mismatch must warn:\n{stderr}"
    );
    assert!(
        stderr.contains("9.9.9") && stderr.contains(running_version()),
        "stderr={stderr}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn matching_pin_is_silent_success() {
    let dir = temp_dir();
    let ver = running_version();
    write_manifest(
        &dir,
        &format!("module = \"github.com/acme/app\"\ntoolchain = \"{ver}\"\n"),
    );
    let src = write_program(&dir, "ok.drac", "let x = 1 + 2;\n");

    let (code, stdout, stderr) = run(draconic().arg("check").arg(&src));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        !stderr.to_ascii_lowercase().contains("warning"),
        "matching pin must not warn:\n{stderr}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn no_manifest_is_unpinned_success() {
    let dir = temp_dir();
    let src = write_program(&dir, "ok.drac", "let x = 1 + 2;\n");

    let (code, stdout, stderr) = run(draconic().arg("check").arg(&src));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        !stderr.to_ascii_lowercase().contains("toolchain"),
        "unpinned must not mention toolchain:\n{stderr}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn version_flag_ignores_required_pin_mismatch() {
    let dir = temp_dir();
    write_manifest(
        &dir,
        "module = \"github.com/acme/app\"\ntoolchain = { version = \"9.9.9\", required = true }\n",
    );

    let (code, stdout, stderr) = run(draconic().current_dir(&dir).arg("-V"));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains(running_version()),
        "version must still print:\n{stdout}"
    );
    assert!(
        !stderr.to_ascii_lowercase().contains("toolchain"),
        "-V must not enforce pin:\n{stderr}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn required_pin_is_discovered_from_parent_dir() {
    let dir = temp_dir();
    write_manifest(
        &dir,
        "module = \"github.com/acme/app\"\ntoolchain = { version = \"9.9.9\", required = true }\n",
    );
    let nested = dir.join("src");
    fs::create_dir_all(&nested).unwrap();
    let src = write_program(&nested, "ok.drac", "let x = 1;\n");

    let (code, stdout, stderr) = run(draconic().arg("check").arg(&src));
    assert_eq!(code, 1, "stdout={stdout}\nstderr={stderr}");
    assert!(stderr.contains("9.9.9"), "stderr={stderr}");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn required_pin_mismatch_build_exits_nonzero() {
    let dir = temp_dir();
    write_manifest(
        &dir,
        "module = \"github.com/acme/app\"\ntoolchain = { version = \"9.9.9\", required = true }\n",
    );
    let src = write_program(&dir, "ok.drac", "let x = 1 + 2;\n");
    let out = dir.join("out.js");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(code, 1, "stdout={stdout}\nstderr={stderr}");
    assert!(stderr.contains("9.9.9"), "stderr={stderr}");
    assert!(!out.exists(), "mismatch must not emit");

    let _ = fs::remove_dir_all(&dir);
}
