//! ROADMAP U07: native DWARF debug info maps Draconic source lines.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_llvm::{build_native_binary, emit_llvm_ir_with_debug, SourceDebug};
use draconic_frontend::compile_source;

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-native-debug-{}-{}-{}",
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

fn find_dwarfdump() -> Option<PathBuf> {
    for candidate in [
        "dwarfdump",
        "/usr/bin/dwarfdump",
        "/Library/Developer/CommandLineTools/usr/bin/dwarfdump",
    ] {
        let ok = Command::new(candidate)
            .arg("--help")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return Some(PathBuf::from(candidate));
        }
    }
    // macOS xcrun fallback
    let out = Command::new("xcrun")
        .args(["--find", "dwarfdump"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if out.status.success() {
        let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !p.is_empty() && Path::new(&p).is_file() {
            return Some(PathBuf::from(p));
        }
    }
    None
}

#[test]
fn native_binary_dwarf_maps_draconic_source_lines() {
    let dir = temp_dir();
    let src_path = dir.join("sample.drac");
    // Multi-line so line 2 is a real statement line.
    let source = "let a: i32 = 1;\nlet b: i32 = 2;\n";
    fs::write(&src_path, source).unwrap();

    let module = compile_source(source).expect("compile");
    let debug = SourceDebug::from_path(&src_path, source);
    let ll = emit_llvm_ir_with_debug(&module, &debug).expect("emit with debug");

    assert!(
        ll.contains("!llvm.dbg.cu"),
        "IR must include compile unit:\n{ll}"
    );
    assert!(
        ll.contains("sample.drac"),
        "IR DIFile must name source file:\n{ll}"
    );
    assert!(
        ll.contains("DILocation(line: 1,") || ll.contains("DILocation(line: 2,"),
        "IR must map at least one Draconic source line:\n{ll}"
    );
    assert!(
        ll.contains("define i32 @main() !dbg !"),
        "main must carry !dbg:\n{ll}"
    );

    let bin = dir.join("sample");
    build_native_binary(&ll, &bin).expect("build_native_binary");

    let Some(dwarfdump) = find_dwarfdump() else {
        // Toolchain without dwarfdump: IR-level asserts above still pin U07 emit.
        eprintln!("dwarfdump not found; skipping binary DWARF inspection");
        return;
    };

    // macOS keeps DWARF in .dSYM; prefer that when present.
    let dump_target = {
        let dsym = PathBuf::from(format!("{}.dSYM", bin.display()));
        if dsym.is_dir() {
            dsym
        } else {
            bin.clone()
        }
    };

    let dump = Command::new(&dwarfdump)
        .arg(&dump_target)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run dwarfdump");
    let stdout = String::from_utf8_lossy(&dump.stdout);
    let stderr = String::from_utf8_lossy(&dump.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(
        dump.status.success() || combined.contains("sample"),
        "dwarfdump failed: status={:?} stderr={stderr}",
        dump.status
    );
    assert!(
        combined.contains("sample.drac"),
        "DWARF must reference Draconic source file; dump:\n{combined}"
    );
    assert!(
        combined.contains("DW_AT_decl_line") || combined.contains("decl_line"),
        "DWARF must include source line mapping; dump:\n{combined}"
    );
}

#[test]
fn llvm_ir_debug_markers_cover_both_lines() {
    let source = "let a: i32 = 10;\nlet b: i32 = 20;\n";
    let module = compile_source(source).expect("compile");
    let debug = SourceDebug {
        path: "two_lines.drac".into(),
        source: source.into(),
    };
    let ll = emit_llvm_ir_with_debug(&module, &debug).expect("emit");
    assert!(ll.contains("DILocation(line: 1,"), "line 1 missing:\n{ll}");
    assert!(ll.contains("DILocation(line: 2,"), "line 2 missing:\n{ll}");
}
