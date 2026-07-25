//! LLVM backend: IR → native (ROADMAP B08 stub).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::Module;

/// Emit LLVM IR text for a shared IR module.
///
/// B08 stub: ignores module body and emits `@main` that calls Runtime `draconic_rt_hello`.
pub fn emit_llvm_ir(_module: &Module) -> Result<String, Diagnostic> {
    Ok(concat!(
        "; Draconic LLVM backend stub (B08)\n",
        "declare void @draconic_rt_hello()\n",
        "\n",
        "define i32 @main() {\n",
        "entry:\n",
        "  call void @draconic_rt_hello()\n",
        "  ret i32 0\n",
        "}\n",
    )
    .to_string())
}

/// Compile LLVM IR + Runtime C into a native executable via `clang`.
pub fn build_native_binary(llvm_ir: &str, out_bin: &Path) -> Result<(), Diagnostic> {
    let clang = find_clang().ok_or_else(|| {
        Diagnostic::new(
            "clang not found (set CLANG or install a C toolchain)",
            Span::dummy(),
        )
    })?;

    let work = work_dir("draconic-llvm-build")?;
    let ll_path = work.join("program.ll");
    std::fs::write(&ll_path, llvm_ir).map_err(|e| {
        Diagnostic::new(format!("write LLVM IR failed: {e}"), Span::dummy())
    })?;

    let rt_c = draconic_runtime::c_runtime_path();
    if !rt_c.is_file() {
        return Err(Diagnostic::new(
            format!("runtime C source missing: {}", rt_c.display()),
            Span::dummy(),
        ));
    }

    if let Some(parent) = out_bin.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            Diagnostic::new(format!("create output dir failed: {e}"), Span::dummy())
        })?;
    }

    let output = Command::new(&clang)
        .arg(&ll_path)
        .arg(&rt_c)
        .arg("-o")
        .arg(out_bin)
        .arg("-Wno-override-module")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Diagnostic::new(format!("spawn clang failed: {e}"), Span::dummy()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Diagnostic::new(
            format!("clang failed: {stderr}"),
            Span::dummy(),
        ));
    }
    Ok(())
}

fn find_clang() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CLANG") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    for candidate in [
        "clang",
        "/usr/bin/clang",
        "/opt/homebrew/opt/llvm@22/bin/clang",
        "/opt/homebrew/opt/llvm/bin/clang",
    ] {
        let ok = Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return Some(PathBuf::from(candidate));
        }
    }
    None
}

fn work_dir(prefix: &str) -> Result<PathBuf, Diagnostic> {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "{prefix}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).map_err(|e| {
        Diagnostic::new(format!("temp dir failed: {e}"), Span::dummy())
    })?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_check::check;
    use draconic_ir::lower;
    use draconic_parser::parse;

    fn empty_module() -> Module {
        let program = parse("").expect("parse");
        let checked = check(program).expect("check");
        lower(&checked)
    }

    #[test]
    fn emit_stub_calls_runtime_hello() {
        let ir = emit_llvm_ir(&empty_module()).expect("emit");
        assert!(
            ir.contains("draconic_rt_hello"),
            "IR must declare/call runtime hello:\n{ir}"
        );
        assert!(ir.contains("define i32 @main"), "IR must define main:\n{ir}");
        assert!(
            ir.contains("call void @draconic_rt_hello"),
            "main must call hello:\n{ir}"
        );
        assert!(ir.contains("ret i32 0"), "main must return 0:\n{ir}");
    }

    #[test]
    fn native_binary_prints_hello() {
        let ir = emit_llvm_ir(&empty_module()).expect("emit");
        let dir = work_dir("draconic-llvm-test").expect("workdir");
        let bin = dir.join("hello");
        build_native_binary(&ir, &bin).expect("build_native_binary");

        let output = Command::new(&bin).output().expect("run binary");
        assert!(
            output.status.success(),
            "binary exit {:?}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "hello\n", "stdout={stdout:?}");
    }

    #[test]
    fn emit_accepts_nonempty_module() {
        let program = parse("let x = 1;").expect("parse");
        let checked = check(program).expect("check");
        let module = lower(&checked);
        let ir = emit_llvm_ir(&module).expect("emit");
        assert!(ir.contains("@main"));
    }
}
