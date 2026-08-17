//! ROADMAP U10: `draconic build --watch` / `check --watch`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn draconic() -> Command {
    Command::new(env!("CARGO_BIN_EXE_draconic"))
}

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-cli-watch-{}-{}-{}",
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

fn wait_until(deadline: Instant, mut pred: impl FnMut() -> bool) -> bool {
    while Instant::now() < deadline {
        if pred() {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    pred()
}

#[test]
fn help_lists_watch() {
    let output = draconic()
        .arg("help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("help");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--watch") || stdout.contains("watch"),
        "help should mention watch:\n{stdout}"
    );
}

#[test]
fn build_watch_rebuilds_on_source_change() {
    let dir = temp_dir();
    let src = write_program(&dir, "prog.drac", "let x = 1;\n");
    let out = dir.join("out.js");

    let mut child = draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg("--watch")
        .arg(&src)
        .arg("-o")
        .arg(&out)
        .env("DRACONIC_WATCH_POLL_MS", "50")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn build --watch");

    let deadline = Instant::now() + Duration::from_secs(10);
    assert!(
        wait_until(deadline, || out.is_file()),
        "initial build should write {}",
        out.display()
    );
    let first = fs::read_to_string(&out).expect("read first out");
    assert!(first.contains("let x"), "first emit:\n{first}");

    // Ensure mtime advances on all filesystems.
    thread::sleep(Duration::from_millis(20));
    fs::write(&src, "let y = 2;\n").unwrap();

    let rebuilt = wait_until(deadline, || {
        fs::read_to_string(&out)
            .map(|s| s.contains("let y"))
            .unwrap_or(false)
    });
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        rebuilt,
        "watch rebuild should emit let y; last out={}",
        fs::read_to_string(&out).unwrap_or_default()
    );
}

#[test]
fn check_watch_reruns_on_source_change() {
    let dir = temp_dir();
    let src = write_program(&dir, "ok.drac", "let x = 1;\n");
    let marker = dir.join("check-ok.marker");

    // Parent process observes rebuilds via marker file written by a tiny wrapper:
    // we assert check --watch accepts the flag and stays running after a change
    // without writing emit artifacts, then exits cleanly when killed.
    let mut child = draconic()
        .arg("check")
        .arg("--watch")
        .arg(&src)
        .env("DRACONIC_WATCH_POLL_MS", "50")
        .env("DRACONIC_WATCH_MARKER", &marker)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn check --watch");

    let deadline = Instant::now() + Duration::from_secs(10);
    assert!(
        wait_until(deadline, || marker.is_file()),
        "check --watch should write marker on each successful check"
    );
    let n1 = fs::read_to_string(&marker)
        .unwrap()
        .trim()
        .parse::<u64>()
        .unwrap_or(0);
    assert!(n1 >= 1, "marker count={n1}");

    thread::sleep(Duration::from_millis(20));
    fs::write(&src, "let z = 3;\n").unwrap();

    let bumped = wait_until(deadline, || {
        fs::read_to_string(&marker)
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|n| n > n1)
            .unwrap_or(false)
    });

    // No emit artifacts from check.
    assert!(!dir.join("ok.js").exists());

    let _ = child.kill();
    let _ = child.wait();
    assert!(bumped, "check --watch should re-run after source change");
}
