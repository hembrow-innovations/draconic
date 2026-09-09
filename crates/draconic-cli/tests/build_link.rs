//! Native `--link` of static and dynamic C libraries.

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
        "draconic-cli-build-{}-{}-{}",
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

fn run_ok(cmd: &mut Command) -> (String, String) {
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn draconic");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "command failed: {:?}\nstdout={stdout}\nstderr={stderr}",
        cmd
    );
    (stdout, stderr)
}

/// F04.01: `build --target native --link lib.a` resolves one C symbol from the archive.
#[test]
fn build_native_link_static_lib_resolves_symbol() {
    use draconic_backend_llvm::build_c_static_lib;

    let dir = temp_dir();
    let c_src = dir.join("touch.c");
    fs::write(&c_src, "void draconic_link_static_touch(void) {}\n").unwrap();
    let archive = dir.join("libtouch.a");
    build_c_static_lib(&c_src, &archive).expect("build .a");

    let src = write_program(
        &dir,
        "prog.drac",
        "extern \"C\" function draconic_link_static_touch(): void;\ndraconic_link_static_touch();\nlet x: i32 = 1;\n",
    );
    let out_fail = dir.join("fail");
    let failed = draconic()
        .arg("build")
        .arg("--target")
        .arg("native")
        .arg(&src)
        .arg("-o")
        .arg(&out_fail)
        .output()
        .expect("spawn");
    assert!(
        !failed.status.success(),
        "build without --link must fail to resolve the C symbol"
    );
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(
        stderr.contains("draconic_link_static_touch")
            || stderr.contains("undefined")
            || stderr.contains("Unresolved"),
        "stderr={stderr}"
    );

    let out = dir.join("prog");
    run_ok(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("native")
            .arg("--link")
            .arg(&archive)
            .arg(&src)
            .arg("-o")
            .arg(&out),
    );
    assert!(out.is_file(), "native binary missing at {}", out.display());
}

/// F05.01: `build --target native --link lib.dylib` resolves one C symbol from the shared lib.
#[test]
fn build_native_link_dynamic_lib_resolves_symbol() {
    use draconic_backend_llvm::{build_c_dynamic_lib, dynamic_lib_file_name};

    let dir = temp_dir();
    let c_src = dir.join("touch.c");
    fs::write(&c_src, "void draconic_link_dynamic_touch(void) {}\n").unwrap();
    let dylib = dir.join(dynamic_lib_file_name("touch"));
    build_c_dynamic_lib(&c_src, &dylib).expect("build shared lib");

    let src = write_program(
        &dir,
        "prog.drac",
        "extern \"C\" function draconic_link_dynamic_touch(): void;\ndraconic_link_dynamic_touch();\nlet x: i32 = 1;\n",
    );
    let out_fail = dir.join("fail");
    let failed = draconic()
        .arg("build")
        .arg("--target")
        .arg("native")
        .arg(&src)
        .arg("-o")
        .arg(&out_fail)
        .output()
        .expect("spawn");
    assert!(
        !failed.status.success(),
        "build without --link must fail to resolve the C symbol"
    );
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(
        stderr.contains("draconic_link_dynamic_touch")
            || stderr.contains("undefined")
            || stderr.contains("Unresolved"),
        "stderr={stderr}"
    );

    let out = dir.join("prog");
    run_ok(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("native")
            .arg("--link")
            .arg(&dylib)
            .arg(&src)
            .arg("-o")
            .arg(&out),
    );
    assert!(out.is_file(), "native binary missing at {}", out.display());
}

/// F05.02: `build --target native --link lib.dylib` then run: stdout is the C return value.
#[test]
fn build_native_link_dynamic_lib_call_end_to_end() {
    use draconic_backend_llvm::{build_c_dynamic_lib, dynamic_lib_file_name};

    let dir = temp_dir();
    let c_src = dir.join("add.c");
    fs::write(
        &c_src,
        "int draconic_link_dynamic_add(int a, int b) { return a + b; }\n",
    )
    .unwrap();
    let dylib = dir.join(dynamic_lib_file_name("add"));
    build_c_dynamic_lib(&c_src, &dylib).expect("build shared lib");

    let src = write_program(
        &dir,
        "prog.drac",
        "extern \"C\" function draconic_link_dynamic_add(a: i32, b: i32): i32;\nlet s: i32 = draconic_link_dynamic_add(20, 22);\nlet t: i32 = draconic_link_dynamic_add(-5, 12);\n",
    );
    let out = dir.join("prog");
    run_ok(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("native")
            .arg("--link")
            .arg(&dylib)
            .arg(&src)
            .arg("-o")
            .arg(&out),
    );
    let output = Command::new(&out).output().expect("run");
    assert!(
        output.status.success(),
        "exit {:?}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "42\n7\n",
        "stdout must be C-computed returns"
    );
}

/// F05.02: `--link` of a missing shared lib is E0402, not a raw linker dump.
#[test]
fn build_native_link_dynamic_lib_missing_is_typed_error() {
    use draconic_backend_llvm::dynamic_lib_file_name;

    let dir = temp_dir();
    let src = write_program(
        &dir,
        "prog.drac",
        "extern \"C\" function draconic_link_dynamic_add(a: i32, b: i32): i32;\nlet s: i32 = draconic_link_dynamic_add(20, 22);\n",
    );
    let missing = dir.join(dynamic_lib_file_name("no_such"));
    let out_fail = dir.join("fail");
    let failed = draconic()
        .arg("build")
        .arg("--target")
        .arg("native")
        .arg("--link")
        .arg(&missing)
        .arg(&src)
        .arg("-o")
        .arg(&out_fail)
        .output()
        .expect("spawn");
    assert!(
        !failed.status.success(),
        "build with missing --link dylib must fail"
    );
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(
        stderr.contains("E0402"),
        "missing dylib must be typed E0402, stderr={stderr}"
    );
    assert!(stderr.contains("dynamic lib not found"), "stderr={stderr}");
}

/// F04.02: `build --target native --link lib.a` then run: stdout is the C return value.
#[test]
fn build_native_link_static_lib_call_end_to_end() {
    use draconic_backend_llvm::build_c_static_lib;

    let dir = temp_dir();
    let c_src = dir.join("add.c");
    fs::write(
        &c_src,
        "int draconic_link_static_add(int a, int b) { return a + b; }\n",
    )
    .unwrap();
    let archive = dir.join("libadd.a");
    build_c_static_lib(&c_src, &archive).expect("build .a");

    let src = write_program(
        &dir,
        "prog.drac",
        "extern \"C\" function draconic_link_static_add(a: i32, b: i32): i32;\nlet s: i32 = draconic_link_static_add(20, 22);\nlet t: i32 = draconic_link_static_add(-5, 12);\n",
    );
    let out = dir.join("prog");
    run_ok(
        draconic()
            .arg("build")
            .arg("--target")
            .arg("native")
            .arg("--link")
            .arg(&archive)
            .arg(&src)
            .arg("-o")
            .arg(&out),
    );
    let output = Command::new(&out).output().expect("run");
    assert!(
        output.status.success(),
        "exit {:?}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "42\n7\n",
        "stdout must be C-computed returns"
    );
}
