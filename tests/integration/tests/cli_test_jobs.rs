//! ROADMAP C04.01: `draconic test` runs fixtures on a worker pool (N>1).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-c0401-{}-{}-{}",
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

fn write_js_fixture(dir: &Path, id: &str, source: &str) {
    fs::write(dir.join(format!("{id}.drac")), source).unwrap();
    fs::write(
        dir.join(format!("{id}.meta")),
        format!(
            "\
id: {id}
targets: js
js.exit: 0
"
        ),
    )
    .unwrap();
}

fn barrier_program(self_name: &str, peer_name: &str) -> String {
    format!(
        r#"
let dir = envGet("DRACONIC_C0401_BARRIER");
writeFileText(dir + "/{self_name}.started", "1");
let t0 = Date.now();
while (!exists(dir + "/{peer_name}.started")) {{
  if (Date.now() - t0 > 8000) {{
    throw 1;
  }}
}}
"#
    )
}

#[test]
fn cli_test_jobs_two_overlaps_barrier() {
    let dir = temp_dir();
    let barrier = dir.join("barrier");
    fs::create_dir_all(&barrier).unwrap();
    write_js_fixture(&dir, "left", &barrier_program("left", "right"));
    write_js_fixture(&dir, "right", &barrier_program("right", "left"));

    let output = Command::new(draconic_bin())
        .arg("test")
        .arg("--jobs")
        .arg("2")
        .env("DRACONIC_C0401_BARRIER", &barrier)
        .arg(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn draconic test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code().unwrap_or(1),
        0,
        "stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("left") && stdout.contains("right"),
        "stdout={stdout}"
    );
}
