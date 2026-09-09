//! Clang/ar link of LLVM IR plus Runtime into a native binary.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use draconic_diagnostics::{codes, Diagnostic, Span};

/// Compile LLVM IR + Runtime C into a native executable via `clang`.
pub fn build_native_binary(llvm_ir: &str, out_bin: &Path) -> Result<(), Diagnostic> {
    build_native_binary_with_libs(llvm_ir, out_bin, &[], &[], false)
}

/// F04.01: same as [`build_native_binary`], plus extra `.a` archives on the link line.
pub fn build_native_binary_with_static_libs(
    llvm_ir: &str,
    out_bin: &Path,
    extra_static_libs: &[PathBuf],
) -> Result<(), Diagnostic> {
    build_native_binary_with_libs(llvm_ir, out_bin, extra_static_libs, &[], false)
}

/// F05.01: same as [`build_native_binary`], plus extra shared libraries on the link line.
pub fn build_native_binary_with_dynamic_libs(
    llvm_ir: &str,
    out_bin: &Path,
    extra_dynamic_libs: &[PathBuf],
) -> Result<(), Diagnostic> {
    build_native_binary_with_libs(llvm_ir, out_bin, &[], extra_dynamic_libs, false)
}

/// D05.02: same as [`build_native_binary_with_static_libs`], with optional LTO.
pub fn build_native_binary_with_lto(
    llvm_ir: &str,
    out_bin: &Path,
    extra_static_libs: &[PathBuf],
    lto: bool,
) -> Result<(), Diagnostic> {
    build_native_binary_with_libs(llvm_ir, out_bin, extra_static_libs, &[], lto)
}

fn build_native_binary_with_libs(
    llvm_ir: &str,
    out_bin: &Path,
    extra_static_libs: &[PathBuf],
    extra_dynamic_libs: &[PathBuf],
    lto: bool,
) -> Result<(), Diagnostic> {
    let clang = find_clang().ok_or_else(|| {
        Diagnostic::new(
            "clang not found (set CLANG or install a C toolchain)",
            Span::dummy(),
        )
    })?;

    let work = work_dir("draconic-llvm-build")?;
    let ll_path = work.join("program.ll");
    std::fs::write(&ll_path, llvm_ir)
        .map_err(|e| Diagnostic::new(format!("write LLVM IR failed: {e}"), Span::dummy()))?;

    let rt_lib = draconic_runtime::build_runtime_static_lib_with_lto(&work, lto).map_err(|e| {
        Diagnostic::new(
            format!("build runtime static lib failed: {e}"),
            Span::dummy(),
        )
    })?;

    if let Some(parent) = out_bin.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            Diagnostic::new(format!("create output dir failed: {e}"), Span::dummy())
        })?;
    }

    let want_debug = llvm_ir.contains("!llvm.dbg.cu");

    // Object step first so DWARF from IR metadata is materialized (U07). Direct
    // `clang file.ll -o bin` drops debug on Apple ld without a retained .o.
    let obj_path = work.join("program.o");
    let mut cc_obj = Command::new(&clang);
    cc_obj
        .arg("-c")
        .arg(&ll_path)
        .arg("-o")
        .arg(&obj_path)
        .arg("-Wno-override-module");
    if lto {
        cc_obj.arg("-flto").arg("-Os");
    }
    if want_debug {
        cc_obj.arg("-g");
    }
    let output = cc_obj
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Diagnostic::new(format!("spawn clang -c failed: {e}"), Span::dummy()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Diagnostic::new(
            format!("clang -c failed: {stderr}"),
            Span::dummy(),
        ));
    }

    let mut static_libs: Vec<&Path> = Vec::new();
    let mut dynamic_libs: Vec<&Path> = extra_dynamic_libs.iter().map(|p| p.as_path()).collect();
    for lib in extra_static_libs {
        if is_shared_lib(lib) {
            dynamic_libs.push(lib);
        } else {
            static_libs.push(lib);
        }
    }

    for lib in &static_libs {
        if !lib.is_file() {
            return Err(Diagnostic::new(
                format!("static lib not found: {}", lib.display()),
                Span::dummy(),
            ));
        }
    }
    for lib in &dynamic_libs {
        if !lib.is_file() {
            return Err(Diagnostic::new(
                format!("dynamic lib not found: {}", lib.display()),
                Span::dummy(),
            )
            .with_code(codes::MISSING_DYNAMIC_LIB));
        }
    }

    let mut cc_link = Command::new(&clang);
    cc_link.arg(&obj_path);
    for lib in &static_libs {
        cc_link.arg(lib);
    }
    for lib in &dynamic_libs {
        cc_link.arg(lib);
        if let Some(parent) = lib.parent() {
            let parent = if parent.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                parent.to_path_buf()
            };
            let rpath = if parent.is_absolute() {
                parent
            } else {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(&parent)
            };
            cc_link.arg(format!("-Wl,-rpath,{}", rpath.display()));
        }
    }
    cc_link.arg(&rt_lib).arg("-o").arg(out_bin);
    if lto {
        cc_link.arg("-flto").arg("-Os");
        if cfg!(target_os = "macos") {
            cc_link.arg("-Wl,-dead_strip");
        } else {
            cc_link.arg("-Wl,--gc-sections");
        }
    }
    if want_debug {
        cc_link.arg("-g");
    }
    draconic_runtime::apply_runtime_link_flags(&mut cc_link);
    let output = cc_link
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Diagnostic::new(format!("spawn clang link failed: {e}"), Span::dummy()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Diagnostic::new(
            format!("clang link failed: {stderr}"),
            Span::dummy(),
        ));
    }

    // macOS: DWARF lives in a .dSYM companion; generate it when we emitted debug.
    if want_debug && cfg!(target_os = "macos") {
        let _ = Command::new("dsymutil")
            .arg(out_bin)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    Ok(())
}

