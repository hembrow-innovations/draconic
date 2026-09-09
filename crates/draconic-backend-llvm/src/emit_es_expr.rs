use super::*;
use std::process::Command;

use draconic_frontend::compile_source;
use draconic_ir::Module;

fn module_of(src: &str) -> Module {
    compile_source(src).expect("compile")
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
        stdout, "8\n1024\n512\n64\n1\n1\n32\n9\n-8\n16\nbigint\n",
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
        stdout, "3\n3\n4\n4\n1\n3\n1024\n3\n-1\ntrue\ntrue\nobject\n4\ntrue\ntrue\n",
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
        stdout, "symbol\nsymbol\ntrue\ntrue\nshared\nfunction\nfunction\nfunction\n",
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
        stdout, "1\nundefined\n2\nundefined\n3\n3\nundefined\n4\nundefined\n5\n6\n7\n6\n",
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
        stdout,
        expected,
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
        stdout,
        expected,
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
        stdout,
        expected,
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
        stdout, "hello1\nworld\nhello world!\nx1y2z3\na`b\ntrue\np9q\nm7n\n",
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
