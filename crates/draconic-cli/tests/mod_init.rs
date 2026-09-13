//! `draconic mod init <module_path>` writes a first module manifest.

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
        "draconic-cli-mod-init-{}-{}-{}",
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
fn mod_init_writes_module_and_get_can_follow() {
    let root = temp_dir();
    let upstream = tagged_upstream(&root);
    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    let cache = root.join("cache");

    let (code, stdout, stderr) = run(draconic()
        .arg("mod")
        .arg("init")
        .arg("github.com/acme/app")
        .arg("--dir")
        .arg(&ws));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");

    let mf = fs::read_to_string(ws.join("draconic.toml")).unwrap();
    assert_eq!(mf, "module = \"github.com/acme/app\"\n");
    assert!(!mf.contains("version"), "{mf}");
    assert!(!mf.contains("exports"), "{mf}");

    let (code, stdout, stderr) = run(draconic()
        .arg("get")
        .arg("github.com/org/lib@1.2.3")
        .arg("--url")
        .arg(upstream.to_str().unwrap())
        .arg("--dir")
        .arg(&ws)
        .arg("--cache-dir")
        .arg(&cache));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");

    let mf = fs::read_to_string(ws.join("draconic.toml")).unwrap();
    assert!(mf.contains("[dependencies]"), "{mf}");
    assert!(mf.contains("github.com/org/lib"), "{mf}");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn mod_init_refuses_existing_manifest() {
    let root = temp_dir();
    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    let path = ws.join("draconic.toml");
    let original = "module = \"github.com/org/old\"\n";
    fs::write(&path, original).unwrap();

    let (code, _stdout, stderr) = run(draconic()
        .arg("mod")
        .arg("init")
        .arg("github.com/org/new")
        .arg("--dir")
        .arg(&ws));
    assert_ne!(code, 0, "stderr={stderr}");
    assert!(stderr.contains("already exists"), "stderr={stderr}");
    assert_eq!(fs::read_to_string(&path).unwrap(), original);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn mod_init_help_lists_init() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("mod init"),
        "help should list mod init:\n{stdout}"
    );
}
