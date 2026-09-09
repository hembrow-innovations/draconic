use super::*;
use std::process::Command;

use draconic_frontend::compile_source;
use draconic_ir::Module;

fn module_of(src: &str) -> Module {
    compile_source(src).expect("compile")
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
        ir.contains("N08.10") || ir.contains("throw/try/catch"),
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
    assert_eq!(stdout, "1\n1\n1\n7\n5\n", "stdout={stdout:?}\nir=\n{ir}");
}

#[test]
fn r04_01_catchable_exceptions_prints_native() {
    let ir = emit_llvm_ir(&module_of(include_str!(
        "../../../tests/conformance/fixtures/security/panic_policy/catchable_exceptions.drac"
    )))
    .expect("emit");
    assert!(
        !ir.contains("draconic_rt_hello"),
        "catchable_exceptions must not use hello stub:\n{ir}"
    );
    assert!(
        ir.contains("N08.10") || ir.contains("throw/try/catch"),
        "should use exceptions emit path:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-r04-01-catchable").expect("workdir");
    let bin = dir.join("catchable_exceptions");
    build_native_binary(&ir, &bin).expect("build");
    let output = Command::new(&bin).output().expect("run");
    assert!(
        output.status.success(),
        "exit {:?}\nstderr={}\nir=\n{ir}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "1\n1\n7\n1\n", "stdout={stdout:?}\nir=\n{ir}");
}

#[test]
fn r04_02_abort_process_kills_native() {
    let ir = emit_llvm_ir(&module_of(include_str!(
        "../../../tests/conformance/fixtures/security/panic_policy/abort_process.drac"
    )))
    .expect("emit");
    assert!(
        !ir.contains("draconic_rt_hello"),
        "abort_process must not use hello stub:\n{ir}"
    );
    assert!(
        ir.contains("draconic_rt_abort"),
        "should call Runtime abort:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-r04-02-abort").expect("workdir");
    let bin = dir.join("abort_process");
    build_native_binary(&ir, &bin).expect("build");
    let output = Command::new(&bin).output().expect("run");
    assert!(
        !output.status.success(),
        "abort must kill the process; exit {:?}\nstdout={}\nstderr={}\nir=\n{ir}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.is_empty(),
        "abort must not print after; stdout={stdout:?}\nir=\n{ir}"
    );
}

#[test]
fn es_try_finally_prints_native() {
    let ir = emit_llvm_ir(&module_of(include_str!(
        "../../../tests/conformance/fixtures/es/exceptions/try_finally.drac"
    )))
    .expect("emit");
    assert!(
        !ir.contains("draconic_rt_hello"),
        "try_finally must not use hello stub:\n{ir}"
    );
    assert!(
        ir.contains("N08.10.02") || ir.contains("finally"),
        "should use exceptions finally emit path:\n{ir}"
    );
    assert!(
        ir.contains("draconic_rt_print_f64"),
        "should print f64 results:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-n08-try-finally").expect("workdir");
    let bin = dir.join("try_finally");
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
        stdout, "11\n11\n23\n5\n1\n11\n",
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

#[test]
fn es_legacy_with_basic_prints_native() {
    let ir = emit_llvm_ir(&module_of(
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/legacy/with_basic.drac"
        ))
        .expect("read fixture")
        .as_str(),
    ))
    .expect("emit with_basic");
    assert!(
        !ir.contains("draconic_rt_hello"),
        "es_legacy must not use hello stub:\n{ir}"
    );
    assert!(
        ir.contains("draconic_rt_print_f64"),
        "es_legacy must print numbers:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-n08-legacy-with").expect("workdir");
    let bin = dir.join("with_basic");
    build_native_binary(&ir, &bin).expect("build");
    let output = Command::new(&bin).output().expect("run");
    assert!(
        output.status.success(),
        "exit {:?}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "1\n2\n10\n20\n3\n", "stdout={stdout:?}");
}

#[test]
fn es_legacy_with_nested_prints_native() {
    let ir = emit_llvm_ir(&module_of(
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/legacy/with_nested.drac"
        ))
        .expect("read fixture")
        .as_str(),
    ))
    .expect("emit with_nested");
    assert!(
        !ir.contains("draconic_rt_hello"),
        "es_legacy must not use hello stub:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-n08-legacy-with-nested").expect("workdir");
    let bin = dir.join("with_nested");
    build_native_binary(&ir, &bin).expect("build");
    let output = Command::new(&bin).output().expect("run");
    assert!(
        output.status.success(),
        "exit {:?}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "1\n2\n7\n", "stdout={stdout:?}");
}
