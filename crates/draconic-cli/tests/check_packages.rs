//! `draconic check` materialises locked package pins (`packages.check:ensure-locked`).

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
        "draconic-cli-check-pkg-{}-{}-{}",
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

fn run_code(cmd: &mut Command) -> (i32, String, String) {
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

fn git_stdout(args: &[&str], cwd: &Path) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn git");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// `packages.check:ensure-locked`: check fetches missing lock pins, then typechecks with no emit.
#[test]
fn check_ensures_locked() {
    let root = temp_dir();

    let upstream = root.join("upstream");
    fs::create_dir_all(&upstream).unwrap();
    git_ok(&["init"], &upstream);
    git_ok(&["config", "user.email", "test@draconic.local"], &upstream);
    git_ok(&["config", "user.name", "Draconic Test"], &upstream);
    git_ok(&["checkout", "-B", "main"], &upstream);
    fs::write(
        upstream.join("index.drac"),
        "export let value = 41;\nexport function inc(x) { return x + 1; }\n",
    )
    .unwrap();
    git_ok(&["add", "."], &upstream);
    git_ok(&["commit", "-m", "v1.0.0"], &upstream);
    git_ok(&["tag", "v1.0.0"], &upstream);
    let oid = git_stdout(&["rev-parse", "HEAD"], &upstream);

    let seed_cache = root.join("seed-cache");
    let (code, _stdout, stderr) = run_code(
        draconic()
            .arg("get")
            .arg("github.com/org/lib@1.0.0")
            .arg("--url")
            .arg(upstream.to_str().unwrap())
            .arg("--dir")
            .arg({
                let ws = root.join("seed-ws");
                fs::create_dir_all(&ws).unwrap();
                fs::write(
                    ws.join("draconic.toml"),
                    "module = \"github.com/acme/seed\"\n",
                )
                .unwrap();
                ws
            })
            .arg("--cache-dir")
            .arg(&seed_cache),
    );
    assert_eq!(code, 0, "seed get failed: {stderr}");
    let lock_src = fs::read_to_string(root.join("seed-ws/draconic.lock")).unwrap();
    let content_hash = lock_src
        .lines()
        .find_map(|l| l.trim().strip_prefix("content_hash = \""))
        .and_then(|s| s.strip_suffix('"'))
        .expect("content_hash in seed lock")
        .to_string();

    let ws = root.join("app");
    fs::create_dir_all(&ws).unwrap();
    fs::write(
        ws.join("draconic.toml"),
        format!(
            "module = \"github.com/acme/app\"\n\n[dependencies]\n\"github.com/org/lib\" = \"1.0.0\"\n\n[urls]\n\"github.com/org/lib\" = \"{}\"\n",
            upstream.display()
        ),
    )
    .unwrap();
    fs::write(
        ws.join("draconic.lock"),
        format!(
            r#"version = 1

[[package]]
path = "github.com/org/lib"
version = "1.0.0"
git_url = "{}"
commit_oid = "{oid}"
content_hash = "{content_hash}"
"#,
            upstream.display()
        ),
    )
    .unwrap();
    let main = ws.join("main.drac");
    fs::write(
        &main,
        "import { value, inc } from \"github.com/org/lib\";\nlet a = value;\nlet b = inc(value);\n",
    )
    .unwrap();

    let cache_mod = ws
        .join(".draconic/mod-cache/mod/github.com/org/lib")
        .join(&oid);
    assert!(
        !cache_mod.is_dir(),
        "cache must be empty before check auto-fetch"
    );

    let (code, _stdout, stderr) = run_code(draconic().arg("check").arg(&main));
    assert_eq!(
        code, 0,
        "check should fetch lock pins then typecheck; stderr={stderr}"
    );
    assert!(
        cache_mod.is_dir(),
        "check should materialize locked checkout at {}",
        cache_mod.display()
    );
    assert!(
        !ws.join("main.js").exists() && !ws.join("main.out.js").exists(),
        "check must not write JS emit"
    );
    assert!(
        !ws.join("main").exists() && !ws.join("main.out").exists(),
        "check must not write native emit"
    );

    let _ = fs::remove_dir_all(&root);
}
