use super::*;
use std::process::Command;

use draconic_frontend::compile_source;
use draconic_ir::Module;

fn module_of(src: &str) -> Module {
    compile_source(src).expect("compile")
}

#[test]
fn for_await_of_custom_async_iterable_prints_native() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/es/annex-b/for_await_of.drac"
    ))
    .expect("read");
    let m = module_of(&src);
    assert!(
        crate::es_generators::is_es_generators_module(&m),
        "expected es_generators classify to accept for_await_of"
    );
    let ir = emit_llvm_ir(&m).expect("emit");
    assert!(
        !ir.contains("draconic_rt_hello"),
        "for_await_of must not use hello stub:\n{ir}"
    );
    let dir = work_dir("draconic-llvm-n08-for-await-of").expect("workdir");
    let bin = dir.join("for_await_of");
    build_native_binary(&ir, &bin).expect("build");
    let output = std::process::Command::new(&bin).output().expect("run");
    assert!(
        output.status.success(),
        "exit {:?}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "6\n9\n30\n6\n3\n4\n",
        "stdout={:?}",
        String::from_utf8_lossy(&output.stdout)
    );
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
