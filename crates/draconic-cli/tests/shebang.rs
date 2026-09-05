//! ROADMAP P05: documented `#!/usr/bin/env draconic` shebang run path (with U14).

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const DOCUMENTED_SHEBANG: &str = "#!/usr/bin/env draconic";
const EXAMPLE_STDOUT: &str = "hello-shebang";

fn draconic() -> Command {
    Command::new(env!("CARGO_BIN_EXE_draconic"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn shebang_example() -> PathBuf {
    repo_root()
        .join("examples/shebang/hello.drac")
        .canonicalize()
        .expect("examples/shebang/hello.drac")
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

/// Docs name the env-draconic shebang on the README and Reference CLI page.
#[test]
fn docs_name_env_draconic_shebang() {
    let readme = fs::read_to_string(repo_root().join("README.md")).expect("README.md");
    let cli = fs::read_to_string(repo_root().join("website/cli.md")).expect("website/cli.md");
    assert!(
        readme.contains(DOCUMENTED_SHEBANG),
        "README.md must name {DOCUMENTED_SHEBANG}"
    );
    assert!(
        cli.contains(DOCUMENTED_SHEBANG),
        "website/cli.md must name {DOCUMENTED_SHEBANG}"
    );
}

/// In-repo example starts with the documented shebang line.
#[test]
fn example_starts_with_env_draconic_shebang() {
    let src = fs::read_to_string(shebang_example()).expect("read shebang example");
    assert!(
        src.starts_with(DOCUMENTED_SHEBANG),
        "examples/shebang/hello.drac must start with {DOCUMENTED_SHEBANG}:\n{src}"
    );
}

/// U14 `draconic run` executes the documented shebang example.
#[test]
fn example_runs_via_u14_run() {
    let src = shebang_example();
    let (code, stdout, stderr) = run(draconic().arg("run").arg(&src));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains(EXAMPLE_STDOUT),
        "stdout={stdout}\nstderr={stderr}"
    );
}

/// A stranger can chmod the example and execute it when `draconic` is on PATH.
#[test]
fn example_runs_as_executable_shebang() {
    use std::os::unix::fs::PermissionsExt;

    let src = shebang_example();
    let mut perms = fs::metadata(&src).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&src, perms).expect("chmod +x shebang example");

    let draconic_bin = PathBuf::from(env!("CARGO_BIN_EXE_draconic"));
    let bin_dir = draconic_bin.parent().expect("bin dir");
    let path = format!(
        "{}:{}",
        bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let output = Command::new(&src)
        .env("PATH", &path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn shebang example");
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains(EXAMPLE_STDOUT),
        "stdout={stdout}\nstderr={stderr}"
    );
}