pub(crate) fn find_clang() -> Option<PathBuf> {
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

fn find_ar() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("AR") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    for candidate in [
        "ar",
        "/usr/bin/ar",
        "llvm-ar",
        "/opt/homebrew/opt/llvm/bin/llvm-ar",
    ] {
        let ok = Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or_else(|_| {
                Command::new(candidate)
                    .arg("-V")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            });
        if ok {
            return Some(PathBuf::from(candidate));
        }
        let probe = Command::new(candidate)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if probe.is_ok() {
            return Some(PathBuf::from(candidate));
        }
    }
    None
}

/// Compile one C file into a static archive (`.a`). Used to feed extra libs to
/// [`build_native_binary_with_static_libs`] (F04.01).
pub fn build_c_static_lib(c_src: &Path, archive: &Path) -> Result<(), Diagnostic> {
    let clang = find_clang().ok_or_else(|| {
        Diagnostic::new(
            "clang not found (set CLANG or install a C toolchain)",
            Span::dummy(),
        )
    })?;
    let ar = find_ar().ok_or_else(|| {
        Diagnostic::new(
            "ar not found (set AR or install a C toolchain)",
            Span::dummy(),
        )
    })?;
    if !c_src.is_file() {
        return Err(Diagnostic::new(
            format!("C source not found: {}", c_src.display()),
            Span::dummy(),
        ));
    }
    if let Some(parent) = archive.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Diagnostic::new(format!("create archive dir failed: {e}"), Span::dummy())
            })?;
        }
    }
    let work = work_dir("draconic-c-static")?;
    let obj = work.join("lib.o");
    let compile = Command::new(&clang)
        .arg("-c")
        .arg(c_src)
        .arg("-o")
        .arg(&obj)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Diagnostic::new(format!("spawn clang -c failed: {e}"), Span::dummy()))?;
    if !compile.status.success() {
        let stderr = String::from_utf8_lossy(&compile.stderr);
        return Err(Diagnostic::new(
            format!("clang -c {} failed: {stderr}", c_src.display()),
            Span::dummy(),
        ));
    }
    let archive_out = Command::new(&ar)
        .arg("rcs")
        .arg(archive)
        .arg(&obj)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Diagnostic::new(format!("spawn ar failed: {e}"), Span::dummy()))?;
    if !archive_out.status.success() {
        let stderr = String::from_utf8_lossy(&archive_out.stderr);
        return Err(Diagnostic::new(
            format!("ar rcs failed: {stderr}"),
            Span::dummy(),
        ));
    }
    if !archive.is_file() {
        return Err(Diagnostic::new(
            format!("static lib missing after ar: {}", archive.display()),
            Span::dummy(),
        ));
    }
    Ok(())
}

