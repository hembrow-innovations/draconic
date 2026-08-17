//! ROADMAP U14: `draconic run` — build+execute convenience; shebang-friendly.

use std::fs;
use std::os::unix::fs::PermissionsExt;
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
        "draconic-cli-run-{}-{}-{}",
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

#[test]
fn help_lists_run() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("run"),
        "help should list run:\n{stdout}"
    );
}

#[test]
fn run_requires_file() {
    let (code, _stdout, stderr) = run(draconic().arg("run"));
    assert_ne!(code, 0);
    assert!(
        stderr.contains("usage") || stderr.contains("file"),
        "stderr={stderr}"
    );
}

#[test]
fn run_target_js_executes_console_log() {
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "hi.drac",
        "let console = globalThis.console;\nconsole.log(\"hello-run-js\");\n",
    );

    let (code, stdout, stderr) = run(
        draconic()
            .arg("run")
            .arg("--target")
            .arg("js")
            .arg(&src),
    );
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("hello-run-js"),
        "stdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn run_defaults_to_js() {
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "hi.drac",
        "let console = globalThis.console;\nconsole.log(\"default-js\");\n",
    );

    let (code, stdout, stderr) = run(draconic().arg("run").arg(&src));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("default-js"),
        "stdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn run_target_native_executes_scalar() {
    let dir = temp_dir();
    let src = write_program(&dir, "n.drac", "let x: i32 = 7;\n");

    let (code, stdout, stderr) = run(
        draconic()
            .arg("run")
            .arg("--target")
            .arg("native")
            .arg(&src),
    );
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert_eq!(stdout, "7\n", "stdout={stdout:?}\nstderr={stderr}");
}

#[test]
fn run_reports_parse_error() {
    let dir = temp_dir();
    let src = write_program(&dir, "bad.drac", "let = ;");
    let (code, _stdout, stderr) = run(draconic().arg("run").arg(&src));
    assert_ne!(code, 0);
    assert!(stderr.contains("error"), "stderr={stderr}");
}

#[test]
fn run_forwards_program_args_to_js() {
    let dir = temp_dir();
    // process.argv[2] is first user arg when node runs a file.
    let src = write_program(
        &dir,
        "args.drac",
        r#"
let console = globalThis.console;
let a = globalThis.process.argv[2];
console.log(a);
"#,
    );

    let (code, stdout, stderr) = run(
        draconic()
            .arg("run")
            .arg("--target")
            .arg("js")
            .arg(&src)
            .arg("from-cli"),
    );
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("from-cli"),
        "stdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn shebang_bare_path_runs_like_run() {
    // `#!/usr/bin/env draconic` invokes: draconic <script-path> [args...]
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "script.drac",
        "#!/usr/bin/env draconic\nlet console = globalThis.console;\nconsole.log(\"shebang-ok\");\n",
    );

    let (code, stdout, stderr) = run(draconic().arg(&src));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("shebang-ok"),
        "stdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn shebang_executable_script_via_env_path() {
    // End-to-end: executable file with shebang, PATH includes draconic dir.
    let dir = temp_dir();
    let script = dir.join("hello");
    fs::write(
        &script,
        "#!/usr/bin/env draconic\nlet console = globalThis.console;\nconsole.log(\"exec-shebang\");\n",
    )
    .unwrap();
    let mut perms = fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script, perms).unwrap();

    let draconic_bin = PathBuf::from(env!("CARGO_BIN_EXE_draconic"));
    let bin_dir = draconic_bin.parent().expect("bin dir");
    let path = format!(
        "{}:{}",
        bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let output = Command::new(&script)
        .env("PATH", &path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn shebang script");
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("exec-shebang"),
        "stdout={stdout}\nstderr={stderr}"
    );
}
