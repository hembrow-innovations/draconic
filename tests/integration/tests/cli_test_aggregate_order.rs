//! ROADMAP C04.02: deterministic aggregate exit + stable failure summary order.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-c0402-{}-{}-{}",
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

fn write_failing_js_fixture(dir: &Path, file_stem: &str, id: &str) {
    fs::write(dir.join(format!("{file_stem}.drac")), "let x = 1;\n").unwrap();
    fs::write(
        dir.join(format!("{file_stem}.meta")),
        format!(
            "\
id: {id}
targets: js
js.exit: 0
js.check: if (x !== 99) process.exit(1);
"
        ),
    )
    .unwrap();
}

fn fail_ids(stdout: &str) -> Vec<&str> {
    stdout
        .lines()
        .filter(|l| l.starts_with("FAIL "))
        .filter_map(|l| l.split_whitespace().nth(1))
        .collect()
}

#[test]
fn cli_test_failure_summary_order_by_fixture_id() {
    let dir = temp_dir();
    write_failing_js_fixture(&dir, "z_fail", "alpha");
    write_failing_js_fixture(&dir, "a_fail", "zeta");

    let output = Command::new(draconic_bin())
        .arg("test")
        .arg("--jobs")
        .arg("2")
        .arg(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn draconic test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code().unwrap_or(255),
        1,
        "stdout={stdout}\nstderr={stderr}"
    );
    assert_eq!(
        fail_ids(&stdout),
        ["alpha", "zeta"],
        "FAIL summary must be fixture-id order:\n{stdout}"
    );
}
