//! LLVM backend: IR → native (one lowerer; private adapters for supported subsets).

mod es_eval;
mod es_expr;
mod es_nullish;
mod es_promise;
mod native_ints;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::Module;

use es_eval::{emit_es_eval, is_es_eval_module};
use es_expr::{emit_es_expr, is_es_expr_module};
use es_nullish::{emit_es_nullish, is_es_nullish_module};
use es_promise::{emit_es_promise, is_es_promise_module};
use native_ints::{emit_native_ints, is_native_int_module};

/// Emit LLVM IR text for a shared IR module.
///
/// Selects a private adapter for a supported subset, otherwise returns a hard
/// diagnostic (no silent hello-stub success for arbitrary Programs):
///
/// - **Native scalars/layouts** (`i8`–`i64`, `u8`–`u64`, `f32`/`f64`, `bool`,
///   native structs/arrays/pointers) — N01–N03
/// - **Promise / async** (constructor basics through async/await and async
///   arrows) via Runtime Promise ABI — N06.03–N06.11
/// - **eval / Function** (constant-string fold via Embed) — N07.02–N07.04
/// - **ES expressions** (numeric arithmetic + comparison/equality + logical
///   `&&`/`||`/`!` + bitwise `&` `|` `^` `~` `<<` `>>` `>>>` + `**` +
///   conditional `?:` + simple/compound assignment + prefix/postfix `++`/`--` +
///   comma `,` + unary keywords `typeof`/`void`/`delete` over JS
///   numbers/booleans/strings/undefined) via Runtime prints — N08.01.01–N08.01.04.08
/// - **Nullish / logical assignment** (`??` `??=` `&&=` `||=` with mixed
///   null/undefined/number/bool/string) via tagged slots — N08.01.04.09
/// - **`if` / `else`** (block or expression bodies; ToBoolean on number/boolean
///   tests) via Runtime prints — N08.02.01
/// - **`while`** (block or expression bodies; ToBoolean on number/boolean tests)
///   via Runtime prints — N08.02.02
/// - **`do` / `while`** (block or expression bodies; ToBoolean on number/boolean
///   tests) via Runtime prints — N08.02.03
/// - **`for`** (`for (init; test; update)`; `let` init; omitted clauses; block
///   bodies; ToBoolean on number/boolean tests) via Runtime prints — N08.02.04
/// - **`break` / `continue`** (unlabeled, in `while`/`do`/`for`) via Runtime
///   prints — N08.02.05
/// - **`switch` / `case` / `default`** (number discriminant; fall-through;
///   unlabeled `break`) via Runtime prints — N08.02.06
/// - **Labeled statements** + labeled `break` / `continue` via Runtime prints —
///   N08.02.07
/// - **Empty program** — B08 Runtime hello demo only (`main` calls
///   `draconic_rt_hello`)
pub fn emit_llvm_ir(module: &Module) -> Result<String, Diagnostic> {
    if is_native_int_module(module) {
        return emit_native_ints(module);
    }
    if is_es_promise_module(module) {
        return emit_es_promise(module);
    }
    if is_es_eval_module(module) {
        return emit_es_eval(module);
    }
    if is_es_nullish_module(module) {
        return emit_es_nullish(module);
    }
    if is_es_expr_module(module) {
        return emit_es_expr(module);
    }
    if is_empty_program(module) {
        return Ok(emit_empty_hello());
    }
    Err(unsupported_native_diagnostic())
}

fn is_empty_program(module: &Module) -> bool {
    module.body.is_empty()
}

fn unsupported_native_diagnostic() -> Diagnostic {
    Diagnostic::new(
        "native target: unsupported IR (no LLVM lowering for this program; \
         supported: native scalars/layouts, Promise/async subset, eval/Function fold, \
          ES expressions (arithmetic/comparison/logical/bitwise/pow/conditional/assign/compound-assign/update/comma/typeof/void/delete/nullish/logical-assign/if-else/while/do-while/for/break/continue/switch/labeled), empty hello)",
        Span::dummy(),
    )
}

