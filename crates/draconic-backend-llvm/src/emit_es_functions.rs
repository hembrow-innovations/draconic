use super::*;
use std::process::Command;

use draconic_frontend::compile_source;
use draconic_ir::Module;

fn module_of(src: &str) -> Module {
    compile_source(src).expect("compile")
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
fn es_labelled_function_prints_native() {
    let ir = emit_llvm_ir(&module_of(include_str!(
        "../../../tests/conformance/fixtures/es/annex-b/labelled_function.drac"
    )))
    .expect("emit");
    assert!(
        !ir.contains("draconic_rt_hello"),
        "labelled_function fixture must not use hello stub:\n{ir}"
    );
    assert!(
        ir.contains("draconic_rt_print_f64"),
        "should print f64 results:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-n08-16-11-labelled-fn").expect("workdir");
    let bin = dir.join("labelled_function");
    build_native_binary(&ir, &bin).expect("build");
    let output = Command::new(&bin).output().expect("run");
    assert!(
        output.status.success(),
        "exit {:?}\nstderr={}\nir=\n{ir}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "5\n2\n3\n", "stdout={stdout:?}\nir=\n{ir}");
}

#[test]
fn es_if_function_prints_native() {
    let ir = emit_llvm_ir(&module_of(include_str!(
        "../../../tests/conformance/fixtures/es/annex-b/if_function.drac"
    )))
    .expect("emit");
    assert!(
        !ir.contains("draconic_rt_hello"),
        "if_function fixture must not use hello stub:\n{ir}"
    );
    assert!(
        ir.contains("draconic_rt_print_f64"),
        "should print f64 results:\n{ir}"
    );
    assert!(
        ir.contains("draconic_rt_print_str"),
        "should print typeof string:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-n08-16-12-if-fn").expect("workdir");
    let bin = dir.join("if_function");
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
        stdout, "1\nundefined\n4\n5\n7\n",
        "stdout={stdout:?}\nir=\n{ir}"
    );
}

#[test]
fn es_block_function_prints_native() {
    let ir = emit_llvm_ir(&module_of(include_str!(
        "../../../tests/conformance/fixtures/es/annex-b/block_function.drac"
    )))
    .expect("emit");
    assert!(
        !ir.contains("draconic_rt_hello"),
        "block_function fixture must not use hello stub:\n{ir}"
    );
    assert!(
        ir.contains("draconic_rt_print_f64"),
        "should print f64 results:\n{ir}"
    );
    assert!(
        ir.contains("draconic_rt_print_str"),
        "should print typeof strings:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-n08-16-13-block-fn").expect("workdir");
    let bin = dir.join("block_function");
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
        stdout, "1\nfunction\nundefined\n3\n4\n2\n5\n",
        "stdout={stdout:?}\nir=\n{ir}"
    );
}

#[test]
fn es_arguments_object_prints_native() {
    let ir = emit_llvm_ir(&module_of(include_str!(
        "../../../tests/conformance/fixtures/es/annex-b/arguments_object.drac"
    )))
    .expect("emit");
    assert!(
        !ir.contains("draconic_rt_hello"),
        "arguments_object fixture must not use hello stub:\n{ir}"
    );
    assert!(
        ir.contains("draconic_rt_print_f64"),
        "should print f64 results:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-n08-16-24-arguments").expect("workdir");
    let bin = dir.join("arguments_object");
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
        stdout, "5\n32\n1\n8\n3\n6\n",
        "stdout={stdout:?}\nir=\n{ir}"
    );
}

#[test]
fn es_var_decl_prints_native() {
    let ir = emit_llvm_ir(&module_of(include_str!(
        "../../../tests/conformance/fixtures/es/annex-b/var_decl.drac"
    )))
    .expect("emit");
    assert!(
        !ir.contains("draconic_rt_hello"),
        "var_decl fixture must not use hello stub:\n{ir}"
    );
    assert!(
        ir.contains("draconic_rt_print_f64"),
        "should print f64 results:\n{ir}"
    );
    assert!(
        ir.contains("draconic_rt_print_str"),
        "should print undefined strings:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-n08-16-14-var-decl").expect("workdir");
    let bin = dir.join("var_decl");
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
        stdout, "3\n2\nundefined\n4\nundefined\nundefined\nundefined\n6\n",
        "stdout={stdout:?}\nir=\n{ir}"
    );
}

#[test]
fn es_var_for_prints_native() {
    let ir = emit_llvm_ir(&module_of(include_str!(
        "../../../tests/conformance/fixtures/es/annex-b/var_for.drac"
    )))
    .expect("emit");
    assert!(
        !ir.contains("draconic_rt_hello"),
        "var_for fixture must not use hello stub:\n{ir}"
    );
    assert!(
        ir.contains("draconic_rt_print_f64") && ir.contains("draconic_rt_print_str"),
        "should print number and string results:\n{ir}"
    );
    assert!(
        ir.contains("draconic_rt_array_get") || ir.contains("forof_"),
        "should lower array for-of:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-n08-16-15-var-for").expect("workdir");
    let bin = dir.join("var_for");
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
        stdout, "ab\n78\n3\nxy\nx\ny\n",
        "stdout={stdout:?}\nir=\n{ir}"
    );
}
