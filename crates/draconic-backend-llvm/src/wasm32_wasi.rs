//! ROADMAP F09: wasm32/wasi emit + link smoke from the shared IR.
//!
//! One LLVM backend, no WASM-only IR fork (ADR-0002). Object emit uses a
//! wasm-capable clang; the link smoke produces a `.wasm` artifact without a
//! WASI libc / preview2 host.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use draconic_diagnostics::{Diagnostic, Span};

/// LLVM triple for F09 wasm32/wasi emit (WASI preview 1).
pub const WASM32_WASI_TRIPLE: &str = "wasm32-wasip1";

const TRIPLE_FALLBACK: &str = "wasm32-wasi";
const WASM_MAGIC: &[u8] = b"\0asm";

struct WasmTools {
    clang: PathBuf,
    triple: &'static str,
    wasm_ld: PathBuf,
}

/// Compile LLVM IR (from the shared IR) to a wasm32/wasi object.
pub fn compile_object_for_wasm32_wasi(llvm_ir: &str, out_obj: &Path) -> Result<(), Diagnostic> {
    let tools = wasm_tools()?;
    compile_object_with(tools, llvm_ir, out_obj)
}

/// Link LLVM IR emitted from the shared IR into a `.wasm` artifact.
///
/// Smoke only: undefined Runtime symbols stay unresolved (`--allow-undefined`).
/// Not a WASI libc or preview2 host.
pub fn link_wasm32_wasi(llvm_ir: &str, out_wasm: &Path) -> Result<(), Diagnostic> {
    let tools = wasm_tools()?;
    let work = work_dir("draconic-wasm32-wasi-link")?;
    let obj = work.join("program.o");
    compile_object_with(tools, llvm_ir, &obj)?;

    if let Some(parent) = out_wasm.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            Diagnostic::new(format!("create output dir failed: {e}"), Span::dummy())
        })?;
    }

    let output = Command::new(&tools.wasm_ld)
        .arg("--no-entry")
        .arg("--export-all")
        .arg("--allow-undefined")
        .arg(&obj)
        .arg("-o")
        .arg(out_wasm)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Diagnostic::new(format!("spawn wasm-ld failed: {e}"), Span::dummy()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Diagnostic::new(
            format!("wasm32/wasi link failed: {stderr}"),
            Span::dummy(),
        ));
    }
    let bytes = std::fs::read(out_wasm)
        .map_err(|e| Diagnostic::new(format!("read linked wasm failed: {e}"), Span::dummy()))?;
    if !bytes.starts_with(WASM_MAGIC) {
        return Err(Diagnostic::new(
            "wasm32/wasi link did not produce a wasm module",
            Span::dummy(),
        ));
    }
    Ok(())
}

fn compile_object_with(tools: &WasmTools, llvm_ir: &str, out_obj: &Path) -> Result<(), Diagnostic> {
    if let Some(parent) = out_obj.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            Diagnostic::new(format!("create output dir failed: {e}"), Span::dummy())
        })?;
    }

    let ll_path = match out_obj.file_stem() {
        Some(stem) => {
            let mut ll = out_obj.to_path_buf();
            ll.set_file_name(format!("{}.ll", stem.to_string_lossy()));
            ll
        }
        None => out_obj.with_extension("ll"),
    };
    std::fs::write(&ll_path, llvm_ir)
        .map_err(|e| Diagnostic::new(format!("write LLVM IR failed: {e}"), Span::dummy()))?;

    let output = Command::new(&tools.clang)
        .arg("-c")
        .arg(&ll_path)
        .arg("-o")
        .arg(out_obj)
        .arg("-Wno-override-module")
        .arg("-target")
        .arg(tools.triple)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| {
            Diagnostic::new(
                format!("spawn clang -c -target {} failed: {e}", tools.triple),
                Span::dummy(),
            )
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Diagnostic::new(
            format!("clang -c -target {} not available: {stderr}", tools.triple),
            Span::dummy(),
        ));
    }
    let bytes = std::fs::read(out_obj).map_err(|e| {
        Diagnostic::new(
            format!("read wasm32/wasi object failed: {e}"),
            Span::dummy(),
        )
    })?;
    if !bytes.starts_with(WASM_MAGIC) {
        return Err(Diagnostic::new(
            format!("clang -target {} did not emit a wasm object", tools.triple),
            Span::dummy(),
        ));
    }
    Ok(())
}

fn wasm_tools() -> Result<&'static WasmTools, Diagnostic> {
    static TOOLS: OnceLock<Result<WasmTools, String>> = OnceLock::new();
    match TOOLS.get_or_init(discover_wasm_tools) {
        Ok(tools) => Ok(tools),
        Err(msg) => Err(Diagnostic::new(msg.clone(), Span::dummy())),
    }
}

