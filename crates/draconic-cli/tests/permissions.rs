//! ROADMAP R02.03: CLI/runtime flags grant a permission subset (opt-in).
//! `--allow-fs-read` / `--allow-fs-write` / `--allow-net-listen` /
//! `--allow-net-connect` install `DRACONIC_PERMISSIONS` on the child.
//! A granted subset is honoured; an ungranted host op fails closed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn draconic() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_draconic"));
    cmd.env_remove("DRACONIC_PERMISSIONS");
    cmd
}

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-cli-perm-{}-{}-{}",
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
fn help_lists_allow_grant_flags() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    for flag in [
        "--allow-fs-read",
        "--allow-fs-write",
        "--allow-net-listen",
        "--allow-net-connect",
    ] {
        assert!(stdout.contains(flag), "help should list {flag}:\n{stdout}");
    }
}

#[test]
fn run_allow_fs_subset_honoured_js() {
    let dir = temp_dir();
    let file = dir.join("out.txt");
    let src = write_program(
        &dir,
        "grant.drac",
        &format!(
            r#"
writeFileText({path:?}, "r0203-cli-fs");
let t = readFileText({path:?});
let console = globalThis.console;
console.log(t);
"#,
            path = file.display().to_string()
        ),
    );

    let (code, stdout, stderr) = run(draconic()
        .arg("run")
        .arg("--target")
        .arg("js")
        .arg("--allow-fs-read")
        .arg("--allow-fs-write")
        .arg(&src));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("r0203-cli-fs"),
        "stdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn run_allow_fs_write_without_read_fails_closed_js() {
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "deny.drac",
        r#"
let code = "";
let name = "";
try {
  readFileText("/tmp/draconic_r0203_cli_deny.txt");
} catch (e) {
  code = e.code;
  name = e.name;
}
let console = globalThis.console;
console.log(code);
console.log(name);
"#,
    );

    let (code, stdout, stderr) = run(draconic()
        .arg("run")
        .arg("--target")
        .arg("js")
        .arg("--allow-fs-write")
        .arg(&src));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(stdout.contains("EPERM"), "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("HostError"),
        "stdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn run_allow_fs_subset_honoured_native() {
    let dir = temp_dir();
    let file = dir.join("out.txt");
    let src = write_program(
        &dir,
        "grant.drac",
        &format!(
            r#"
writeFileText({path:?}, "r0203-cli-fs");
let t = readFileText({path:?});
"#,
            path = file.display().to_string()
        ),
    );

    let (code, stdout, stderr) = run(draconic()
        .arg("run")
        .arg("--target")
        .arg("native")
        .arg("--allow-fs-read")
        .arg("--allow-fs-write")
        .arg(&src));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert_eq!(
        stdout, "r0203-cli-fs\n",
        "stdout={stdout:?}\nstderr={stderr}"
    );
}

#[test]
fn run_allow_fs_write_without_read_fails_closed_native() {
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "deny.drac",
        r#"readFileText("/tmp/draconic_r0203_cli_deny.txt");
"#,
    );

    let (code, stdout, stderr) = run(draconic()
        .arg("run")
        .arg("--target")
        .arg("native")
        .arg("--allow-fs-write")
        .arg(&src));
    assert_eq!(code, 1, "stdout={stdout}\nstderr={stderr}");
    assert_eq!(stderr, "EPERM\n", "stdout={stdout:?}\nstderr={stderr:?}");
}
