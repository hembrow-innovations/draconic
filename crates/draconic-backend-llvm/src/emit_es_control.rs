use super::*;
use std::process::Command;

use draconic_frontend::compile_source;
use draconic_ir::Module;

fn module_of(src: &str) -> Module {
    compile_source(src).expect("compile")
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
    assert_eq!(stdout, "2\n2\n3\n6\n", "stdout={stdout:?}\nir=\n{ir}");
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
    assert_eq!(stdout, "3\n0\n2\n3\n6\n0\n", "stdout={stdout:?}\nir=\n{ir}");
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
    assert_eq!(stdout, "3\n1\n2\n3\n6\n0\n", "stdout={stdout:?}\nir=\n{ir}");
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
