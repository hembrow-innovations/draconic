//! ROADMAP D04.01: at least one non-host triple LLVM smoke.
//!
//! Parent D04 names the available linux/darwin/windows × amd64/arm64 compile
//! surface and does not require a non-host success. This row proves the LLVM
//! backend can emit for at least one triple that is not the host.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_llvm::{compile_object_for_non_host, emit_llvm_ir, host_cross_compile_pair};
use draconic_frontend::compile_source;

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-cross-compile-non-host-{}-{}-{}",
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
fn program_llvm_emit_succeeds_for_a_non_host_matrix_triple() {
    let host = host_cross_compile_pair().expect("D04.01 host should be a matrix pair");
    let module = compile_source("let x: i32 = 42;").expect("compile Program");
    let ll = emit_llvm_ir(&module).expect("emit_llvm_ir");
    let dir = temp_dir();
    let out = dir.join("non-host.o");

    let pair = compile_object_for_non_host(&ll, &out)
        .expect("D04.01 requires LLVM emit for at least one non-host matrix triple");

    assert_ne!(
        pair.pair, host.pair,
        "smoke triple {} must not be the host {}",
        pair.pair, host.pair
    );
    let bytes = fs::read(&out).unwrap_or_else(|e| {
        panic!(
            "read non-host object for {} ({}): {e}",
            pair.pair, pair.triple
        )
    });
    assert!(
        !bytes.is_empty(),
        "object for {} ({}) should be non-empty",
        pair.pair,
        pair.triple
    );

    let _ = fs::remove_dir_all(&dir);
}
