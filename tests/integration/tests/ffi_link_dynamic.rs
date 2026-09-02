//! ROADMAP F05 / F05.01–F05.02: native build links a shared lib, resolves, and calls one C symbol.
//! F05 parent locks the combined link-dynamic / call-one-symbol surface.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_llvm::{
    build_c_dynamic_lib, build_native_binary, build_native_binary_with_dynamic_libs,
    dynamic_lib_file_name, emit_llvm_ir,
};
use draconic_frontend::compile_source;

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-f05-{}-{}-{}",
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

#[test]
fn e2e_native_build_links_dynamic_lib_and_resolves_symbol() {
    let dir = temp_dir();
    let c_src = dir.join("touch.c");
    fs::write(&c_src, "void draconic_link_dynamic_touch(void) {}\n").unwrap();
    let dylib = dir.join(dynamic_lib_file_name("touch"));
    build_c_dynamic_lib(&c_src, &dylib).expect("build shared lib");

    let module = compile_source(
        "extern \"C\" function draconic_link_dynamic_touch(): void;\ndraconic_link_dynamic_touch();\nlet x: i32 = 1;\n",
    )
    .expect("compile");
    let ll = emit_llvm_ir(&module).expect("emit_llvm_ir");

    let missing = dir.join("no_lib");
    let err = build_native_binary(&ll, Path::new(&missing))
        .expect_err("link without extra dylib must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("draconic_link_dynamic_touch")
            || msg.contains("undefined")
            || msg.contains("Unresolved"),
        "expected unresolved symbol, got {msg}"
    );

    let out = dir.join("prog");
    build_native_binary_with_dynamic_libs(&ll, Path::new(&out), &[dylib])
        .expect("build_native_binary_with_dynamic_libs");
    assert!(out.is_file(), "native binary missing at {}", out.display());
    let output = Command::new(&out).output().expect("run");
    assert!(
        output.status.success(),
        "exit {:?}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "1\n",
        "stdout must be the local let, proving the shared-lib symbol resolved"
    );
}

#[test]
fn e2e_native_build_calls_linked_dynamic_symbol() {
    let dir = temp_dir();
    let c_src = dir.join("add.c");
    fs::write(
        &c_src,
        "int draconic_link_dynamic_add(int a, int b) { return a + b; }\n",
    )
    .unwrap();
    let dylib = dir.join(dynamic_lib_file_name("add"));
    build_c_dynamic_lib(&c_src, &dylib).expect("build shared lib");

    let module = compile_source(
        "extern \"C\" function draconic_link_dynamic_add(a: i32, b: i32): i32;\nlet s: i32 = draconic_link_dynamic_add(20, 22);\nlet t: i32 = draconic_link_dynamic_add(-5, 12);\n",
    )
    .expect("compile");
    let ll = emit_llvm_ir(&module).expect("emit_llvm_ir");

    let out = dir.join("prog");
    build_native_binary_with_dynamic_libs(&ll, Path::new(&out), &[dylib])
        .expect("build_native_binary_with_dynamic_libs");
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

#[test]
fn e2e_native_build_links_and_calls_dynamic_surface() {
    let dir = temp_dir();
    let c_src = dir.join("surface.c");
    fs::write(
        &c_src,
        "void draconic_link_dynamic_touch(void) {}\nint draconic_link_dynamic_add(int a, int b) { return a + b; }\n",
    )
    .unwrap();
    let dylib = dir.join(dynamic_lib_file_name("surface"));
    build_c_dynamic_lib(&c_src, &dylib).expect("build shared lib");

    let module = compile_source(
        "extern \"C\" function draconic_link_dynamic_touch(): void;\nextern \"C\" function draconic_link_dynamic_add(a: i32, b: i32): i32;\ndraconic_link_dynamic_touch();\nlet x: i32 = 1;\nlet s: i32 = draconic_link_dynamic_add(20, 22);\nlet t: i32 = draconic_link_dynamic_add(-5, 12);\n",
    )
    .expect("compile");
    let ll = emit_llvm_ir(&module).expect("emit_llvm_ir");

    let out = dir.join("prog");
    build_native_binary_with_dynamic_libs(&ll, Path::new(&out), &[dylib])
        .expect("build_native_binary_with_dynamic_libs");
    let output = Command::new(&out).output().expect("run");
    assert!(
        output.status.success(),
        "exit {:?}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "1\n42\n7\n",
        "stdout must be resolve print plus C-computed returns"
    );
}

#[test]
fn e2e_native_missing_dynamic_lib_is_typed_error() {
    let dir = temp_dir();
    let module = compile_source(
        "extern \"C\" function draconic_link_dynamic_add(a: i32, b: i32): i32;\nlet s: i32 = draconic_link_dynamic_add(20, 22);\n",
    )
    .expect("compile");
    let ll = emit_llvm_ir(&module).expect("emit_llvm_ir");
    let missing = dir.join(dynamic_lib_file_name("no_such"));
    assert!(!missing.is_file(), "fixture path must not exist");
    let out = dir.join("no_bin");
    let err = build_native_binary_with_dynamic_libs(&ll, Path::new(&out), &[missing.clone()])
        .expect_err("missing dylib must fail");
    assert_eq!(
        err.code,
        Some(draconic_diagnostics::codes::MISSING_DYNAMIC_LIB),
        "missing dylib must carry E0402, got {err}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("E0402"),
        "typed error must include E0402, got {msg}"
    );
    assert!(
        msg.contains("dynamic lib not found"),
        "typed error must name the miss, got {msg}"
    );
    assert!(
        msg.contains(&missing.display().to_string()),
        "typed error must include the path, got {msg}"
    );
}
