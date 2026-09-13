//! K11 CLI wiring: lock what `get`, `mod tidy`, and ensure actually honor.
//! Proxy, tidy-yank, and local monorepo subdir are not silent v1.

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
        "draconic-cli-k11-wiring-{}-{}-{}",
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

fn write_app(ws: &Path) {
    fs::create_dir_all(ws).unwrap();
    fs::write(ws.join("draconic.toml"), "module = \"github.com/acme/app\"\n").unwrap();
}

fn assert_not_proxy_off(stdout: &str, stderr: &str) {
    let combined = format!("{stdout}{stderr}");
    assert!(
        !combined.contains("module proxy is off") && !combined.contains("proxy is off"),
        "CLI used DRACONIC_PROXY:\n{combined}"
    );
}

#[test]
fn k11_04_get_does_not_use_proxy_list() {
    let root = temp_dir();
    let upstream = tagged_upstream(&root);
    let ws = root.join("app");
    write_app(&ws);
    let cache = root.join("cache");

    let (code, stdout, stderr) = run(draconic()
        .env("DRACONIC_PROXY", "off")
        .arg("get")
        .arg("github.com/org/lib@1.2.3")
        .arg("--url")
        .arg(upstream.to_str().unwrap())
        .arg("--dir")
        .arg(&ws)
        .arg("--cache-dir")
        .arg(&cache));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert_not_proxy_off(&stdout, &stderr);
    let lock = fs::read_to_string(ws.join("draconic.lock")).unwrap();
    assert!(lock.contains("github.com/org/lib"), "{lock}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn k11_04_tidy_does_not_use_proxy_list() {
    let root = temp_dir();
    let upstream = tagged_upstream(&root);
    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    fs::write(
        ws.join("draconic.toml"),
        format!(
            r#"module = "github.com/acme/app"

[dependencies]
"github.com/org/lib" = "1.2.3"

[urls]
"github.com/org/lib" = "{url}"
"#,
            url = upstream.display()
        ),
    )
    .unwrap();
    let cache = root.join("cache");

    let (code, stdout, stderr) = run(draconic()
        .env("DRACONIC_PROXY", "off")
        .arg("mod")
        .arg("tidy")
        .arg("--dir")
        .arg(&ws)
        .arg("--cache-dir")
        .arg(&cache));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert_not_proxy_off(&stdout, &stderr);
    let lock = fs::read_to_string(ws.join("draconic.lock")).unwrap();
    assert!(lock.contains("1.2.3"), "{lock}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn k11_04_ensure_does_not_use_proxy_list() {
    let root = temp_dir();
    let upstream = tagged_upstream(&root);
    let seed = root.join("seed");
    write_app(&seed);
    let seed_cache = root.join("seed-cache");
    let (code, stdout, stderr) = run(draconic()
        .arg("get")
        .arg("github.com/org/lib@1.2.3")
        .arg("--url")
        .arg(upstream.to_str().unwrap())
        .arg("--dir")
        .arg(&seed)
        .arg("--cache-dir")
        .arg(&seed_cache));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    let lock = fs::read_to_string(seed.join("draconic.lock")).unwrap();

    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    fs::write(
        ws.join("draconic.toml"),
        format!(
            r#"module = "github.com/acme/app"

[dependencies]
"github.com/org/lib" = "1.2.3"

[urls]
"github.com/org/lib" = "{url}"
"#,
            url = upstream.display()
        ),
    )
    .unwrap();
    fs::write(ws.join("draconic.lock"), &lock).unwrap();
    fs::write(ws.join("main.drac"), "let n = 1;\n").unwrap();
    let empty_cache = root.join("empty-cache");
    fs::create_dir_all(&empty_cache).unwrap();
    let out = ws.join("out.js");

    let (code, stdout, stderr) = run(draconic()
        .env("DRACONIC_PROXY", "off")
        .env("DRACONIC_MOD_CACHE", empty_cache.to_str().unwrap())
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg(ws.join("main.drac"))
        .arg("-o")
        .arg(&out));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert_not_proxy_off(&stdout, &stderr);
    assert!(out.is_file(), "ensure must still clone without proxy");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn k11_05_tidy_does_not_honor_advisory() {
    let root = temp_dir();
    let upstream = tagged_upstream(&root);
    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    fs::write(
        ws.join("draconic.toml"),
        format!(
            r#"module = "github.com/acme/app"

[dependencies]
"github.com/org/lib" = "1.2.3"

[urls]
"github.com/org/lib" = "{url}"
"#,
            url = upstream.display()
        ),
    )
    .unwrap();
    let advisory = root.join("advisory.txt");
    fs::write(&advisory, "yank github.com/org/lib 1.2.3\n").unwrap();
    let cache = root.join("cache");

    let (code, stdout, stderr) = run(draconic()
        .env("DRACONIC_ADVISORY", advisory.to_str().unwrap())
        .arg("mod")
        .arg("tidy")
        .arg("--dir")
        .arg(&ws)
        .arg("--cache-dir")
        .arg(&cache));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    let combined = format!("{stdout}{stderr}");
    assert!(
        !combined.contains("yanked") && !combined.contains("advisory"),
        "tidy must not honor DRACONIC_ADVISORY:\n{combined}"
    );
    let lock = fs::read_to_string(ws.join("draconic.lock")).unwrap();
    assert!(lock.contains("github.com/org/lib"), "{lock}");
    assert!(lock.contains("1.2.3"), "{lock}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn k11_05_get_honors_advisory_when_configured() {
    let root = temp_dir();
    let upstream = tagged_upstream(&root);
    let ws = root.join("app");
    write_app(&ws);
    let advisory = root.join("advisory.txt");
    fs::write(&advisory, "yank github.com/org/lib 1.2.3\n").unwrap();
    let cache = root.join("cache");

    let (code, stdout, stderr) = run(draconic()
        .env("DRACONIC_ADVISORY", advisory.to_str().unwrap())
        .arg("get")
        .arg("github.com/org/lib@1.2.3")
        .arg("--url")
        .arg(upstream.to_str().unwrap())
        .arg("--dir")
        .arg(&ws)
        .arg("--cache-dir")
        .arg(&cache));
    assert_ne!(code, 0, "get must refuse yanked version");
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("yanked") || combined.contains("advisory"),
        "stderr={stderr}\nstdout={stdout}"
    );
    assert!(!ws.join("draconic.lock").exists(), "must not pin yanked");
    let _ = fs::remove_dir_all(&root);
}