/// B08 empty-program demo: link Runtime hello. Not used for non-empty unsupported IR.
fn emit_empty_hello() -> String {
    use draconic_runtime::abi::HELLO;
    format!(
        "; Draconic LLVM backend empty program (B08 hello)\n{}\n\ndefine i32 @main() {{\nentry:\n  {}\n  ret i32 0\n}}\n",
        HELLO.declare(),
        HELLO.call(""),
    )
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
    std::fs::write(&ll_path, llvm_ir)
        .map_err(|e| Diagnostic::new(format!("write LLVM IR failed: {e}"), Span::dummy()))?;

    let rt_lib = draconic_runtime::build_runtime_static_lib(&work).map_err(|e| {
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
    std::fs::create_dir_all(&dir)
        .map_err(|e| Diagnostic::new(format!("temp dir failed: {e}"), Span::dummy()))?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    fn module_of(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn empty_program_emits_runtime_hello() {
        let ir = emit_llvm_ir(&module_of("")).expect("emit");
        assert!(
            ir.contains("draconic_rt_hello"),
            "IR must declare/call runtime hello:\n{ir}"
        );
        assert!(
            ir.contains("define i32 @main"),
            "IR must define main:\n{ir}"
        );
        assert!(
            ir.contains("call void @draconic_rt_hello"),
            "main must call hello:\n{ir}"
        );
        assert!(ir.contains("ret i32 0"), "main must return 0:\n{ir}");
    }

    #[test]
    fn empty_native_binary_prints_hello() {
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
    fn unsupported_js_module_errors() {
        let err = emit_llvm_ir(&module_of("let o = {};")).expect_err("must reject");
        let msg = err.to_string();
        assert!(
            msg.contains("unsupported") || msg.contains("native target"),
            "diagnostic should mention unsupported native IR:\n{msg}"
        );
        assert!(
            !msg.contains("draconic_rt_hello"),
            "error must not be a hello-stub success path:\n{msg}"
        );
    }

    #[test]
    fn es_expr_unary_keywords_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let t_num = typeof 1;
            let t_str = typeof "hi";
            let t_bool = typeof true;
            let t_null = typeof null;
            let v0 = void 0;
            let v1 = void 1;
            let d_lit = delete 1;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr unary keywords must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_str"),
            "should print string/undefined results:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bool"),
            "should print bool delete result:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-unary-keywords").expect("workdir");
        let bin = dir.join("unary_keywords");
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
            stdout, "number\nstring\nboolean\nobject\nundefined\nundefined\ntrue\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_arithmetic_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let sum = 1 + 2;
            let diff = 10 - 4;
            let prod = 3 * 4;
            let quot = 20 / 5;
            let rem = 10 % 3;
            let prec = 1 + 2 * 3;
            let grouped = (1 + 2) * 3;
            let unary_minus = -5;
            let unary_plus = +7;
            let chain = 1 + 2 + 3 - 4;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr arithmetic must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-arith").expect("workdir");
        let bin = dir.join("arith");
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
            stdout, "3\n6\n12\n4\n1\n7\n9\n-5\n7\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_arithmetic_with_local_refs_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let a = 10;
            let b = 3;
            let sum = a + b;
            let prod = a * b;
            let div = a / b;
            let rem = a % b;
            let chain = a + b * 2 - 4;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "local-ref arithmetic must not use hello stub:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-arith-local").expect("workdir");
        let bin = dir.join("arith_local");
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
            stdout, "10\n3\n13\n30\n3.3333333333333335\n1\n12\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_object_literal_unsupported() {
        let err = emit_llvm_ir(&module_of(
            r#"
            let o = {};
            let n = 1 + 2;
            "#,
        ))
        .expect_err("object program must reject");
        assert!(
            err.to_string().contains("unsupported"),
            "diagnostic should mention unsupported:\n{}",
            err
        );
    }

    #[test]
    fn es_expr_string_literal_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let s = "hi";
            let n = 1 + 2;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr string must not use hello stub:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-string-lit").expect("workdir");
        let bin = dir.join("string_lit");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "hi\n3\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_expr_comparison_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let lt = 1 < 2;
            let lte = 2 <= 2;
            let gt = 3 > 1;
            let gte = 3 >= 3;
            let eq_loose = 1 == 1;
            let ne_loose = 1 != 2;
            let eq_strict = 1 === 1;
            let ne_strict = 1 !== 2;
            let chain = 1 < 2 === true;
            let falsey = 2 < 1;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr comparison must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bool"),
            "should print bool results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-cmp").expect("workdir");
        let bin = dir.join("cmp");
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
            stdout, "true\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\nfalse\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_logical_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let and_tt = true && true;
            let and_tf = true && false;
            let and_ft = false && true;
            let or_ff = false || false;
            let or_ft = false || true;
            let or_tf = true || false;
            let not_t = !true;
            let not_f = !false;
            let prec = !false && true || false;
            let value_and = 1 && 2;
            let value_or = 0 || 3;
            let group = !(false || true);
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr logical must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bool"),
            "should print bool results:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print number results for value-preserving &&/||:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-logical").expect("workdir");
        let bin = dir.join("logical");
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
            stdout, "true\nfalse\nfalse\nfalse\ntrue\ntrue\nfalse\ntrue\ntrue\n2\n3\nfalse\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_bitwise_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let band = 5 & 3;
            let bor = 5 | 2;
            let bxor = 5 ^ 1;
            let bnot = ~0;
            let shl = 1 << 3;
            let shr = -8 >> 2;
            let ushr = -8 >>> 2;
            let prec = 1 | 2 & 4;
            let group = (1 | 2) & 4;
            let chain = 15 & 7 | 8;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr bitwise must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-bitwise").expect("workdir");
        let bin = dir.join("bitwise");
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
            stdout, "1\n7\n4\n-1\n8\n-2\n1073741822\n1\n0\n15\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_exponentiation_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let pow = 2 ** 3;
            let right_assoc = 2 ** 3 ** 2;
            let prec = 2 * 3 ** 2;
            let group = (2 * 3) ** 2;
            let nested = 2 ** (1 + 2);
            let zero = 5 ** 0;
            let one = 9 ** 1;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr exponentiation must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("llvm.pow.f64"),
            "should use pow intrinsic:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-exponentiation").expect("workdir");
        let bin = dir.join("exponentiation");
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
            stdout, "8\n512\n18\n36\n8\n1\n9\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_conditional_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let t = true ? 1 : 2;
            let f = false ? 1 : 2;
            let nested = true ? false ? 3 : 4 : 5;
            let right_assoc = false ? 1 : true ? 2 : 3;
            let prec = 1 < 2 ? 10 : 20;
            let group = (false ? 1 : 2) + 3;
            let num = 0 ? 100 : 200;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr conditional must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("select i1"),
            "should use select for ternary:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-conditional").expect("workdir");
        let bin = dir.join("conditional");
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
            stdout, "1\n2\n4\n2\n10\n5\n200\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_assignment_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let x = 0;
            x = 1;
            let y = 0;
            y = x = 2;
            let z = 0;
            z = true ? 3 : 4;
            let w = 0;
            w = false ? 5 : w = 6;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr assignment must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("store double"),
            "should store assigned values:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-assignment").expect("workdir");
        let bin = dir.join("assignment");
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
            stdout, "2\n2\n3\n6\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_update_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let a = 1;
            let pre_inc = ++a;
            let b = 1;
            let pre_dec = --b;
            let c = 1;
            let post_inc = c++;
            let d = 1;
            let post_dec = d--;
            let e = 5;
            ++e;
            e++;
            --e;
            e--;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr update must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("fadd double") || ir.contains("fsub double"),
            "should use fadd/fsub for ++/--:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-update").expect("workdir");
        let bin = dir.join("update");
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
            stdout, "2\n2\n0\n0\n2\n1\n0\n1\n5\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_compound_assignment_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let a = 10;
            a += 5;
            let b = 10;
            b -= 3;
            let c = 4;
            c *= 3;
            let d = 20;
            d /= 4;
            let e = 17;
            e %= 5;
            let f = 2;
            f **= 3;
            let g = 1;
            g <<= 3;
            let h = 16;
            h >>= 2;
            let i = -8;
            i >>>= 1;
            let j = 15;
            j &= 9;
            let k = 12;
            k ^= 5;
            let l = 8;
            l |= 3;
            let m = 1;
            let n = 2;
            m += n += 3;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr compound assignment must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-compound-assignment").expect("workdir");
        let bin = dir.join("compound_assignment");
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
            stdout, "15\n7\n12\n5\n2\n8\n8\n4\n2147483644\n9\n9\n11\n6\n5\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_comma_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let a = (1, 2);
            let b = (1, 2, 3);
            let c = 0;
            let d = (c = 1, c = 2, 3);
            let e = (true ? 1 : 2, 4);
            let side = 0;
            let f = (side = side + 1, side = side + 1, side);
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr comma must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-comma").expect("workdir");
        let bin = dir.join("comma");
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
            stdout, "2\n3\n2\n3\n4\n2\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_break_continue_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let a = 0;
            while (true) {
              a = a + 1;
              if (a === 3) break;
            }
            let b = 0;
            let c = 0;
            while (b < 5) {
              b = b + 1;
              if (b === 2) continue;
              c = c + 1;
            }
            let d = 0;
            for (let i = 0; i < 10; i = i + 1) {
              d = d + 1;
              if (i === 2) break;
            }
            let e = 0;
            for (let j = 0; j < 5; j = j + 1) {
              if (j === 2) continue;
              e = e + 1;
            }
            let f = 0;
            do {
              f = f + 1;
              if (f === 2) break;
            } while (true);
            let g = 0;
            let h = 0;
            do {
              g = g + 1;
              if (g === 2) continue;
              h = h + 1;
            } while (g < 4);
            let outer = 0;
            let inner = 0;
            while (outer < 3) {
              outer = outer + 1;
              while (true) {
                inner = inner + 1;
                break;
              }
            }
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr break/continue must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-break-continue").expect("workdir");
        let bin = dir.join("break_continue");
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
            stdout, "3\n5\n4\n3\n4\n2\n4\n3\n3\n3\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_switch_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/statements/switch.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "switch fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("switch_end") || ir.contains("case"),
            "should lower switch with case labels:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-switch").expect("workdir");
        let bin = dir.join("switch");
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
            stdout, "10\n20\n40\n11\n1\n5\n1\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_labeled_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/statements/labeled.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "labeled fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("lbl_end") || ir.contains("while_end") || ir.contains("for_end"),
            "should lower labeled break/continue targets:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-labeled").expect("workdir");
        let bin = dir.join("labeled");
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
            stdout, "1\n5\n4\n1\n1\n2\n1\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
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
    fn es_async_await_prints_after_drain() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let resolved = 0;
            let fromAwait = 0;
            let rejected = 0;
            let exprResolved = 0;
            async function f() {
              return 42;
            }
            f().then(function (v) {
              resolved = v;
            });
            async function g() {
              let x = await Promise.resolve(7);
              return x + 1;
            }
            g().then(function (v) {
              fromAwait = v;
            });
            async function h() {
              throw 9;
            }
            h().then(
              function () {
                rejected = -1;
              },
              function (e) {
                rejected = e;
              }
            );
            let af = async function () {
              return 1;
            };
            af().then(function (v) {
              exprResolved = v;
            });
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "async/await must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_await") || ir.contains("draconic_rt_promise_new"),
            "should lower async via Runtime Promise ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_job_drain"),
            "should drain jobs before observe:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n06-async-await").expect("workdir");
        let bin = dir.join("async_await");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "42\n8\n9\n1\n", "stdout={stdout:?}\nir=\n{ir}");
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

    #[test]
    fn es_direct_eval_prints_via_embed() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let t = typeof eval;
            let g = globalThis.eval === eval;
            let a = eval("1 + 2");
            let b = eval("typeof undefined");
            let c = eval("3 * 4");
            let d = eval("'hi'");
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "direct eval must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("N07.02") || ir.contains("direct eval"),
            "should use eval emit path:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n07-direct-eval").expect("workdir");
        let bin = dir.join("direct_eval");
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
            stdout, "function\ntrue\n3\nundefined\n12\nhi\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_new_function_prints_via_embed() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let tf = typeof Function;
            let same = globalThis.Function === Function;
            let f = new Function("a", "b", "return a + b");
            let g = Function("x", "return x * 2");
            let h = new Function("return 7");
            let r1 = f(1, 2);
            let r2 = g(3);
            let r3 = h();
            let t1 = typeof f;
            let t2 = typeof g;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "new Function must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("N07.03") || ir.contains("Function via Embed"),
            "should use Function emit path:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n07-new-function").expect("workdir");
        let bin = dir.join("new_function");
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
            stdout, "function\ntrue\nfunction\nfunction\nfunction\n3\n6\n7\nfunction\nfunction\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_nullish_logical_assign_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/expressions/nullish_logical_assign.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "nullish fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("N08.01.04.09") || ir.contains("nullish"),
            "should use nullish emit path:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-nullish").expect("workdir");
        let bin = dir.join("nullish");
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
            stdout, "1\n2\n0\n\nfalse\n6\n7\n10\n0\n\n13\n14\n1\n16\nnull\n18\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_if_else_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/statements/if_else.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "if_else fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("br i1") && ir.contains("then") && ir.contains("endif"),
            "should lower if/else with branches:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-if-else").expect("workdir");
        let bin = dir.join("if_else");
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
            stdout, "1\n0\n10\n20\n3\n5\n1\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_while_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/statements/while.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "while fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("br i1") && ir.contains("while_head") && ir.contains("while_end"),
            "should lower while with loop branches:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-while").expect("workdir");
        let bin = dir.join("while");
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
            stdout, "3\n0\n2\n3\n6\n0\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_do_while_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/statements/do_while.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "do_while fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("br i1") && ir.contains("do_body") && ir.contains("do_end"),
            "should lower do/while with loop branches:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-do-while").expect("workdir");
        let bin = dir.join("do_while");
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
            stdout, "3\n1\n2\n3\n6\n0\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_for_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/statements/for.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "for fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("br i1") && ir.contains("for_head") && ir.contains("for_end"),
            "should lower for with loop branches:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-for").expect("workdir");
        let bin = dir.join("for");
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
            stdout, "3\n2\n6\n3\n5\n3\n2\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_indirect_eval_prints_via_embed() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            globalThis.gx = 100;
            function probeDirect() {
              let gx = 200;
              return eval("gx");
            }
            function probeIndirectComma() {
              let gx = 200;
              return (0, eval)("gx");
            }
            function probeIndirectGlobalThis() {
              let gx = 200;
              return globalThis.eval("gx");
            }
            let d = probeDirect();
            let i = probeIndirectComma();
            let g = probeIndirectGlobalThis();
            let t = typeof (0, eval);
            let same = globalThis.eval === eval;
            let a = (0, eval)("1 + 2");
            let b = globalThis.eval("'hi'");
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "indirect eval must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("N07.04") || ir.contains("indirect eval"),
            "should use indirect eval emit path:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n07-indirect-eval").expect("workdir");
        let bin = dir.join("indirect_eval");
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
            stdout, "200\n100\n100\nfunction\ntrue\n3\nhi\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }
}
