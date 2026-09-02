//! ROADMAP D05.02: LTO native artifacts are smaller than the default native build.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-lto-{}-{}-{}",
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

fn run(cmd: &mut Command) -> (i32, String, String) {
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn");
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

#[test]
fn lto_native_binary_is_smaller_than_default() {
    let dir = temp_dir();
    let src = dir.join("prog.drac");
    fs::write(&src, "let x: i32 = 42;\n").unwrap();

    let default_bin = dir.join("default");
    let lto_bin = dir.join("lto");
    let bin = draconic_bin();

    let (code, stdout, stderr) = run(Command::new(&bin)
        .arg("build")
        .arg("--target")
        .arg("native")
        .arg(&src)
        .arg("-o")
        .arg(&default_bin));
    assert_eq!(
        code, 0,
        "default native build failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(default_bin.is_file(), "missing {}", default_bin.display());

    let (code, stdout, stderr) = run(Command::new(&bin)
        .arg("build")
        .arg("--target")
        .arg("native")
        .arg("--lto")
        .arg(&src)
        .arg("-o")
        .arg(&lto_bin));
    assert_eq!(
        code, 0,
        "LTO native build failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(lto_bin.is_file(), "missing {}", lto_bin.display());

    let default_len = fs::metadata(&default_bin).unwrap().len();
    let lto_len = fs::metadata(&lto_bin).unwrap().len();
    assert!(
        lto_len < default_len,
        "LTO artifact must be smaller than the default native build\n\
         default_len={default_len} lto_len={lto_len} delta={}",
        default_len as i64 - lto_len as i64
    );

    let _ = fs::remove_dir_all(&dir);
}
