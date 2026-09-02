//! ROADMAP D05.01: stripped native artifacts are smaller or lack symbols.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-strip-{}-{}-{}",
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

fn has_named_symbols(bin: &Path) -> bool {
    let output = Command::new("nm")
        .arg(bin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().any(|line| {
        let t = line.trim();
        !t.is_empty() && !t.contains("no name list") && !t.contains("no symbols")
    })
}

#[test]
fn stripped_native_binary_is_smaller_or_lacks_symbols() {
    let dir = temp_dir();
    let src = dir.join("prog.drac");
    fs::write(&src, "let x: i32 = 42;\n").unwrap();

    let unstripped = dir.join("unstripped");
    let stripped = dir.join("stripped");
    let bin = draconic_bin();

    let (code, stdout, stderr) = run(Command::new(&bin)
        .arg("build")
        .arg("--target")
        .arg("native")
        .arg(&src)
        .arg("-o")
        .arg(&unstripped));
    assert_eq!(
        code, 0,
        "unstripped build failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(unstripped.is_file(), "missing {}", unstripped.display());

    let (code, stdout, stderr) = run(Command::new(&bin)
        .arg("build")
        .arg("--target")
        .arg("native")
        .arg("--strip")
        .arg(&src)
        .arg("-o")
        .arg(&stripped));
    assert_eq!(
        code, 0,
        "stripped build failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stripped.is_file(), "missing {}", stripped.display());

    let unstripped_len = fs::metadata(&unstripped).unwrap().len();
    let stripped_len = fs::metadata(&stripped).unwrap().len();
    let unstripped_syms = has_named_symbols(&unstripped);
    let stripped_syms = has_named_symbols(&stripped);

    assert!(
        stripped_len < unstripped_len || (unstripped_syms && !stripped_syms),
        "stripped artifact must be smaller or lack symbols the unstripped build kept\n\
         unstripped_len={unstripped_len} stripped_len={stripped_len}\n\
         unstripped_syms={unstripped_syms} stripped_syms={stripped_syms}"
    );

    let _ = fs::remove_dir_all(&dir);
}
