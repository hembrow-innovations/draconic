//! ROADMAP U13: verbose version — commit, host target, LLVM (`draconic -V`).

use std::process::{Command, Stdio};

fn draconic() -> Command {
    Command::new(env!("CARGO_BIN_EXE_draconic"))
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

fn assert_verbose_version(stdout: &str, label: &str) {
    let version = env!("CARGO_PKG_VERSION");
    assert!(
        stdout.contains(&format!("draconic {version}"))
            || stdout.lines().next().is_some_and(|l| l.contains(version)),
        "{label}: first line should include package version {version}:\n{stdout}"
    );
    assert!(
        stdout.to_ascii_lowercase().contains("commit"),
        "{label}: should report git commit (or unknown):\n{stdout}"
    );
    assert!(
        stdout.to_ascii_lowercase().contains("host"),
        "{label}: should report host target triple:\n{stdout}"
    );
    let host_line = stdout
        .lines()
        .find(|l| l.to_ascii_lowercase().contains("host"))
        .unwrap_or("");
    assert!(
        host_line.contains('-') || host_line.to_ascii_lowercase().contains("unknown"),
        "{label}: host line should look like a triple or unknown:\n{stdout}"
    );
    assert!(
        stdout.to_ascii_lowercase().contains("llvm"),
        "{label}: should report LLVM version (or unknown):\n{stdout}"
    );
}

#[test]
fn version_flag_short_is_verbose() {
    let (code, stdout, stderr) = run(draconic().arg("-V"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert_verbose_version(&stdout, "-V");
}

#[test]
fn version_flag_long_is_verbose() {
    let (code, stdout, stderr) = run(draconic().arg("--version"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert_verbose_version(&stdout, "--version");
}

#[test]
fn version_subcommand_is_verbose() {
    let (code, stdout, stderr) = run(draconic().arg("version"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert_verbose_version(&stdout, "version");
}

#[test]
fn help_mentions_version() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("version") || stdout.contains("-V"),
        "help should mention version:\n{stdout}"
    );
}
