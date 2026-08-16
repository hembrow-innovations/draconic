//! LLVM backend: IR → native (one lowerer; private adapters for supported subsets).

mod es_arrays;
mod es_call_spread;
mod es_classes;
mod es_coercion;
mod es_eval;
mod es_exceptions;
mod es_expr;
mod es_functions;
mod es_nullish;
mod es_objects;
mod es_promise;
mod es_tagged_template;
mod es_to_primitive;
mod es_values;
mod native_ints;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::Module;

use es_arrays::{emit_es_arrays, is_es_arrays_module};
use es_call_spread::{emit_es_call_spread, is_es_call_spread_module};
use es_classes::{emit_es_classes, is_es_classes_module};
use es_coercion::{emit_es_coercion, is_es_coercion_module};
use es_eval::{emit_es_eval, is_es_eval_module};
use es_exceptions::{emit_es_exceptions, is_es_exceptions_module};
use es_expr::{emit_es_expr, is_es_expr_module};
use es_functions::{emit_es_functions, is_es_functions_module};
use es_nullish::{emit_es_nullish, is_es_nullish_module};
use es_objects::{emit_es_objects, is_es_objects_module};
use es_promise::{emit_es_promise, is_es_promise_module};
use es_tagged_template::{emit_es_tagged_template, is_es_tagged_template_module};
use es_to_primitive::{emit_es_to_primitive, is_es_to_primitive_module};
use es_values::{emit_es_values, is_es_values_module};
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
///   numbers/BigInts/booleans/strings/undefined + Math + Number/NaN/Infinity) via Runtime prints — N08.01.01–N08.01.04.08 / N08.08.01–N08.08.06
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
/// - **`for-in` / `for-of`** over strings (`let`/assign binding; string concat)
///   via Runtime prints — N08.02.08
/// - **Function declaration/expression/arrow** + `return` + call (simple ident params,
///   defaults) + nested decls with free-variable capture + IIFE/named/higher-order via
///   Runtime prints — N08.03.01–N08.03.07
/// - **Object literals** + property access/assignment (string keys; nested objects;
///   number props) via Runtime GC/object ABI — N08.04.01–N08.04.02
/// - **Class declarations** (base + `extends`/`super()` + instance/static methods;
///   `super.m(…)`; `new`; prototype chain) via Runtime GC/object ABI — N08.05.01–N08.05.04
/// - **Array literals** + index access + `.length` + destructuring via Runtime
///   array ABI — N08.06.01–N08.06.06
/// - **String lit** + concat (incl. number ToString) + `.length` + index via
///   length-aware C-string ABI — N08.07.01
/// - **Untagged templates** — N08.07.02
/// - **Unicode escapes** (`\x`/`\u`/`\u{}`) + UTF-16 `.length` — N08.07.03
/// - **Tagged templates** `` tag`…` `` (quasi array + interps; method/call tags) — N08.07.04
/// - **Symbol basics** (`Symbol()` / `Symbol.for` / `Symbol.keyFor` / typeof / `===`) — N08.09.01
/// - **Symbol property keys** (computed/get/set; no string collision) — N08.09.02
/// - **Abstract equality & coercion** (`==`/`!=` mixed types; ToNumber/ToString/ToBoolean) — N08.09.03
/// - **ToPrimitive** (`valueOf`/`toString` hooks in `+` and `==`) — N08.09.04
/// - **Exceptions** (`throw` + bare `try`/`catch`; catch binding; nested; throw from fn) — N08.10.01
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
    if is_es_exceptions_module(module) {
        return emit_es_exceptions(module);
    }
    if is_es_nullish_module(module) {
        return emit_es_nullish(module);
    }
    if is_es_to_primitive_module(module) {
        return emit_es_to_primitive(module);
    }
    if is_es_coercion_module(module) {
        return emit_es_coercion(module);
    }
    if is_es_values_module(module) {
        return emit_es_values(module);
    }
    if is_es_call_spread_module(module) {
        return emit_es_call_spread(module);
    }
    if is_es_tagged_template_module(module) {
        return emit_es_tagged_template(module);
    }
    if is_es_functions_module(module) {
        return emit_es_functions(module);
    }
    if is_es_classes_module(module) {
        return emit_es_classes(module);
    }
    if is_es_objects_module(module) {
        return emit_es_objects(module);
    }
    if is_es_arrays_module(module) {
        return emit_es_arrays(module);
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
           ES expressions (arithmetic/comparison/logical/bitwise/pow/conditional/assign/compound-assign/update/comma/typeof/void/delete/nullish/logical-assign/if-else/while/do-while/for/for-in/for-of/break/continue/switch/labeled), ES function decl/expr/arrow/return/call (simple params+defaults+rest, nested+capture, IIFE/named/HOF), ES object lit + property access/assignment + method this, ES class decl (base ctor+methods), ES array lit + index/length, ES throw/try/catch, empty hello)",
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
    fn es_numbers_number_literals_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/numbers/number_literals.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "number_literals must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-number-literals").expect("workdir");
        let bin = dir.join("number_literals");
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
            stdout,
            "42\n0\n3.1400000000000001\n0.5\n0.5\n5\n1000\n1000\n150\n200\n0.10000000000000001\n6.02e+23\n255\n255\n16\n10\n10\n15\n15\n1000\n1000000\n65535\n161\n1000.5\n10000000000\n36\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_numbers_bigint_literals_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/numbers/bigint_literals.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "bigint_literals must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_i64"),
            "should print i64 BigInt results:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bytes") || ir.contains("draconic_rt_print_str"),
            "should print typeof string:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-bigint-literals").expect("workdir");
        let bin = dir.join("bigint_literals");
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
            stdout,
            "1\n2\n3\n2\n20\n3\n1\n255\n255\n10\n10\n15\n15\n1000\n65535\n161\n0\n-1\nbigint\n36\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_numbers_bigint_pow_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/numbers/bigint_pow.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "bigint_pow must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_i64"),
            "should print i64 BigInt results:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bytes") || ir.contains("draconic_rt_print_str"),
            "should print typeof string:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-bigint-pow").expect("workdir");
        let bin = dir.join("bigint_pow");
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
            stdout,
            "8\n1024\n512\n64\n1\n1\n32\n9\n-8\n16\nbigint\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_numbers_math_basics_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/numbers/math_basics.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "math_basics must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("llvm.fabs.f64") || ir.contains("llvm.floor.f64"),
            "should use Math f64 intrinsics:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 Math results:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bytes") || ir.contains("draconic_rt_print_str"),
            "should print typeof Math string:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-math-basics").expect("workdir");
        let bin = dir.join("math_basics");
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
            stdout,
            "3\n3\n4\n4\n1\n3\n1024\n3\n-1\ntrue\ntrue\nobject\n4\ntrue\ntrue\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_numbers_number_global_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/numbers/number_global.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "number_global must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("fcmp uno") || ir.contains("0x7FF8000000000000"),
            "should lower Number.isNaN / NaN:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bool"),
            "should print boolean Number results:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bytes") || ir.contains("draconic_rt_print_str"),
            "should print typeof strings:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-number-global").expect("workdir");
        let bin = dir.join("number_global");
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
            stdout,
            "true\nfalse\ntrue\nfalse\ntrue\nfalse\ntrue\nfalse\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\nfunction\nnumber\nnumber\ntrue\ntrue\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_values_symbol_basics_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/values/symbol_basics.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "symbol_basics must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_symbol_new") && ir.contains("draconic_rt_symbol_for"),
            "should lower Symbol / Symbol.for:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_symbol_key_for"),
            "should lower Symbol.keyFor:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-symbol-basics").expect("workdir");
        let bin = dir.join("symbol_basics");
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
            stdout,
            "symbol\nsymbol\ntrue\ntrue\nshared\nfunction\nfunction\nfunction\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_values_symbol_property_keys_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/values/symbol_property_keys.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "symbol_property_keys must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_object_set_symbol")
                && ir.contains("draconic_rt_object_get_symbol"),
            "should lower symbol-keyed get/set:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "should alloc objects:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-symbol-property-keys").expect("workdir");
        let bin = dir.join("symbol_property_keys");
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
            stdout,
            "1\nundefined\n2\nundefined\n3\n3\nundefined\n4\nundefined\n5\n6\n7\n6\n",
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
    fn es_objects_lit_access_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/objects/object_lit_access.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit object_lit_access");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_objects must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "es_objects must alloc objects:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-objects").expect("workdir");
        let bin = dir.join("object_lit_access");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "1\n1\n3\n3\n4\n4\n", "stdout={stdout:?}");
    }

    #[test]
    fn es_objects_property_assign_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/objects/property_assign.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit property_assign");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_objects must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_object_set"),
            "es_objects must set properties:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-objects-assign").expect("workdir");
        let bin = dir.join("property_assign");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "2\n3\n4\n5\n6\n7\n8\n8\n", "stdout={stdout:?}");
    }

    #[test]
    fn es_objects_this_method_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/objects/this_method.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit this_method");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_objects must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_") || ir.contains("define double @es_m_fn_"),
            "es_objects must emit method functions:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-objects-this").expect("workdir");
        let bin = dir.join("this_method");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "10\n10\n15\n3\n7\n7\n", "stdout={stdout:?}");
    }

    #[test]
    fn es_objects_new_ctor_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/objects/new_ctor.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit new_ctor");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_objects must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "es_objects new must alloc instances:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_"),
            "es_objects must emit constructor functions:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-objects-new").expect("workdir");
        let bin = dir.join("new_ctor");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "1\n2\n10\n10\n3\n6\n3\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_objects_prototype_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/objects/prototype.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit prototype");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_objects must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_object_set_proto"),
            "es_objects prototype must set [[Prototype]]:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_"),
            "es_objects must emit prototype methods:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-objects-proto").expect("workdir");
        let bin = dir.join("prototype");
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
            stdout, "3\n3\n6\n9\n7\n7\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_objects_lit_sugar_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/objects/object_lit_sugar.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit object_lit_sugar");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_objects must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "es_objects sugar must alloc objects:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_"),
            "es_objects sugar must emit method functions:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-objects-sugar").expect("workdir");
        let bin = dir.join("object_lit_sugar");
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
            stdout, "1\n2\n1\n2\n3\n4\n5\n6\n7\n1\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_classes_basic_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/classes/class_basic.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit class_basic");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_classes must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "es_classes must alloc objects:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_object_set_proto"),
            "es_classes must set [[Prototype]]:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_"),
            "es_classes must emit ctor/method functions:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-classes-basic").expect("workdir");
        let bin = dir.join("class_basic");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "1\n2\n3\n6\n7\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_classes_extends_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/classes/class_extends.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit class_extends");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_classes extends must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "es_classes extends must alloc objects:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_object_set_proto"),
            "es_classes extends must set [[Prototype]]:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_"),
            "es_classes extends must emit ctor/method functions:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-classes-extends").expect("workdir");
        let bin = dir.join("class_extends");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "1\n3\n1\n1\n2\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_classes_static_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/classes/class_static.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit class_static");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_classes static must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "es_classes static must alloc objects:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_"),
            "es_classes static must emit ctor/method functions:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-classes-static").expect("workdir");
        let bin = dir.join("class_static");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "5\n42\n7\n9\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_classes_super_access_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/classes/class_super_access.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit class_super_access");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_classes super must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "es_classes super must alloc objects:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_object_set_proto"),
            "es_classes super must set [[Prototype]]:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_"),
            "es_classes super must emit ctor/method functions:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-classes-super-access").expect("workdir");
        let bin = dir.join("class_super_access");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "5\n7\n15\n9\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_arrays_lit_access_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/arrays/array_lit_access.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit array_lit_access");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_arrays must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_new"),
            "es_arrays must alloc arrays:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_get"),
            "es_arrays must get elements:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_len"),
            "es_arrays must read length:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-arrays").expect("workdir");
        let bin = dir.join("array_lit_access");
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
            stdout, "1\n2\n3\n3\n0\n10\n21\n1\n2\ntwo\n7\n8\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_arrays_element_assign_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/arrays/array_element_assign.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit array_element_assign");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_arrays assign must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_set"),
            "es_arrays assign must set elements:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_get"),
            "es_arrays assign must get elements:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-arrays-assign").expect("workdir");
        let bin = dir.join("array_element_assign");
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
            stdout, "10\n2\n20\n2\n30\n40\n40\n7\n1\n9\n3\n5\n6\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_arrays_spread_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/arrays/array_spread.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit array_spread");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_arrays spread must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_spread_array")
                || ir.contains("draconic_rt_array_spread_cstr"),
            "es_arrays spread must call spread helpers:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-arrays-spread").expect("workdir");
        let bin = dir.join("array_spread");
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
            stdout,
            "1\n2\n2\n1\n2\n3\n3\n0\n1\n2\n3\n4\n10\n1\n2\n3\n1\n2\n99\n7\n0\n5\n1\n1\n2\n3\n3\na\nb\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_arrays_for_of_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/arrays/array_for_of.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit array_for_of");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_arrays for-of must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_get") && ir.contains("draconic_rt_array_len"),
            "es_arrays for-of must iterate via array get/len:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-arrays-for-of").expect("workdir");
        let bin = dir.join("array_for_of");
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
            stdout,
            "6\n0\nab\n60\n15\n5\n3\n5\n6\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_arrays_destructure_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/arrays/array_destructure.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(src.as_str())).expect("emit array_destructure");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_arrays destructure must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_get") && ir.contains("draconic_rt_array_new"),
            "es_arrays destructure must use array ABI:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-arrays-destructure").expect("workdir");
        let bin = dir.join("array_destructure");
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
            stdout,
            "1\n2\n3\n1\n2\n3\n4\n2\n10\n20\n30\n60\n7\n8\n15\n100\n200\n300\n2\n5\n6\n11\n2\n12\n13\n1\n3\n7\n4\n1\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_call_spread_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/arrays/call_spread.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit call_spread");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "call_spread must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("cs_fn_") || ir.contains("call double"),
            "call_spread must emit calls:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-call-spread").expect("workdir");
        let bin = dir.join("call_spread");
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
            stdout, "6\n60\n6\n6\n6\nxyz\n7\n8\n1\n2\n15\n",
            "stdout={stdout:?}\nir=\n{ir}"
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
    fn es_strings_lit_access_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/strings/string_lit_access.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "string_lit_access must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bytes"),
            "should print length-aware strings:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-string-lit-access").expect("workdir");
        let bin = dir.join("string_lit_access");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = output.stdout;
        let expected = b"hello\nworld\n\n\nhelloworld\nabc\nn1\n2n\nx\ny\nabc\n3\n0\na\nb\nc\n1\nb\na\nb\na\tb\na\rb\na\\b\na\"b\na'b\na\0b\nit's \"ok\"\nstring\ntrue\ntrue\n";
        assert_eq!(
            stdout, expected,
            "stdout={:?}\nir=\n{ir}",
            String::from_utf8_lossy(&stdout)
        );
    }

    #[test]
    fn es_strings_template_lit_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/strings/template_lit.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "template_lit must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bytes"),
            "should print length-aware strings:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-template-lit").expect("workdir");
        let bin = dir.join("template_lit");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = output.stdout;
        let expected = b"hello\n\na\nb\nworld\nhello world\nworld!\naworldb\n3\nn=3\nx1y2z\nsum=3\nouter inner world end\na`b$c\\d\na\nb\nstring\ntrue\nab\n";
        assert_eq!(
            stdout, expected,
            "stdout={:?}\nir=\n{ir}",
            String::from_utf8_lossy(&stdout)
        );
    }

    #[test]
    fn es_strings_unicode_escapes_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/strings/unicode_escapes.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "unicode_escapes must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bytes"),
            "should print length-aware strings:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-unicode-escapes").expect("workdir");
        let bin = dir.join("unicode_escapes");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = output.stdout;
        // Content is UTF-8; .length is UTF-16 code units (emoji=2, ©=1).
        let mut expected = Vec::new();
        expected.extend_from_slice(b"AB\na\n");
        expected.push(0); // hex_null
        expected.extend_from_slice(b"\nA\n");
        expected.extend_from_slice("\u{00A9}".as_bytes());
        expected.extend_from_slice(b"\n \nA\n");
        expected.extend_from_slice("\u{1F600}".as_bytes());
        expected.extend_from_slice(b"\n");
        expected.extend_from_slice("\u{00FF}".as_bytes());
        expected.extend_from_slice(b"\nABC\nHi\nOK\n");
        expected.extend_from_slice("\u{1F4A9}".as_bytes());
        expected.extend_from_slice(b"\nxAy\ntrue\ntrue\ntrue\n2\n1\nHi\n");
        assert_eq!(
            stdout, expected,
            "stdout={:?}\nir=\n{ir}",
            String::from_utf8_lossy(&stdout)
        );
    }

    #[test]
    fn es_strings_tagged_template_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/strings/tagged_template.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "tagged_template must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_new"),
            "should build quasi array:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-tagged-template").expect("workdir");
        let bin = dir.join("tagged_template");
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
            stdout,
            "hello1\nworld\nhello world!\nx1y2z3\na`b\ntrue\np9q\nm7n\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
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
    fn es_for_in_of_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/statements/for_in_of.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "for_in_of fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("forin_") || ir.contains("forof_"),
            "should lower for-in/for-of loops:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_cstr_concat")
                && ir.contains("draconic_rt_cstr_from_u64")
                && ir.contains("draconic_rt_cstr_from_code_unit"),
            "should use cstr helpers:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-for-in-of").expect("workdir");
        let bin = dir.join("for_in_of");
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
            stdout, "ab\n01\nab\nx\n0\nx\nyz\nz\n01\n1\n2\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_function_decl_return_call_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/functions/decl_return_call.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "decl_return_call fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 result:\n{ir}"
        );
        assert!(
            ir.contains("define double @"),
            "should emit LLVM function for JS function decl:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-fn-decl").expect("workdir");
        let bin = dir.join("decl_return_call");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "1\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_function_nested_capture_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/functions/nested_capture.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "nested_capture fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        assert!(
            ir.contains("define double @"),
            "should emit LLVM functions for nested decls:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-fn-nested").expect("workdir");
        let bin = dir.join("nested_capture");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "42\n17\n3\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_function_default_params_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/functions/default_params.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "default_params fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        assert!(
            ir.contains("define double @"),
            "should emit LLVM functions with defaults:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-fn-defaults").expect("workdir");
        let bin = dir.join("default_params");
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
            stdout, "10\n7\n11\n3\n3\n9\n5\n9\n6\n8\n10\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_function_rest_params_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/functions/rest_params.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "rest_params fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        assert!(
            ir.contains("define double @"),
            "should emit LLVM functions with rest:\n{ir}"
        );
        assert!(
            ir.contains("%rest_buf") || ir.contains("rest_buf"),
            "should pack rest args:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-fn-rest").expect("workdir");
        let bin = dir.join("rest_params");
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
            stdout, "0\n1\n6\n12\n7\n2\n9\n0\n",
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
    fn es_throw_try_catch_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/exceptions/throw_try_catch.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "throw_try_catch must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("N08.10.01") || ir.contains("throw/try/catch"),
            "should use exceptions emit path:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-throw-try-catch").expect("workdir");
        let bin = dir.join("throw_try_catch");
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
            stdout, "1\n1\n1\n7\n5\n",
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


