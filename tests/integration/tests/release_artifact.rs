//! ROADMAP D01.01: CI/release produces a platform binary artifact for the host triple.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-d0101-{}-{}-{}",
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

fn run_script(args: &[&str]) -> (i32, String, String) {
    let script = repo_root().join("scripts/release-artifact.sh");
    assert!(
        script.is_file(),
        "missing {} (D01.01 release artifact script)",
        script.display()
    );
    let output = Command::new("bash")
        .arg(&script)
        .args(args)
        .current_dir(repo_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn release-artifact.sh");
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

#[test]
fn release_script_stages_host_triple_binary() {
    let out = temp_dir();
    let bin = draconic_bin();
    let triple = host_triple();
    let name = artifact_name(&triple);
    let staged = out.join(&name);

    let (code, stdout, stderr) = run_script(&[
        "--bin",
        bin.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(
        code, 0,
        "release-artifact.sh failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        staged.is_file(),
        "expected staged artifact {}\nstdout={stdout}\nstderr={stderr}",
        staged.display()
    );
    assert!(
        stdout.contains(&name) || Path::new(stdout.trim()).file_name() == Some(name.as_ref()),
        "script should print staged artifact path containing {name}\nstdout={stdout}"
    );

    let output = Command::new(&staged)
        .arg("-V")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run staged artifact -V");
    assert!(
        output.status.success(),
        "staged artifact -V failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let ver = String::from_utf8_lossy(&output.stdout);
    assert!(
        ver.contains("draconic"),
        "staged -V should print version:\n{ver}"
    );
}

#[test]
fn ci_workflow_builds_and_uploads_host_artifact() {
    let workflow = repo_root().join(".github/workflows/release-artifact.yml");
    assert!(
        workflow.is_file(),
        "missing {} (D01.01 CI/release workflow)",
        workflow.display()
    );
    let text = fs::read_to_string(&workflow).expect("read workflow");
    assert!(
        text.contains("release-artifact.sh") || text.contains("scripts/release-artifact"),
        "workflow should invoke the release artifact script:\n{text}"
    );
    assert!(
        text.contains("upload-artifact"),
        "workflow should upload the host-triple binary as a GitHub Actions artifact:\n{text}"
    );
    assert!(
        text.contains("draconic-cli") || text.contains("cargo build"),
        "workflow should build the draconic CLI:\n{text}"
    );
}
