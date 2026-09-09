use super::*;
use std::process::Command;

use draconic_frontend::compile_source;
use draconic_ir::Module;

fn module_of(src: &str) -> Module {
    compile_source(src).expect("compile")
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

/// F06.03: `extern "C"` lowers to LLVM `declare` ABI surface; call links libc `abs`.
#[test]
fn native_extern_c_declare_and_call_abs() {
    let m = module_of(
        r#"
        extern "C" function abs(x: i32): i32;
        extern "C" function puts(s: *u8): i32;
        extern "C" function free(p: *u8): void;
        let a: i32 = abs(-42);
        "#,
    );
    assert!(m.has_extern_ffi);
    assert!(
        m.body.iter().any(|s| matches!(
            s,
            draconic_ir::Stmt::ExternFunction { name, .. } if name == "abs"
        )),
        "IR must keep ExternFunction: {:?}",
        m.body
    );
    let ir = emit_llvm_ir(&m).expect("emit");
    assert!(
        !ir.contains("draconic_rt_hello"),
        "extern module must not use hello stub:\n{ir}"
    );
    assert!(
        ir.contains("declare i32 @abs(i32)"),
        "expected declare abs:\n{ir}"
    );
    assert!(
        ir.contains("declare i32 @puts(ptr)"),
        "expected declare puts:\n{ir}"
    );
    assert!(
        ir.contains("declare void @free(ptr)"),
        "expected declare free:\n{ir}"
    );
    assert!(ir.contains("call i32 @abs("), "expected call abs:\n{ir}");
    let dir = work_dir("draconic-llvm-f06-03-extern").expect("workdir");
    let bin = dir.join("extern_abs");
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

/// F01.01: multi-arg i32 extern "C" call → runtime `draconic_rt_add_i32`.
#[test]
fn native_extern_c_call_i32_multi_arg() {
    let m = module_of(
        r#"
        extern "C" function draconic_rt_add_i32(a: i32, b: i32): i32;
        let s: i32 = draconic_rt_add_i32(20, 22);
        "#,
    );
    let ir = emit_llvm_ir(&m).expect("emit");
    assert!(
        ir.contains("declare i32 @draconic_rt_add_i32(i32, i32)"),
        "expected declare add_i32:\n{ir}"
    );
    assert!(
        ir.contains("call i32 @draconic_rt_add_i32("),
        "expected call add_i32:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-f01-01-add-i32").expect("workdir");
    let bin = dir.join("extern_add_i32");
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

/// F01.02: i64 / f64 / void extern "C" calls via Runtime ABI.
#[test]
fn native_extern_c_call_i64_f64_void() {
    let m = module_of(
        r#"
        extern "C" function draconic_rt_add_i64(a: i64, b: i64): i64;
        extern "C" function draconic_rt_add_f64(a: f64, b: f64): f64;
        extern "C" function draconic_rt_touch_void(): void;
        draconic_rt_touch_void();
        let s: i64 = draconic_rt_add_i64(3000000000, 2000000000);
        let t: f64 = draconic_rt_add_f64(10.5, 2.0);
        "#,
    );
    let ir = emit_llvm_ir(&m).expect("emit");
    assert!(
        ir.contains("declare i64 @draconic_rt_add_i64(i64, i64)"),
        "expected declare add_i64:\n{ir}"
    );
    assert!(
        ir.contains("declare double @draconic_rt_add_f64(double, double)"),
        "expected declare add_f64:\n{ir}"
    );
    assert!(
        ir.contains("declare void @draconic_rt_touch_void()"),
        "expected declare touch_void:\n{ir}"
    );
    assert!(
        ir.contains("call i64 @draconic_rt_add_i64("),
        "expected call add_i64:\n{ir}"
    );
    assert!(
        ir.contains("call double @draconic_rt_add_f64("),
        "expected call add_f64:\n{ir}"
    );
    assert!(
        ir.contains("call void @draconic_rt_touch_void()"),
        "expected call touch_void:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-f01-02-i64-f64-void").expect("workdir");
    let bin = dir.join("extern_i64_f64_void");
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
        stdout, "void\n5000000000\n12.5\n",
        "stdout={stdout:?}\nir=\n{ir}"
    );
}

/// F01.03: pointer (`*i32`) and null args to extern "C" via Runtime ABI.
#[test]
fn native_extern_c_call_ptr_and_null() {
    let m = module_of(
        r#"
        extern "C" function draconic_rt_load_i32(p: *i32): i32;
        let x: i32 = 42;
        let p: *i32 = &x;
        let a: i32 = draconic_rt_load_i32(p);
        let b: i32 = draconic_rt_load_i32(&x);
        let n: *i32 = null;
        let c: i32 = draconic_rt_load_i32(n);
        let d: i32 = draconic_rt_load_i32(null);
        "#,
    );
    let ir = emit_llvm_ir(&m).expect("emit");
    assert!(
        ir.contains("declare i32 @draconic_rt_load_i32(ptr)"),
        "expected declare load_i32:\n{ir}"
    );
    assert!(
        ir.contains("call i32 @draconic_rt_load_i32(ptr"),
        "expected call load_i32 with ptr:\n{ir}"
    );
    assert!(
        ir.contains("call i32 @draconic_rt_load_i32(ptr null)")
            || ir.contains("call i32 @draconic_rt_load_i32(ptr null,"),
        "expected call with null pointer:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-f01-03-ptr").expect("workdir");
    let bin = dir.join("extern_ptr");
    build_native_binary(&ir, &bin).expect("build");
    let output = Command::new(&bin).output().expect("run");
    assert!(
        output.status.success(),
        "exit {:?}\nstderr={}\nir=\n{ir}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "42\n42\n42\n0\n0\n", "stdout={stdout:?}\nir=\n{ir}");
}

/// F02.01: export a Draconic fn as a C function pointer (pass to extern).
#[test]
fn native_export_fn_as_c_function_pointer() {
    let m = module_of(
        r#"
        function twice(x: i32): i32 {
          return x + x;
        }
        extern "C" function draconic_rt_fnptr_nonnull(cb: function): i32;
        let ok: i32 = draconic_rt_fnptr_nonnull(twice);
        "#,
    );
    let ir = emit_llvm_ir(&m).expect("emit");
    assert!(
        ir.contains("declare i32 @draconic_rt_fnptr_nonnull(ptr)"),
        "expected declare fnptr helper:\n{ir}"
    );
    assert!(
        ir.contains("define i32 @d_twice_"),
        "expected Draconic fn define:\n{ir}"
    );
    assert!(
        ir.contains("call i32 @draconic_rt_fnptr_nonnull(ptr @d_twice_"),
        "expected pass of fn address as ptr:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-f02-01-fnptr").expect("workdir");
    let bin = dir.join("export_fnptr");
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

/// F02.02: host invokes callback with scalar args; return value observed.
#[test]
fn native_host_invokes_callback_scalar_args() {
    let m = module_of(
        r#"
        function add(a: i32, b: i32): i32 {
          return a + b;
        }
        extern "C" function draconic_rt_call_i32_i32(cb: function, a: i32, b: i32): i32;
        let r: i32 = draconic_rt_call_i32_i32(add, 20, 22);
        let s: i32 = draconic_rt_call_i32_i32(add, -5, 12);
        "#,
    );
    let ir = emit_llvm_ir(&m).expect("emit");
    assert!(
        ir.contains("declare i32 @draconic_rt_call_i32_i32(ptr, i32, i32)"),
        "expected declare call helper:\n{ir}"
    );
    assert!(
        ir.contains("define i32 @d_add_"),
        "expected Draconic fn define:\n{ir}"
    );
    assert!(
        ir.contains("call i32 @draconic_rt_call_i32_i32(ptr @d_add_"),
        "expected pass of fn address as ptr:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-f02-02-invoke").expect("workdir");
    let bin = dir.join("invoke_scalar");
    build_native_binary(&ir, &bin).expect("build");
    let output = Command::new(&bin).output().expect("run");
    assert!(
        output.status.success(),
        "exit {:?}\nstderr={}\nir=\n{ir}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "42\n7\n", "stdout={stdout:?}\nir=\n{ir}");
}

/// F03.01: native layout field offsets match C ABI (i32+i64 padding; i8+i32 padding).
#[test]
fn native_repr_c_struct_field_offsets() {
    let m = module_of(
        r#"
        type Pair = { a: i32; b: i64 };
        type Small = { x: i8; y: i32 };
        extern "C" function draconic_rt_layout_i32_i64_a(p: *u8): i32;
        extern "C" function draconic_rt_layout_i32_i64_b(p: *u8): i64;
        extern "C" function draconic_rt_layout_i32_i64_write(p: *u8, a: i32, b: i64): void;
        extern "C" function draconic_rt_layout_i8_i32_x(p: *u8): i8;
        extern "C" function draconic_rt_layout_i8_i32_y(p: *u8): i32;
        let p: Pair = { a: 10, b: 20 };
        let ra: i32 = draconic_rt_layout_i32_i64_a(&p);
        let rb: i64 = draconic_rt_layout_i32_i64_b(&p);
        let q: Pair = { a: 0, b: 0 };
        draconic_rt_layout_i32_i64_write(&q, 7, 8);
        let qa: i32 = q.a;
        let qb: i64 = q.b;
        let s: Small = { x: 1, y: 99 };
        let sx: i8 = draconic_rt_layout_i8_i32_x(&s);
        let sy: i32 = draconic_rt_layout_i8_i32_y(&s);
        "#,
    );
    let ir = emit_llvm_ir(&m).expect("emit");
    assert!(
        ir.contains("declare i32 @draconic_rt_layout_i32_i64_a(ptr)"),
        "expected declare layout a:\n{ir}"
    );
    assert!(
        ir.contains("{ i32, i64 }"),
        "expected LLVM struct {{ i32, i64 }}:\n{ir}"
    );
    assert!(
        ir.contains("{ i8, i32 }"),
        "expected LLVM struct {{ i8, i32 }}:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-f03-01-layout").expect("workdir");
    let bin = dir.join("layout_offsets");
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
        stdout, "10\n20\n10\n20\n7\n8\n7\n8\n1\n99\n1\n99\n",
        "stdout={stdout:?}\nir=\n{ir}"
    );
}

/// F03.02: pass/return native layout struct by value or pointer across FFI.
#[test]
fn native_pass_return_struct_across_ffi() {
    let m = module_of(
        r#"
        type Pair = { a: i32; b: i64 };
        extern "C" function draconic_rt_layout_pass_i32_i64(p: Pair): i32;
        extern "C" function draconic_rt_layout_ret_i32_i64(a: i32, b: i64): Pair;
        extern "C" function draconic_rt_layout_pass_i32_i64_ptr(p: *u8): i32;
        let p: Pair = { a: 10, b: 20 };
        let by_val: i32 = draconic_rt_layout_pass_i32_i64(p);
        let by_ptr: i32 = draconic_rt_layout_pass_i32_i64_ptr(&p);
        let q: Pair = draconic_rt_layout_ret_i32_i64(7, 8);
        let qa: i32 = q.a;
        let qb: i64 = q.b;
        "#,
    );
    let ir = emit_llvm_ir(&m).expect("emit");
    assert!(
        ir.contains("declare i32 @draconic_rt_layout_pass_i32_i64([2 x i64])"),
        "expected by-value Pair param as [2 x i64]:\n{ir}"
    );
    assert!(
        ir.contains("declare [2 x i64] @draconic_rt_layout_ret_i32_i64(i32, i64)"),
        "expected by-value Pair return as [2 x i64]:\n{ir}"
    );
    assert!(
        ir.contains("declare i32 @draconic_rt_layout_pass_i32_i64_ptr(ptr)"),
        "expected pointer Pair param:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-f03-02-pass-return").expect("workdir");
    let bin = dir.join("pass_return");
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
        stdout, "10\n20\n30\n30\n7\n8\n7\n8\n",
        "stdout={stdout:?}\nir=\n{ir}"
    );
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
