//! ROADMAP R06: native panic/abort reports Draconic source locations via U07 DWARF.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_llvm::{build_native_binary, emit_llvm_ir_with_debug, SourceDebug};
use draconic_frontend::compile_source;

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-panic-backtrace-{}-{}-{}",
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
fn native_abort_backtrace_names_draconic_source_location() {
    let dir = temp_dir();
    let src_path = dir.join("panic.drac");
    // Line 2 is the abort-class call the backtrace must name.
    let source = "extern \"C\" function draconic_rt_abort(): void;\ndraconic_rt_abort();\n";
    fs::write(&src_path, source).unwrap();

    let module = compile_source(source).expect("compile");
    let debug = SourceDebug::from_path(&src_path, source);
    let ll = emit_llvm_ir_with_debug(&module, &debug).expect("emit with debug");
    assert!(
        ll.contains("!llvm.dbg.cu"),
        "IR must include U07 compile unit:\n{ll}"
    );
    assert!(
        ll.contains("panic.drac"),
        "IR DIFile must name Draconic source:\n{ll}"
    );

    let bin = dir.join("panic");
    build_native_binary(&ll, &bin).expect("build_native_binary");

    let output = Command::new(&bin).output().expect("run aborting program");
    assert!(
        !output.status.success(),
        "native abort must kill the process: {:?}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("draconic_rt: abort"),
        "canonical abort message missing; stderr={stderr}"
    );
    assert!(
        stderr.contains("draconic_rt: backtrace"),
        "abort must emit a backtrace; stderr={stderr}"
    );
    assert!(
        stderr.contains("panic.drac:2"),
        "backtrace must name Draconic source location via U07 DWARF; stderr={stderr}"
    );
}