fn discover_wasm_tools() -> Result<WasmTools, String> {
    let (clang, triple) = find_wasm_clang().ok_or_else(|| {
        "clang with wasm32/wasi target not found (set CLANG to a wasm-capable clang, e.g. Homebrew LLVM)".to_string()
    })?;
    let wasm_ld = find_wasm_ld(&clang).ok_or_else(|| {
        "wasm-ld not found (set WASM_LD or install lld / a Rust sysroot with wasm-ld)".to_string()
    })?;
    Ok(WasmTools {
        clang,
        triple,
        wasm_ld,
    })
}

fn find_wasm_clang() -> Option<(PathBuf, &'static str)> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("CLANG") {
        let p = PathBuf::from(p);
        if p.is_file() {
            candidates.push(p);
        }
    }
    for candidate in [
        "/opt/homebrew/opt/llvm@22/bin/clang",
        "/opt/homebrew/opt/llvm/bin/clang",
        "clang",
        "/usr/bin/clang",
    ] {
        candidates.push(PathBuf::from(candidate));
    }
    if let Some(found) = crate::find_clang() {
        candidates.push(found);
    }
    for clang in candidates {
        if let Some(triple) = clang_wasm_triple(&clang) {
            return Some((clang, triple));
        }
    }
    None
}

fn clang_wasm_triple(clang: &Path) -> Option<&'static str> {
    [WASM32_WASI_TRIPLE, TRIPLE_FALLBACK]
        .into_iter()
        .find(|&triple| clang_emits_wasm(clang, triple))
        .map(|v| v as _)
}

fn clang_emits_wasm(clang: &Path, triple: &str) -> bool {
    let Ok(dir) = work_dir("draconic-wasm32-wasi-probe") else {
        return false;
    };
    let ll = dir.join("probe.ll");
    let obj = dir.join("probe.o");
    if std::fs::write(&ll, "define i32 @main() {\n  ret i32 0\n}\n").is_err() {
        let _ = std::fs::remove_dir_all(&dir);
        return false;
    }
    let ok = Command::new(clang)
        .arg("-c")
        .arg(&ll)
        .arg("-o")
        .arg(&obj)
        .arg("-Wno-override-module")
        .arg("-target")
        .arg(triple)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let wasm = ok
        && std::fs::read(&obj)
            .map(|b| b.starts_with(WASM_MAGIC))
            .unwrap_or(false);
    let _ = std::fs::remove_dir_all(&dir);
    wasm
}

fn find_wasm_ld(clang: &Path) -> Option<PathBuf> {
    if let Ok(p) = std::env::var("WASM_LD") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Some(parent) = clang.parent() {
        let next_to_clang = parent.join("wasm-ld");
        if next_to_clang.is_file() {
            return Some(next_to_clang);
        }
    }
    for candidate in ["wasm-ld", "/opt/homebrew/opt/lld/bin/wasm-ld"] {
        if command_ok(candidate) {
            return Some(PathBuf::from(candidate));
        }
    }
    rustup_wasm_ld()
}

fn command_ok(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn rustup_wasm_ld() -> Option<PathBuf> {
    let output = Command::new("rustc")
        .arg("--print")
        .arg("sysroot")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sysroot = String::from_utf8_lossy(&output.stdout);
    let rustlib = Path::new(sysroot.trim()).join("lib/rustlib");
    let entries = std::fs::read_dir(rustlib).ok()?;
    for entry in entries.flatten() {
        let candidate = entry.path().join("bin").join("gcc-ld").join("wasm-ld");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn work_dir(prefix: &str) -> Result<PathBuf, Diagnostic> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "{prefix}-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir)
        .map_err(|e| Diagnostic::new(format!("temp dir failed: {e}"), Span::dummy()))?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emit_llvm_ir;
    use draconic_frontend::compile_source;

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
    fn emits_wasm32_wasi_object_from_shared_ir() {
        let dir = work_dir("draconic-wasm32-wasi-unit-obj").unwrap();
        let out = dir.join("smoke.o");
        compile_object_for_wasm32_wasi(&shared_ir_llvm(), &out)
            .expect("F09 emit wasm32/wasi object");
        let bytes = std::fs::read(&out).unwrap();
        assert!(
            bytes.starts_with(WASM_MAGIC),
            "wasm32/wasi object must be a wasm module"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_smoke_produces_linked_wasm_artifact() {
        let dir = work_dir("draconic-wasm32-wasi-unit-link").unwrap();
        let out = dir.join("smoke.wasm");
        link_wasm32_wasi(&shared_ir_llvm(), &out).expect("F09 wasm32/wasi link smoke");
        let bytes = std::fs::read(&out).unwrap();
        assert!(
            bytes.starts_with(WASM_MAGIC),
            "linked artifact must be a wasm module"
        );
        assert!(bytes.len() > WASM_MAGIC.len());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
