//! ROADMAP D03.02: same source + pin → byte-identical or documented-equivalent emit.
//!
//! **Byte-identical emit** is the JS artifact and LLVM IR text produced from the
//! same Program source by the same toolchain. Two independent compiles must
//! match byte-for-byte.
//!
//! **Documented-equivalent:** linked native binaries are not the emit under this
//! row. DWARF directories follow the source path (same path → identical IR).
//! Mach-O/ELF timestamps, UUIDs, and linker noise may differ; equivalence for
//! native is identical LLVM IR for the same source, pin, and path. Timestamp
//! and path policy for packaged binaries is D03.01.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_js::emit_js;
use draconic_backend_llvm::{emit_llvm_ir, emit_llvm_ir_with_debug, SourceDebug};
use draconic_frontend::compile_source;

const SRC: &str = "let x = 1 + 2;\nfunction add(a, b) { return a + b; }\n";

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-reproducible-emit-{}-{}-{}",
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

fn running_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
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

fn compile_js(src: &str) -> String {
    let module = compile_source(src).expect("compile");
    emit_js(&module).expect("emit_js")
}

fn compile_llvm(src: &str) -> String {
    let module = compile_source(src).expect("compile");
    emit_llvm_ir(&module).expect("emit_llvm_ir")
}

#[test]
fn js_emit_is_byte_identical_across_two_compiles() {
    let a = compile_js(SRC);
    let b = compile_js(SRC);
    assert_eq!(a, b, "JS emit must be byte-identical for the same source");
    assert!(!a.is_empty(), "expected non-empty JS emit");
}

#[test]
fn llvm_ir_emit_is_byte_identical_across_two_compiles() {
    let a = compile_llvm(SRC);
    let b = compile_llvm(SRC);
    assert_eq!(
        a, b,
        "LLVM IR emit must be byte-identical for the same source"
    );
    assert!(!a.is_empty(), "expected non-empty LLVM IR");
}

#[test]
fn llvm_ir_with_debug_same_path_is_byte_identical() {
    let dir = temp_dir();
    let src_path = write_program(&dir, "prog.drac", SRC);
    let debug = SourceDebug::from_path(&src_path, SRC);
    let module_a = compile_source(SRC).expect("compile a");
    let module_b = compile_source(SRC).expect("compile b");
    let a = emit_llvm_ir_with_debug(&module_a, &debug).expect("emit a");
    let b = emit_llvm_ir_with_debug(&module_b, &debug).expect("emit b");
    assert_eq!(
        a, b,
        "LLVM IR + DWARF must be byte-identical for the same source path"
    );
    assert!(a.contains("!DIFile"), "expected DWARF file metadata");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cli_js_build_twice_with_matching_pin_is_byte_identical() {
    let dir = temp_dir();
    let ver = running_version();
    fs::write(
        dir.join("draconic.toml"),
        format!(
            "module = \"github.com/acme/app\"\ntoolchain = {{ version = \"{ver}\", required = true }}\n"
        ),
    )
    .unwrap();
    let src = write_program(&dir, "prog.drac", SRC);
    let out_a = dir.join("a.js");
    let out_b = dir.join("b.js");

    let (code_a, stdout_a, stderr_a) = run(Command::new(draconic_bin())
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg(&src)
        .arg("-o")
        .arg(&out_a));
    assert_eq!(code_a, 0, "stdout={stdout_a}\nstderr={stderr_a}");

    let (code_b, stdout_b, stderr_b) = run(Command::new(draconic_bin())
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg(&src)
        .arg("-o")
        .arg(&out_b));
    assert_eq!(code_b, 0, "stdout={stdout_b}\nstderr={stderr_b}");

    let bytes_a = fs::read(&out_a).expect("read a.js");
    let bytes_b = fs::read(&out_b).expect("read b.js");
    assert_eq!(
        bytes_a, bytes_b,
        "CLI JS artifacts must be byte-identical for the same source + pin"
    );
    assert!(!bytes_a.is_empty(), "expected non-empty JS artifact");

    let _ = fs::remove_dir_all(&dir);
}