fn is_shared_lib(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("so") | Some("dylib") | Some("dll")
    )
}

/// Host shared-library file name (`libfoo.dylib` / `libfoo.so` / `foo.dll`).
pub fn dynamic_lib_file_name(stem: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{stem}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{stem}.dylib")
    } else {
        format!("lib{stem}.so")
    }
}

/// F05.01: compile one C file into a shared library (`.so` / `.dylib` / `.dll`).
pub fn build_c_dynamic_lib(c_src: &Path, dylib: &Path) -> Result<(), Diagnostic> {
    let clang = find_clang().ok_or_else(|| {
        Diagnostic::new(
            "clang not found (set CLANG or install a C toolchain)",
            Span::dummy(),
        )
    })?;
    if !c_src.is_file() {
        return Err(Diagnostic::new(
            format!("C source not found: {}", c_src.display()),
            Span::dummy(),
        ));
    }
    if let Some(parent) = dylib.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Diagnostic::new(format!("create dylib dir failed: {e}"), Span::dummy())
            })?;
        }
    }
    let abs_dylib = if dylib.is_absolute() {
        dylib.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| Diagnostic::new(format!("cwd failed: {e}"), Span::dummy()))?
            .join(dylib)
    };
    let mut compile = Command::new(&clang);
    compile
        .arg("-shared")
        .arg("-fPIC")
        .arg("-o")
        .arg(dylib)
        .arg(c_src);
    if cfg!(target_os = "macos") {
        compile.arg("-install_name").arg(&abs_dylib);
    }
    let compile = compile
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Diagnostic::new(format!("spawn clang -shared failed: {e}"), Span::dummy()))?;
    if !compile.status.success() {
        let stderr = String::from_utf8_lossy(&compile.stderr);
        return Err(Diagnostic::new(
            format!("clang -shared {} failed: {stderr}", c_src.display()),
            Span::dummy(),
        ));
    }
    if !dylib.is_file() {
        return Err(Diagnostic::new(
            format!("dynamic lib missing after clang: {}", dylib.display()),
            Span::dummy(),
        ));
    }
    Ok(())
}

