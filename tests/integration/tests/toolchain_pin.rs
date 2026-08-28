//! ROADMAP D02.02: CLI enforces or warns when running toolchain ≠ pin.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-toolchain-pin-{}-{}-{}",
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

fn running_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
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
fn required_pin_mismatch_check_fails() {
    let dir = temp_dir();
    fs::write(
        dir.join("draconic.toml"),
        "module = \"github.com/acme/app\"\ntoolchain = { version = \"9.9.9\", required = true }\n",
    )
    .unwrap();
    let src = write_program(&dir, "ok.drac", "let x = 1 + 2;\n");

    let (code, stdout, stderr) = run(Command::new(draconic_bin()).arg("check").arg(&src));
    assert_eq!(code, 1, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stderr.contains("9.9.9") && stderr.contains(running_version()),
        "stderr={stderr}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn optional_pin_mismatch_check_warns() {
    let dir = temp_dir();
    fs::write(
        dir.join("draconic.toml"),
        "module = \"github.com/acme/app\"\ntoolchain = \"9.9.9\"\n",
    )
    .unwrap();
    let src = write_program(&dir, "ok.drac", "let x = 1 + 2;\n");

    let (code, stdout, stderr) = run(Command::new(draconic_bin()).arg("check").arg(&src));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stderr.to_ascii_lowercase().contains("warning"),
        "stderr={stderr}"
    );
    assert!(stderr.contains("9.9.9"), "stderr={stderr}");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn matching_required_pin_check_succeeds() {
    let dir = temp_dir();
    let ver = running_version();
    fs::write(
        dir.join("draconic.toml"),
        format!(
            "module = \"github.com/acme/app\"\ntoolchain = {{ version = \"{ver}\", required = true }}\n"
        ),
    )
    .unwrap();
    let src = write_program(&dir, "ok.drac", "let x = 1 + 2;\n");

    let (code, stdout, stderr) = run(Command::new(draconic_bin()).arg("check").arg(&src));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        !stderr.to_ascii_lowercase().contains("warning"),
        "stderr={stderr}"
    );

    let _ = fs::remove_dir_all(&dir);
}
