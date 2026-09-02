//! ROADMAP D03: same source + pin → documented-equivalent artifacts.
//!
//! Combined surface for the parent row: one Program, one matching toolchain pin,
//! both backends. **Byte-identical** is the JS artifact. **Documented-equivalent**
//! for native is identical LLVM IR for the same source, pin, and path (linked
//! Mach-O/ELF timestamps and UUIDs are not the artifact this row compares).
//! Timestamp/path policy for packaged binaries is D03.01.

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
        "draconic-integration-reproducible-builds-{}-{}-{}",
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

fn write_matching_pin(dir: &Path) {
    let ver = running_version();
    fs::write(
        dir.join("draconic.toml"),
        format!(
            "module = \"github.com/acme/app\"\ntoolchain = {{ version = \"{ver}\", required = true }}\n"
        ),
    )
    .unwrap();
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

fn cli_build_js(src: &Path, out: &Path) -> Vec<u8> {
    let (code, stdout, stderr) = run(Command::new(draconic_bin())
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg(src)
        .arg("-o")
        .arg(out));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    let bytes = fs::read(out).expect("read js artifact");
    assert!(!bytes.is_empty(), "expected non-empty JS artifact");
    bytes
}

/// One Program + matching pin: JS artifacts and LLVM IR both match across two builds.
#[test]
fn same_source_and_pin_js_and_llvm_ir_are_documented_equivalent() {
    let dir = temp_dir();
    write_matching_pin(&dir);
    let src = write_program(&dir, "prog.drac", SRC);

    let js_a = cli_build_js(&src, &dir.join("a.js"));
    let js_b = cli_build_js(&src, &dir.join("b.js"));
    assert_eq!(
        js_a, js_b,
        "JS artifacts must be byte-identical for the same source + pin"
    );

    let debug = SourceDebug::from_path(&src, SRC);
    let module_a = compile_source(SRC).expect("compile a");
    let module_b = compile_source(SRC).expect("compile b");
    let ir_a = emit_llvm_ir_with_debug(&module_a, &debug).expect("emit a");
    let ir_b = emit_llvm_ir_with_debug(&module_b, &debug).expect("emit b");
    assert_eq!(
        ir_a, ir_b,
        "LLVM IR must be byte-identical for the same source, pin, and path"
    );
    assert!(!ir_a.is_empty(), "expected non-empty LLVM IR");

    let js_lib_a = emit_js(&module_a).expect("emit_js a");
    let js_lib_b = emit_js(&module_b).expect("emit_js b");
    assert_eq!(js_lib_a, js_lib_b, "in-process JS emit must match");
    assert_eq!(
        js_a,
        js_lib_a.as_bytes(),
        "CLI JS artifact must match in-process emit for the same source + pin"
    );

    let _ = fs::remove_dir_all(&dir);
}

/// Native documented-equivalent is IR identity, not linked-binary identity.
#[test]
fn native_documented_equivalent_is_llvm_ir_for_same_source_pin_and_path() {
    let dir = temp_dir();
    write_matching_pin(&dir);
    let src = write_program(&dir, "prog.drac", SRC);

    let (code, stdout, stderr) = run(Command::new(draconic_bin()).arg("check").arg(&src));
    assert_eq!(code, 0, "pin must match\nstdout={stdout}\nstderr={stderr}");

    let ir_a = emit_llvm_ir(&compile_source(SRC).expect("compile a")).expect("ir a");
    let ir_b = emit_llvm_ir(&compile_source(SRC).expect("compile b")).expect("ir b");
    assert_eq!(
        ir_a, ir_b,
        "native documented-equivalent is identical LLVM IR"
    );

    let _ = fs::remove_dir_all(&dir);
}
