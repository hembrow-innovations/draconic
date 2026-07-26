//! LLVM backend: IR → native (ROADMAP B08 stub + N01–N03 native + N06.03–N06.07 Promise).

mod es_promise;
mod native_ints;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::Module;

use es_promise::{emit_es_promise, is_es_promise_module};
use native_ints::{emit_native_ints, is_native_int_module};

/// Emit LLVM IR text for a shared IR module.
///
/// Programs that use only native scalar types (`i8`–`i64`, `u8`–`u64`, `f32`/
/// `f64`, `bool`) and/or native layout structs (shapes of native scalar fields)
/// with a supported statement/expression subset are lowered for real. Promise
/// constructor basics (N06.03) and statics/catch (N06.04) lower via the Runtime
/// Promise ABI. Everything else keeps the B08 hello stub so existing ES
/// conformance fixtures stay green.
pub fn emit_llvm_ir(module: &Module) -> Result<String, Diagnostic> {
    if is_native_int_module(module) {
        emit_native_ints(module)
    } else if is_es_promise_module(module) {
        emit_es_promise(module)
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

    let rt_lib = draconic_runtime::build_runtime_static_lib(&work).map_err(|e| {
        Diagnostic::new(format!("build runtime static lib failed: {e}"), Span::dummy())
    })?;

    if let Some(parent) = out_bin.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            Diagnostic::new(format!("create output dir failed: {e}"), Span::dummy())
        })?;
    }

    let output = Command::new(&clang)
        .arg(&ll_path)
        .arg(&rt_lib)
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
    fn es_promise_basics_prints_after_drain() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let tf = typeof Promise;
            let resolved = 0;
            let rejected = 0;
            let chained = 0;
            let p = new Promise(function (resolve) {
              resolve(42);
            });
            p.then(function (v) {
              resolved = v;
            });
            let q = new Promise(function (_resolve, reject) {
              reject(7);
            });
            q.then(
              function () {
                rejected = -1;
              },
              function (e) {
                rejected = e;
              }
            );
            new Promise(function (resolve) {
              resolve(1);
            }).then(function (v) {
              return v + 1;
            }).then(function (v) {
              chained = v;
            });
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "Promise basics must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_construct"),
            "should construct via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_then"),
            "should then via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_job_drain"),
            "should drain jobs before observe:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n06-promise").expect("workdir");
        let bin = dir.join("promise");
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
            stdout, "function\n42\n7\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_promise_resolve_reject_catch_prints_after_drain() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let tResolve = typeof Promise.resolve;
            let tReject = typeof Promise.reject;
            let resolved = 0;
            let rejected = 0;
            let caught = 0;
            let p = Promise.resolve(42);
            p.then(function (v) {
              resolved = v;
            });
            let q = Promise.reject(7);
            q.then(
              function () {
                rejected = -1;
              },
              function (e) {
                rejected = e;
              }
            );
            let r = Promise.reject(9);
            r.catch(function (e) {
              caught = e;
            });
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "Promise resolve/reject must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_new"),
            "should allocate via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_resolve"),
            "should resolve via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_reject"),
            "should reject via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_then"),
            "should then/catch via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_job_drain"),
            "should drain jobs before observe:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n06-promise-rr").expect("workdir");
        let bin = dir.join("promise_rr");
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
            stdout, "function\nfunction\n42\n7\n9\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_promise_finally_prints_after_drain() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let tFinally = typeof Promise.resolve(1).finally;
            let fulfilledSide = 0;
            let rejectedSide = 0;
            let resolved = 0;
            let caught = 0;
            let p = Promise.resolve(42);
            p.finally(function () {
              fulfilledSide = 1;
            }).then(function (v) {
              resolved = v;
            });
            let q = Promise.reject(7);
            q.finally(function () {
              rejectedSide = 1;
            }).catch(function (e) {
              caught = e;
            });
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "Promise finally must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_finally"),
            "should finally via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_then"),
            "should then/catch via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_job_drain"),
            "should drain jobs before observe:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n06-promise-finally").expect("workdir");
        let bin = dir.join("promise_finally");
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
            stdout, "function\n1\n1\n42\n7\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_promise_all_prints_after_drain() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let tAll = typeof Promise.all;
            let emptyLen = -1;
            let allLen = -1;
            let a0 = -1;
            let a1 = -1;
            let mixed0 = -1;
            let mixed1 = -1;
            let rejected = 0;
            Promise.all([]).then(function (v) {
              emptyLen = v.length;
            });
            Promise.all([Promise.resolve(10), Promise.resolve(20)]).then(function (v) {
              allLen = v.length;
              a0 = v[0];
              a1 = v[1];
            });
            Promise.all([1, Promise.resolve(2)]).then(function (v) {
              mixed0 = v[0];
              mixed1 = v[1];
            });
            Promise.all([Promise.resolve(1), Promise.reject(7)]).then(
              function () {
                rejected = -1;
              },
              function (e) {
                rejected = e;
              }
            );
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "Promise.all must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_all"),
            "should Promise.all via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_new"),
            "should allocate arrays via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_job_drain"),
            "should drain jobs before observe:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n06-promise-all").expect("workdir");
        let bin = dir.join("promise_all");
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
            stdout, "function\n0\n2\n10\n20\n1\n2\n7\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_promise_race_prints_after_drain() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let tRace = typeof Promise.race;
            let winner = -1;
            let mixed = -1;
            let rejected = 0;
            Promise.race([Promise.resolve(10), Promise.resolve(20)]).then(function (v) {
              winner = v;
            });
            Promise.race([1, Promise.resolve(2)]).then(function (v) {
              mixed = v;
            });
            Promise.race([Promise.reject(7), Promise.resolve(1)]).then(
              function () {
                rejected = -1;
              },
              function (e) {
                rejected = e;
              }
            );
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "Promise.race must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_race"),
            "should Promise.race via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_job_drain"),
            "should drain jobs before observe:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n06-promise-race").expect("workdir");
        let bin = dir.join("promise_race");
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
            stdout, "function\n10\n1\n7\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_promise_all_settled_prints_after_drain() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let tAllSettled = typeof Promise.allSettled;
            let emptyLen = -1;
            let settledLen = -1;
            let s0 = "";
            let v0 = -1;
            let s1 = "";
            let r1 = -1;
            let mixed0 = "";
            let mixedV0 = -1;
            let mixed1 = "";
            let mixedV1 = -1;
            Promise.allSettled([]).then(function (v) {
              emptyLen = v.length;
            });
            Promise.allSettled([Promise.resolve(10), Promise.reject(7)]).then(function (v) {
              settledLen = v.length;
              s0 = v[0].status;
              v0 = v[0].value;
              s1 = v[1].status;
              r1 = v[1].reason;
            });
            Promise.allSettled([1, Promise.resolve(2)]).then(function (v) {
              mixed0 = v[0].status;
              mixedV0 = v[0].value;
              mixed1 = v[1].status;
              mixedV1 = v[1].value;
            });
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "Promise.allSettled must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_all_settled"),
            "should Promise.allSettled via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_object_get"),
            "should read status/value/reason via object_get:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_job_drain"),
            "should drain jobs before observe:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n06-promise-all-settled").expect("workdir");
        let bin = dir.join("promise_all_settled");
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
            stdout, "function\n0\n2\nfulfilled\n10\nrejected\n7\nfulfilled\n1\nfulfilled\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_promise_any_prints_after_drain() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let tAny = typeof Promise.any;
            let winner = -1;
            let mixed = -1;
            let allRejected = 0;
            let errName = "";
            let errLen = -1;
            let emptyRejected = 0;
            let emptyName = "";
            let emptyLen = -1;
            Promise.any([Promise.resolve(10), Promise.resolve(20)]).then(function (v) {
              winner = v;
            });
            Promise.any([1, Promise.resolve(2)]).then(function (v) {
              mixed = v;
            });
            Promise.any([Promise.reject(7), Promise.reject(9)]).then(
              function () {
                allRejected = -1;
              },
              function (e) {
                allRejected = 1;
                errName = e.name;
                errLen = e.errors.length;
              }
            );
            Promise.any([]).then(
              function () {
                emptyRejected = -1;
              },
              function (e) {
                emptyRejected = 1;
                emptyName = e.name;
                emptyLen = e.errors.length;
              }
            );
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "Promise.any must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_any"),
            "should Promise.any via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_object_get"),
            "should read name/errors via object_get:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_job_drain"),
            "should drain jobs before observe:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n06-promise-any").expect("workdir");
        let bin = dir.join("promise_any");
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
            stdout, "function\n10\n1\n1\nAggregateError\n2\n1\nAggregateError\n0\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
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
