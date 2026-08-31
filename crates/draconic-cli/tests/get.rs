//! ROADMAP K05.01: `draconic get <module_path>@<ver>` — fetch, update manifest+lock+cache.

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
        "draconic-cli-get-{}-{}-{}",
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
fn help_lists_get() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(stdout.contains("get"), "help should list get:\n{stdout}");
}

#[test]
fn get_requires_spec() {
    let (code, _stdout, stderr) = run(draconic().arg("get"));
    assert_ne!(code, 0);
    assert!(
        stderr.contains("usage") || stderr.contains("module_path"),
        "stderr={stderr}"
    );
}

#[test]
fn get_fetches_and_writes_manifest_lock() {
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

    let (code, stdout, stderr) = run(draconic()
        .arg("get")
        .arg("github.com/org/lib@^1.0.0")
        .arg("--url")
        .arg(upstream.to_str().unwrap())
        .arg("--dir")
        .arg(&ws)
        .arg("--cache-dir")
        .arg(&cache));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("github.com/org/lib") && stdout.contains("1.2.3"),
        "stdout={stdout}"
    );

    let mf = fs::read_to_string(ws.join("draconic.toml")).unwrap();
    assert!(mf.contains("github.com/org/lib"), "{mf}");
    assert!(mf.contains("^1.0.0"), "{mf}");
    assert!(mf.contains(upstream.to_str().unwrap()), "{mf}");

    let lock = fs::read_to_string(ws.join("draconic.lock")).unwrap();
    assert!(lock.contains("github.com/org/lib"), "{lock}");
    assert!(
        lock.contains("version = \"1.2.3\"") || lock.contains("1.2.3"),
        "{lock}"
    );
    assert!(lock.contains("commit_oid"), "{lock}");
    assert!(lock.contains("content_hash"), "{lock}");

    // Cache has a checkout under mod/
    let mod_root = cache.join("mod").join("github.com").join("org").join("lib");
    assert!(
        mod_root.is_dir(),
        "expected cache checkout under {}",
        mod_root.display()
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn get_missing_manifest_fails() {
    let root = temp_dir();
    let (code, _stdout, stderr) = run(draconic()
        .arg("get")
        .arg("github.com/org/lib@1.0.0")
        .arg("--dir")
        .arg(&root)
        .arg("--cache-dir")
        .arg(root.join("cache")));
    assert_ne!(code, 0);
    assert!(
        stderr.contains("missing") || stderr.contains("draconic.toml"),
        "stderr={stderr}"
    );
    let _ = fs::remove_dir_all(&root);
}
