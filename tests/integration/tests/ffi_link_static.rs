//! ROADMAP F04.01: native build links an extra `.a` and resolves one C symbol.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_llvm::{
    build_c_static_lib, build_native_binary, build_native_binary_with_static_libs, emit_llvm_ir,
};
use draconic_frontend::compile_source;

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-f04-01-{}-{}-{}",
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
fn e2e_native_build_links_static_lib_and_resolves_symbol() {
    let dir = temp_dir();
    let c_src = dir.join("touch.c");
    fs::write(&c_src, "void draconic_link_static_touch(void) {}\n").unwrap();
    let archive = dir.join("libtouch.a");
    build_c_static_lib(&c_src, &archive).expect("build .a");

    let module = compile_source(
        "extern \"C\" function draconic_link_static_touch(): void;\ndraconic_link_static_touch();\nlet x: i32 = 1;\n",
    )
    .expect("compile");
    let ll = emit_llvm_ir(&module).expect("emit_llvm_ir");

    let missing = dir.join("no_lib");
    let err = build_native_binary(&ll, Path::new(&missing))
        .expect_err("link without extra .a must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("draconic_link_static_touch")
            || msg.contains("undefined")
            || msg.contains("Unresolved"),
        "expected unresolved symbol, got {msg}"
    );

    let out = dir.join("prog");
    build_native_binary_with_static_libs(&ll, Path::new(&out), &[archive])
        .expect("build_native_binary_with_static_libs");
    assert!(out.is_file(), "native binary missing at {}", out.display());
}
