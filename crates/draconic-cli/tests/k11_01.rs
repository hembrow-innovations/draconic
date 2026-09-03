//! ROADMAP K11.01: private git auth CLI surface (HTTPS token / SSH).
//! Credentials come from the environment; they must not land in manifest or lock.

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
        "draconic-cli-k11-01-{}-{}-{}",
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

fn git_ok(args: &[&str], cwd: &Path) {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "Draconic Test")
        .env("GIT_AUTHOR_EMAIL", "test@draconic.local")
        .env("GIT_COMMITTER_NAME", "Draconic Test")
        .env("GIT_COMMITTER_EMAIL", "test@draconic.local")
        .output()
        .expect("spawn git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn tagged_upstream(root: &Path) -> PathBuf {
    let repo = root.join("upstream");
    fs::create_dir_all(&repo).unwrap();
    git_ok(&["init"], &repo);
    git_ok(&["config", "user.email", "test@draconic.local"], &repo);
    git_ok(&["config", "user.name", "Draconic Test"], &repo);
    git_ok(&["checkout", "-B", "main"], &repo);
    fs::write(repo.join("lib.drac"), "export let x = 42;\n").unwrap();
    git_ok(&["add", "."], &repo);
    git_ok(&["commit", "-m", "v1.2.3"], &repo);
    git_ok(&["tag", "v1.2.3"], &repo);
    repo
}

#[test]
fn k11_01_get_with_token_does_not_write_secret_to_manifest_or_lock() {
    let root = temp_dir();
    let upstream = tagged_upstream(&root);
    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    fs::write(
        ws.join("draconic.toml"),
        "module = \"github.com/acme/app\"\n",
    )
    .unwrap();
    let cache = root.join("cache");
    let token = "s3cret-token-k11-01-cli";

    let (code, stdout, stderr) = run(draconic()
        .env("DRACONIC_GIT_TOKEN", token)
        .arg("get")
        .arg("github.com/org/lib@1.2.3")
        .arg("--url")
        .arg(upstream.to_str().unwrap())
        .arg("--dir")
        .arg(&ws)
        .arg("--cache-dir")
        .arg(&cache));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(!stdout.contains(token), "stdout leaked token:\n{stdout}");
    assert!(!stderr.contains(token), "stderr leaked token:\n{stderr}");

    let mf = fs::read_to_string(ws.join("draconic.toml")).unwrap();
    let lock = fs::read_to_string(ws.join("draconic.lock")).unwrap();
    assert!(!mf.contains(token), "manifest leaked token:\n{mf}");
    assert!(!lock.contains(token), "lock leaked token:\n{lock}");
    assert!(mf.contains("github.com/org/lib"), "{mf}");
    assert!(lock.contains("github.com/org/lib"), "{lock}");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn k11_01_get_missing_ssh_identity_fails_closed() {
    let root = temp_dir();
    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    fs::write(
        ws.join("draconic.toml"),
        "module = \"github.com/acme/app\"\n",
    )
    .unwrap();
    let cache = root.join("cache");

    let (code, stdout, stderr) = run(draconic()
        .env("DRACONIC_GIT_SSH_KEY", "/no/such/k11-01-cli-ssh-key")
        .arg("get")
        .arg("github.com/org/lib@1.0.0")
        .arg("--url")
        .arg("git@github.com:org/lib.git")
        .arg("--dir")
        .arg(&ws)
        .arg("--cache-dir")
        .arg(&cache));
    assert_ne!(code, 0, "stdout={stdout}\nstderr={stderr}");
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("missing") || combined.contains("private git auth"),
        "stderr={stderr}\nstdout={stdout}"
    );
    assert!(
        combined.contains("SSH") || combined.contains("ssh") || combined.contains("identity"),
        "stderr={stderr}"
    );
    assert!(!ws.join("draconic.lock").exists(), "must not write lock");
    let mf = fs::read_to_string(ws.join("draconic.toml")).unwrap();
    assert!(
        !mf.contains("github.com/org/lib"),
        "must not write dep on auth failure:\n{mf}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn k11_01_get_rejected_ssh_credentials_fail_closed() {
    let root = temp_dir();
    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    fs::write(
        ws.join("draconic.toml"),
        "module = \"github.com/acme/app\"\n",
    )
    .unwrap();
    let dummy_key = root.join("dummy-key");
    fs::write(&dummy_key, "not-a-real-ssh-key\n").unwrap();
    let cache = root.join("cache");

    let (code, stdout, stderr) = run(draconic()
        .env("DRACONIC_GIT_SSH_KEY", dummy_key.to_str().unwrap())
        .arg("get")
        .arg("github.com/org/lib@1.0.0")
        .arg("--url")
        .arg("ssh://git@127.0.0.1:1/org/lib.git")
        .arg("--dir")
        .arg(&ws)
        .arg("--cache-dir")
        .arg(&cache));
    assert_ne!(code, 0, "stdout={stdout}\nstderr={stderr}");
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("auth")
            || combined.contains("git")
            || combined.contains("rejected")
            || combined.contains("Permission")
            || combined.contains("failed"),
        "expected fail-closed diagnostic, stderr={stderr}"
    );
    assert!(
        !combined.contains("not-a-real-ssh-key"),
        "stderr leaked key material:\n{stderr}"
    );
    assert!(!ws.join("draconic.lock").exists(), "must not write lock");

    let _ = fs::remove_dir_all(&root);
}
