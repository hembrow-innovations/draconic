//! LLVM backend: IR → native (ROADMAP B08 stub + N01–N03.02 native scalars/layouts).

mod native_ints;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::Module;

use native_ints::{emit_native_ints, is_native_int_module};

/// Emit LLVM IR text for a shared IR module.
///
/// Programs that use only native scalar types (`i8`–`i64`, `u8`–`u64`, `f32`/
/// `f64`, `bool`) and/or native layout structs (shapes of native scalar fields)
/// with a supported statement/expression subset are lowered for real. Everything
/// else keeps the B08 hello stub so existing ES conformance fixtures stay green.
pub fn emit_llvm_ir(module: &Module) -> Result<String, Diagnostic> {
    if is_native_int_module(module) {
        emit_native_ints(module)
    } else {
        Ok(emit_hello_stub())
    }
}

fn emit_hello_stub() -> String {
    concat!(
        "; Draconic LLVM backend stub (B08)\n",
        "declare void @draconic_rt_hello()\n",
        "\n",
        "define i32 @main() {\n",
        "entry:\n",
        "  call void @draconic_rt_hello()\n",
        "  ret i32 0\n",
        "}\n",
    )
    .to_string()
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

    fn module_of(src: &str) -> Module {
        let program = parse(src).expect("parse");
        let checked = check(program).expect("check");
        lower(&checked)
    }

    #[test]
    fn emit_stub_calls_runtime_hello() {
        let ir = emit_llvm_ir(&module_of("")).expect("emit");
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
        let ir = emit_llvm_ir(&module_of("")).expect("emit");
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
    fn emit_accepts_nonempty_js_module_as_stub() {
        let ir = emit_llvm_ir(&module_of("let x = 1;")).expect("emit");
        assert!(ir.contains("@main"));
        assert!(ir.contains("draconic_rt_hello"));
    }

    #[test]
    fn native_ints_add_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let a: i32 = 10;
            let b: i32 = 3;
            let sum: i32 = a + b;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "native int program should not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_i64"),
            "should print ints:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n01").expect("workdir");
        let bin = dir.join("ints");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "10\n3\n13\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn native_ints_function_call() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            function add(x: i32, y: i32): i32 {
              return x + y;
            }
            let s: i32 = add(20, 22);
            "#,
        ))
        .expect("emit");
        let dir = work_dir("draconic-llvm-n01-fn").expect("workdir");
        let bin = dir.join("fn");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "42\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn native_ints_wrapping_i8() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let a: i8 = 120;
            let b: i8 = a + 10;
            "#,
        ))
        .expect("emit");
        let dir = work_dir("draconic-llvm-n01-wrap").expect("workdir");
        let bin = dir.join("wrap");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        // 120 + 10 = 130 → i8 wrap → -126
        assert_eq!(stdout, "120\n-126\n", "stdout={stdout:?}");
    }

    #[test]
    fn native_floats_add_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let a: f64 = 10.5;
            let b: f64 = 2.0;
            let sum: f64 = a + b;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "native float program should not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print floats:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n02").expect("workdir");
        let bin = dir.join("floats");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "10.5\n2\n12.5\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn native_bool_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let t: bool = true;
            let f: bool = false;
            "#,
        ))
        .expect("emit");
        let dir = work_dir("draconic-llvm-n02-bool").expect("workdir");
        let bin = dir.join("bool");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "true\nfalse\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn native_struct_field_read_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            type Point = { x: i32; y: i32 };
            let p: Point = { x: 10, y: 20 };
            let a: i32 = p.x;
            let b: i32 = p.y;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "native struct program should not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("getelementptr"),
            "should GEP struct fields:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n03-struct").expect("workdir");
        let bin = dir.join("struct");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "10\n20\n10\n20\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn native_fixed_array_index_read_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            type Vec3 = [i32, i32, i32];
            let v: Vec3 = [10, 20, 30];
            let a: i32 = v[0];
            let b: i32 = v[1];
            let c: i32 = v[2];
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "native fixed-array program should not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("getelementptr"),
            "should GEP array elements:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n03-array").expect("workdir");
        let bin = dir.join("array");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "10\n20\n30\n10\n20\n30\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }
}
