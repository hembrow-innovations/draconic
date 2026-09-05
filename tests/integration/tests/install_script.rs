//! ROADMAP D01.02: install script downloads the host artifact and places `draconic` on PATH.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-d0102-{}-{}-{}",
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

fn host_triple() -> String {
    let output = Command::new("rustc")
        .arg("-vV")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("rustc -vV");
    assert!(
        output.status.success(),
        "rustc -vV failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("host: ") {
            let t = rest.trim();
            assert!(!t.is_empty(), "empty host triple in rustc -vV:\n{stdout}");
            return t.to_string();
        }
    }
    panic!("no host: line in rustc -vV:\n{stdout}");
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

fn artifact_name(triple: &str) -> String {
    if cfg!(windows) {
        format!("draconic-{triple}.exe")
    } else {
        format!("draconic-{triple}")
    }
}

fn installed_name() -> &'static str {
    if cfg!(windows) {
        "draconic.exe"
    } else {
        "draconic"
    }
}

fn run_install(args: &[&str]) -> (i32, String, String) {
    let script = repo_root().join("scripts/install.sh");
    assert!(
        script.is_file(),
        "missing {} (D01.02 install script)",
        script.display()
    );
    let output = Command::new("bash")
        .arg(&script)
        .args(args)
        .current_dir(repo_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn install.sh");
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

#[test]
fn install_script_exists() {
    let script = repo_root().join("scripts/install.sh");
    assert!(
        script.is_file(),
        "missing {} (D01.02 one-line install script)",
        script.display()
    );
    let text = fs::read_to_string(&script).expect("read install.sh");
    assert!(
        text.contains("curl") || text.contains("https://"),
        "install.sh should download the host-triple artifact:\n{text}"
    );
}

#[test]
fn readme_documents_one_line_install() {
    let readme = repo_root().join("README.md");
    let text = fs::read_to_string(&readme).expect("read README");
    assert!(
        text.contains("scripts/install.sh") || text.contains("install.sh"),
        "README should document the one-line install script:\n{text}"
    );
    assert!(
        text.contains("curl") && (text.contains("| sh") || text.contains("| bash")),
        "README should show a curl | sh one-liner:\n{text}"
    );
}

#[test]
fn install_script_places_draconic_in_bin_dir() {
    let dist = temp_dir();
    let dest = temp_dir();
    let triple = host_triple();
    let artifact = dist.join(artifact_name(&triple));
    fs::copy(draconic_bin(), &artifact).expect("stage fake dist artifact");

    let (code, stdout, stderr) = run_install(&[
        "--from",
        dist.to_str().unwrap(),
        "--dir",
        dest.to_str().unwrap(),
    ]);
    assert_eq!(
        code, 0,
        "install.sh failed\nstdout={stdout}\nstderr={stderr}"
    );

    let placed = dest.join(installed_name());
    assert!(
        placed.is_file(),
        "expected {} on PATH dir\nstdout={stdout}\nstderr={stderr}",
        placed.display()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&placed).unwrap().permissions().mode();
        assert!(
            mode & 0o111 != 0,
            "installed binary should be executable, mode={mode:#o}"
        );
    }
}

#[test]
fn install_script_binary_is_runnable_from_install_dir() {
    let dist = temp_dir();
    let dest = temp_dir();
    let triple = host_triple();
    fs::copy(draconic_bin(), dist.join(artifact_name(&triple))).expect("stage artifact");

    let (code, stdout, stderr) = run_install(&[
        "--from",
        dist.to_str().unwrap(),
        "--dir",
        dest.to_str().unwrap(),
    ]);
    assert_eq!(
        code, 0,
        "install.sh failed\nstdout={stdout}\nstderr={stderr}"
    );

    let placed = dest.join(installed_name());
    let output = Command::new(&placed)
        .arg("-V")
        .env("PATH", dest.to_str().unwrap())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run installed draconic -V");
    assert!(
        output.status.success(),
        "installed draconic -V failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let ver = String::from_utf8_lossy(&output.stdout);
    assert!(
        ver.contains("draconic"),
        "installed -V should print version:\n{ver}"
    );
}

#[test]
fn install_script_downloads_from_http() {
    let dist = temp_dir();
    let dest = temp_dir();
    let triple = host_triple();
    let name = artifact_name(&triple);
    fs::copy(draconic_bin(), dist.join(&name)).expect("stage artifact");

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind http");
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let mut server = Command::new("python3")
        .args([
            "-m",
            "http.server",
            "--bind",
            "127.0.0.1",
            "--directory",
            dist.to_str().unwrap(),
            &port.to_string(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("python3 http.server");

    let url = format!("http://127.0.0.1:{port}/{name}");
    let mut ready = false;
    for _ in 0..50 {
        if let Ok(ok) = Command::new("curl")
            .args(["-fsS", "-o", "/dev/null", "--max-time", "1", &url])
            .status()
        {
            if ok.success() {
                ready = true;
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    if !ready {
        let _ = server.kill();
        panic!("http.server did not become ready at {url}");
    }

    let (code, stdout, stderr) = run_install(&["--from", &url, "--dir", dest.to_str().unwrap()]);
    let _ = server.kill();
    let _ = server.wait();

    assert_eq!(
        code, 0,
        "install.sh HTTP download failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        dest.join(installed_name()).is_file(),
        "download should place {} \nstdout={stdout}\nstderr={stderr}",
        dest.join(installed_name()).display()
    );
}
