//! R01.03: eval time budget interrupts / fails closed when exceeded.

use super::*;
use std::process::Command;

#[test]
fn eval_set_time_budget_ms_declare() {
    assert_eq!(
        EVAL_SET_TIME_BUDGET_MS.declare(),
        "declare void @draconic_rt_eval_set_time_budget_ms(i64)"
    );
}

#[test]
fn eval_time_budget_ms_declare() {
    assert_eq!(
        EVAL_TIME_BUDGET_MS.declare(),
        "declare i64 @draconic_rt_eval_time_budget_ms()"
    );
}

#[test]
fn eval_time_begin_declare() {
    assert_eq!(
        EVAL_TIME_BEGIN.declare(),
        "declare void @draconic_rt_eval_time_begin()"
    );
}

#[test]
fn eval_time_exceeded_declare() {
    assert_eq!(
        EVAL_TIME_EXCEEDED.declare(),
        "declare i32 @draconic_rt_eval_time_exceeded()"
    );
}

#[test]
fn eval_time_budget_fail_closed_when_exceeded() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_eval_time_budget");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
            #include "draconic_rt.h"
            #include <stdio.h>

            int main(void) {
                if (draconic_rt_eval_time_budget_ms() != 0) {
                    fprintf(stderr, "default budget want 0\n");
                    return 1;
                }
                draconic_rt_eval_time_begin();
                if (draconic_rt_eval_time_exceeded()) {
                    fprintf(stderr, "unlimited should not exceed\n");
                    return 2;
                }

                draconic_rt_eval_set_time_budget_ms(1);
                if (draconic_rt_eval_time_budget_ms() != 1) {
                    fprintf(stderr, "budget getter mismatch\n");
                    return 3;
                }
                draconic_rt_eval_time_begin();
                if (draconic_rt_eval_time_exceeded()) {
                    fprintf(stderr, "fresh 1ms budget already exceeded\n");
                    return 4;
                }
                draconic_rt_sleep_ms(20.0);
                if (!draconic_rt_eval_time_exceeded()) {
                    fprintf(stderr, "1ms budget should fail closed after sleep\n");
                    return 5;
                }

                draconic_rt_eval_set_time_budget_ms(0);
                draconic_rt_eval_time_begin();
                draconic_rt_sleep_ms(5.0);
                if (draconic_rt_eval_time_exceeded()) {
                    fprintf(stderr, "budget 0 should be unlimited\n");
                    return 6;
                }

                draconic_rt_eval_set_time_budget_ms(10000);
                draconic_rt_eval_time_begin();
                if (draconic_rt_eval_time_exceeded()) {
                    fprintf(stderr, "generous budget should not exceed immediately\n");
                    return 7;
                }

                puts("eval-time-budget-ok");
                return 0;
            }
            "#,
    )
    .unwrap();

    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg("-I")
            .arg(&header_dir)
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("spawn clang")
    };
    assert!(
        status.success(),
        "clang failed to link eval time budget test"
    );

    let output = Command::new(&bin).output().expect("run rt_eval_time_budget");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "eval time budget binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "eval-time-budget-ok\n", "stdout={stdout:?}");
}
