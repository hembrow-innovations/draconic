//! ROADMAP K05.02: `draconic mod tidy` — lock matches manifest; fetch missing; prune unused.

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
        "draconic-cli-mod-tidy-{}-{}-{}",
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
fn help_lists_mod_tidy() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("mod tidy"),
        "help should list mod tidy:\n{stdout}"
    );
}

#[test]
fn mod_requires_tidy() {
    let (code, _stdout, stderr) = run(draconic().arg("mod"));
    assert_ne!(code, 0);
    assert!(
        stderr.contains("usage") || stderr.contains("tidy"),
        "stderr={stderr}"
    );
}

#[test]
fn mod_tidy_writes_lock_from_manifest() {
    let root = temp_dir();
    let upstream = tagged_upstream(&root);
    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    let path = "github.com/org/lib";
    fs::write(
        ws.join("draconic.toml"),
        format!(
            r#"module = "github.com/acme/app"

[dependencies]
"{path}" = "^1.0.0"

[urls]
"{path}" = "{url}"
"#,
            path = path,
            url = upstream.to_str().unwrap()
        ),
    )
    .unwrap();
    let cache = root.join("cache");

    let (code, stdout, stderr) = run(
        draconic()
            .arg("mod")
            .arg("tidy")
            .arg("--dir")
            .arg(&ws)
            .arg("--cache-dir")
            .arg(&cache),
    );
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("fetched") || stdout.contains("mod tidy"),
        "stdout={stdout}"
    );

    let lock = fs::read_to_string(ws.join("draconic.lock")).unwrap();
    assert!(lock.contains(path), "{lock}");
    assert!(lock.contains("1.2.3"), "{lock}");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn mod_tidy_prunes_unused() {
    let root = temp_dir();
    let upstream = tagged_upstream(&root);
    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    let cache = root.join("cache");

    fs::write(ws.join("draconic.toml"), "module = \"github.com/acme/app\"\n").unwrap();
    let (code, stdout, stderr) = run(
        draconic()
            .arg("get")
            .arg("github.com/org/lib@1.2.3")
            .arg("--url")
            .arg(upstream.to_str().unwrap())
            .arg("--dir")
            .arg(&ws)
            .arg("--cache-dir")
            .arg(&cache),
    );
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");

    // Drop dep from manifest.
    fs::write(ws.join("draconic.toml"), "module = \"github.com/acme/app\"\n").unwrap();

    let (code, stdout, stderr) = run(
        draconic()
            .arg("mod")
            .arg("tidy")
            .arg("--dir")
            .arg(&ws)
            .arg("--cache-dir")
            .arg(&cache),
    );
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(stdout.contains("pruned 1") || stdout.contains("pruned"), "{stdout}");

    let lock = fs::read_to_string(ws.join("draconic.lock")).unwrap();
    assert!(
        !lock.contains("github.com/org/lib")
            || lock.contains("packages = []")
            || !lock.contains("commit_oid"),
        "lock should not pin removed dep:\n{lock}"
    );
    // Empty lock still has version header; packages table empty.
    assert!(
        !lock.contains("[packages.\"github.com/org/lib\"]")
            && !lock.contains("path = \"github.com/org/lib\""),
        "{lock}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn mod_tidy_accepts_optional_toolchain_pin() {
    let root = temp_dir();
    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    fs::write(
        ws.join("draconic.toml"),
        "module = \"github.com/acme/app\"\ntoolchain = \"0.1.0\"\n",
    )
    .unwrap();

    let (code, stdout, stderr) = run(
        draconic()
            .arg("mod")
            .arg("tidy")
            .arg("--dir")
            .arg(&ws)
            .arg("--cache-dir")
            .arg(root.join("cache")),
    );
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    let mf = fs::read_to_string(ws.join("draconic.toml")).unwrap();
    assert!(
        mf.contains("toolchain"),
        "tidy must preserve toolchain pin:\n{mf}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn mod_tidy_accepts_required_toolchain_pin() {
    let root = temp_dir();
    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    fs::write(
        ws.join("draconic.toml"),
        &format!(
            "module = \"github.com/acme/app\"\ntoolchain = {{ version = \"{}\", required = true }}\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();

    let (code, stdout, stderr) = run(
        draconic()
            .arg("mod")
            .arg("tidy")
            .arg("--dir")
            .arg(&ws)
            .arg("--cache-dir")
            .arg(root.join("cache")),
    );
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    let mf = fs::read_to_string(ws.join("draconic.toml")).unwrap();
    assert!(
        mf.contains("toolchain") && mf.contains(env!("CARGO_PKG_VERSION")),
        "tidy must preserve required toolchain pin:\n{mf}"
    );

    let _ = fs::remove_dir_all(&root);
}