pub(crate) fn work_dir(prefix: &str) -> Result<PathBuf, Diagnostic> {
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
    use std::process::Command;

    use draconic_frontend::compile_source;
    use draconic_ir::Module;

    use crate::emit_llvm_ir;

    fn module_of(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    /// F04.01: extra `.a` on the link line resolves a C symbol not in Runtime/libc.
    #[test]
    fn native_link_static_lib_resolves_c_symbol() {
        let dir = work_dir("draconic-llvm-f04-01-link-static").expect("workdir");
        let c_src = dir.join("touch.c");
        std::fs::write(&c_src, "void draconic_link_static_touch(void) {}\n").expect("write c");
        let archive = dir.join("libtouch.a");
        build_c_static_lib(&c_src, &archive).expect("build .a");

        let m = module_of(
            r#"
            extern "C" function draconic_link_static_touch(): void;
            draconic_link_static_touch();
            let x: i32 = 1;
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            ir.contains("declare void @draconic_link_static_touch()"),
            "expected declare:\n{ir}"
        );
        assert!(
            ir.contains("call void @draconic_link_static_touch()"),
            "expected call:\n{ir}"
        );

        let missing = dir.join("no_lib");
        let err = build_native_binary(&ir, &missing).expect_err("link without .a must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("draconic_link_static_touch")
                || msg.contains("undefined")
                || msg.contains("Unresolved"),
            "expected unresolved symbol, got {msg}"
        );

        let bin = dir.join("linked");
        build_native_binary_with_static_libs(&ir, &bin, &[archive]).expect("link with .a");
        assert!(bin.is_file(), "native binary missing at {}", bin.display());
    }

    /// F04.02: call a linked static symbol; native stdout is the C return value.
    #[test]
    fn native_link_static_lib_call_end_to_end() {
        let dir = work_dir("draconic-llvm-f04-02-link-static-call").expect("workdir");
        let c_src = dir.join("add.c");
        std::fs::write(
            &c_src,
            "int draconic_link_static_add(int a, int b) { return a + b; }\n",
        )
        .expect("write c");
        let archive = dir.join("libadd.a");
        build_c_static_lib(&c_src, &archive).expect("build .a");

        let m = module_of(
            r#"
            extern "C" function draconic_link_static_add(a: i32, b: i32): i32;
            let s: i32 = draconic_link_static_add(20, 22);
            let t: i32 = draconic_link_static_add(-5, 12);
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            ir.contains("declare i32 @draconic_link_static_add(i32, i32)"),
            "expected declare:\n{ir}"
        );
        assert!(
            ir.contains("call i32 @draconic_link_static_add"),
            "expected call:\n{ir}"
        );

        let bin = dir.join("linked");
        build_native_binary_with_static_libs(&ir, &bin, &[archive]).expect("link with .a");
        let output = Command::new(&bin).output().expect("run");
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

    /// F05.01: extra shared lib on the link line resolves a C symbol not in Runtime/libc.
    #[test]
    fn native_link_dynamic_lib_resolves_c_symbol() {
        let dir = work_dir("draconic-llvm-f05-01-link-dynamic").expect("workdir");
        let c_src = dir.join("touch.c");
        std::fs::write(&c_src, "void draconic_link_dynamic_touch(void) {}\n").expect("write c");
        let dylib = dir.join(dynamic_lib_file_name("touch"));
        build_c_dynamic_lib(&c_src, &dylib).expect("build shared lib");

        let m = module_of(
            r#"
            extern "C" function draconic_link_dynamic_touch(): void;
            draconic_link_dynamic_touch();
            let x: i32 = 1;
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            ir.contains("declare void @draconic_link_dynamic_touch()"),
            "expected declare:\n{ir}"
        );
        assert!(
            ir.contains("call void @draconic_link_dynamic_touch()"),
            "expected call:\n{ir}"
        );

        let missing = dir.join("no_lib");
        let err = build_native_binary(&ir, &missing).expect_err("link without dylib must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("draconic_link_dynamic_touch")
                || msg.contains("undefined")
                || msg.contains("Unresolved"),
            "expected unresolved symbol, got {msg}"
        );

        let bin = dir.join("linked");
        build_native_binary_with_dynamic_libs(&ir, &bin, &[dylib]).expect("link with shared lib");
        assert!(bin.is_file(), "native binary missing at {}", bin.display());
        let output = Command::new(&bin).output().expect("run");
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

    /// F05.02: call a linked dynamic symbol; native stdout is the C return value.
    #[test]
    fn native_link_dynamic_lib_call_end_to_end() {
        let dir = work_dir("draconic-llvm-f05-02-link-dynamic-call").expect("workdir");
        let c_src = dir.join("add.c");
        std::fs::write(
            &c_src,
            "int draconic_link_dynamic_add(int a, int b) { return a + b; }\n",
        )
        .expect("write c");
        let dylib = dir.join(dynamic_lib_file_name("add"));
        build_c_dynamic_lib(&c_src, &dylib).expect("build shared lib");

        let m = module_of(
            r#"
            extern "C" function draconic_link_dynamic_add(a: i32, b: i32): i32;
            let s: i32 = draconic_link_dynamic_add(20, 22);
            let t: i32 = draconic_link_dynamic_add(-5, 12);
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            ir.contains("declare i32 @draconic_link_dynamic_add(i32, i32)"),
            "expected declare:\n{ir}"
        );
        assert!(
            ir.contains("call i32 @draconic_link_dynamic_add"),
            "expected call:\n{ir}"
        );

        let bin = dir.join("linked");
        build_native_binary_with_dynamic_libs(&ir, &bin, &[dylib]).expect("link with shared lib");
        let output = Command::new(&bin).output().expect("run");
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

    /// F05.02: missing shared lib is a typed diagnostic (E0402), not a raw linker dump.
    #[test]
    fn native_link_dynamic_lib_missing_is_typed_error() {
        let dir = work_dir("draconic-llvm-f05-02-missing-dylib").expect("workdir");
        let m = module_of(
            r#"
            extern "C" function draconic_link_dynamic_add(a: i32, b: i32): i32;
            let s: i32 = draconic_link_dynamic_add(20, 22);
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        let missing = dir.join(dynamic_lib_file_name("no_such"));
        assert!(!missing.is_file(), "fixture path must not exist");
        let bin = dir.join("no_bin");
        let err = build_native_binary_with_dynamic_libs(&ir, &bin, &[missing.clone()])
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
}
