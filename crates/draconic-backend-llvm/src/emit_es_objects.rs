use super::*;
use std::process::Command;

use draconic_frontend::compile_source;
use draconic_ir::Module;

fn module_of(src: &str) -> Module {
    compile_source(src).expect("compile")
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
    assert_eq!(
        stdout, "1\n2\n10\n10\n3\n6\n3\n",
        "stdout={stdout:?}\nir=\n{ir}"
    );
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
    assert_eq!(stdout, "3\n3\n6\n9\n7\n7\n", "stdout={stdout:?}\nir=\n{ir}");
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
        stdout, "6\n0\nab\n60\n15\n5\n3\n5\n6\n",
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
fn es_class_expr_prints_native() {
    let ir = emit_llvm_ir(&module_of(
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/annex-b/class_expr.drac"
        ))
        .expect("read fixture")
        .as_str(),
    ))
    .expect("emit class_expr");
    assert!(
        !ir.contains("draconic_rt_hello"),
        "class_expr must not use hello stub:\n{ir}"
    );
    assert!(
        ir.contains("draconic_rt_alloc_object"),
        "class_expr must alloc objects:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-n08-class-expr").expect("workdir");
    let bin = dir.join("class_expr");
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
        stdout, "1\n2\n3\n6\nCounter\n13\n10\n7\n42\n",
        "stdout={stdout:?}\nir=\n{ir}"
    );
}

#[test]
fn es_class_expr_name_prints_native() {
    let ir = emit_llvm_ir(&module_of(include_str!(
        "../../../tests/conformance/fixtures/es/annex-b/class_expr_name.drac"
    )))
    .expect("emit class_expr_name");
    assert!(
        !ir.contains("draconic_rt_hello"),
        "class_expr_name must not use hello stub:\n{ir}"
    );
    assert!(
        ir.contains("draconic_rt_print_str"),
        "class_expr_name must print name strings:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-n08-16-34-class-expr-name").expect("workdir");
    let bin = dir.join("class_expr_name");
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
        "cls\nX\nfunction\ndCls\nY\nfunction\noCls\nZ\nfunction\npCls\nW\nfunction\naCls\nQ\n",
        "stdout={stdout:?}\nir=\n{ir}"
    );
}
