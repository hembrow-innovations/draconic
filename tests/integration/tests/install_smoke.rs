//! ROADMAP D01.03: after install, a fresh PATH can run `draconic -V` and parse hello.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-d0103-{}-{}-{}",
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

fn host_triple() -> String {
    let output = Command::new("rustc")
        .arg("-vV")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("rustc -vV");
    assert!(
        output.status.success(),
        "rustc -vV failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("host: ") {
            let t = rest.trim();
            assert!(!t.is_empty(), "empty host triple in rustc -vV:\n{stdout}");
            return t.to_string();
        }
    }
    panic!("no host: line in rustc -vV:\n{stdout}");
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

fn artifact_name(triple: &str) -> String {
    if cfg!(windows) {
        format!("draconic-{triple}.exe")
    } else {
        format!("draconic-{triple}")
    }
}

fn installed_name() -> &'static str {
    if cfg!(windows) {
        "draconic.exe"
    } else {
        "draconic"
    }
}

fn run_install(args: &[&str]) -> (i32, String, String) {
    let script = repo_root().join("scripts/install.sh");
    assert!(
        script.is_file(),
        "missing {} (D01.02 install script)",
        script.display()
    );
    let output = Command::new("bash")
        .arg(&script)
        .args(args)
        .current_dir(repo_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn install.sh");
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn install_to_temp() -> PathBuf {
    let dist = temp_dir();
    let dest = temp_dir();
    let triple = host_triple();
    fs::copy(draconic_bin(), dist.join(artifact_name(&triple))).expect("stage artifact");
    let (code, stdout, stderr) = run_install(&[
        "--from",
        dist.to_str().unwrap(),
        "--dir",
        dest.to_str().unwrap(),
    ]);
    assert_eq!(
        code, 0,
        "install.sh failed\nstdout={stdout}\nstderr={stderr}"
    );
    let placed = dest.join(installed_name());
    assert!(
        placed.is_file(),
        "expected installed binary {}\nstdout={stdout}\nstderr={stderr}",
        placed.display()
    );
    dest
}

/// `env -i PATH=<install-dir>` so lookup cannot see cargo `target/` or the ambient PATH.
fn fresh_path_draconic(install_dir: &str, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new("env");
    cmd.arg("-i")
        .arg(format!("PATH={install_dir}"))
        .arg(installed_name())
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = cmd.output().expect("spawn env -i draconic");
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

#[test]
fn readme_documents_install_smoke() {
    let readme = repo_root().join("README.md");
    let text = fs::read_to_string(&readme).expect("read README");
    let install_idx = text.find("## Install").expect("README should have Install");
    let after_install = &text[install_idx..];
    let next_heading = after_install[2..]
        .find("\n## ")
        .map(|i| i + 2)
        .unwrap_or(after_install.len());
    let install = &after_install[..next_heading];
    assert!(
        install.contains("draconic -V") || install.contains("draconic --version"),
        "Install section should smoke `draconic -V` on a fresh PATH:\n{install}"
    );
    assert!(
        install.contains("draconic parse"),
        "Install section should smoke `draconic parse` hello:\n{install}"
    );
}

#[test]
fn fresh_path_draconic_dash_v() {
    let dest = install_to_temp();
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
}

#[test]
fn fresh_path_draconic_parse_hello() {
    let dest = install_to_temp();
    let dest_s = dest.to_str().unwrap();
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
}
