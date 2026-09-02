//! ROADMAP F09: wasm32/wasi emit + link smoke from the shared IR.
//!
//! Not a WASI libc / preview2 host. The LLVM backend emits a wasm32/wasi
//! object from one Program IR and links a `.wasm` artifact.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_llvm::{
    compile_object_for_wasm32_wasi, emit_llvm_ir, link_wasm32_wasi, WASM32_WASI_TRIPLE,
};
use draconic_frontend::compile_source;

const WASM_MAGIC: &[u8] = b"\0asm";

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-wasm32-wasi-{}-{}-{}",
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

fn shared_ir_llvm() -> String {
    let module = compile_source("let x: i32 = 1;").expect("compile Program");
    emit_llvm_ir(&module).expect("emit_llvm_ir from shared IR")
}

#[test]
fn triple_names_wasm32_wasi() {
    assert!(
        WASM32_WASI_TRIPLE.contains("wasm32"),
        "F09 triple must be wasm32: {WASM32_WASI_TRIPLE}"
    );
    assert!(
        WASM32_WASI_TRIPLE.contains("wasi"),
        "F09 triple must be wasi: {WASM32_WASI_TRIPLE}"
    );
}

#[test]
fn llvm_emits_wasm32_wasi_object_from_shared_ir() {
    let dir = temp_dir();
    let out = dir.join("smoke.o");
    let ll = shared_ir_llvm();
    compile_object_for_wasm32_wasi(&ll, &out).expect("F09 emit wasm32/wasi object");
    let bytes = fs::read(&out).expect("read wasm32/wasi object");
    assert!(
        bytes.starts_with(WASM_MAGIC),
        "wasm32/wasi object must be a wasm module, got {} bytes",
        bytes.len()
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn link_smoke_produces_linked_wasm_artifact() {
    let dir = temp_dir();
    let out = dir.join("smoke.wasm");
    let ll = shared_ir_llvm();
    link_wasm32_wasi(&ll, &out).expect("F09 wasm32/wasi link smoke");
    assert!(out.is_file(), "linked wasm missing at {}", out.display());
    let bytes = fs::read(&out).expect("read linked wasm");
    assert!(
        bytes.starts_with(WASM_MAGIC),
        "linked artifact must be a wasm module, got {} bytes",
        bytes.len()
    );
    assert!(
        bytes.len() > WASM_MAGIC.len(),
        "linked wasm should be more than the magic prefix"
    );
    let _ = fs::remove_dir_all(&dir);
}
