//! R01.02: GC alloc budget fails closed when exceeded.

use super::*;
use std::process::Command;

#[test]
fn gc_set_alloc_budget_declare() {
    assert_eq!(
        GC_SET_ALLOC_BUDGET.declare(),
        "declare void @draconic_rt_gc_set_alloc_budget(i64)"
    );
}

#[test]
fn gc_alloc_budget_declare() {
    assert_eq!(
        GC_ALLOC_BUDGET.declare(),
        "declare i64 @draconic_rt_gc_alloc_budget()"
    );
}

#[test]
fn gc_alloc_bytes_declare() {
    assert_eq!(
        GC_ALLOC_BYTES.declare(),
        "declare i64 @draconic_rt_gc_alloc_bytes()"
    );
}

#[test]
fn gc_alloc_budget_fail_closed_when_exceeded() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_gc_budget");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
            #include "draconic_rt.h"
            #include <stdio.h>
            #include <string.h>

            int main(void) {
                DraconicValue *a;
                DraconicValue *b;
                size_t header;
                size_t budget;

                draconic_rt_gc_init();
                draconic_rt_gc_set_alloc_threshold(0);

                if (draconic_rt_gc_alloc_budget() != 0) {
                    fprintf(stderr, "default budget want 0\n");
                    return 1;
                }
                if (draconic_rt_gc_alloc_bytes() != 0) {
                    fprintf(stderr, "init alloc_bytes want 0\n");
                    return 2;
                }

                a = draconic_rt_alloc_object();
                if (!a) {
                    fprintf(stderr, "unlimited alloc failed\n");
                    return 3;
                }
                header = draconic_rt_gc_alloc_bytes();
                if (header == 0) {
                    fprintf(stderr, "header bytes still 0\n");
                    return 4;
                }

                draconic_rt_gc_root_push(a);
                draconic_rt_gc_set_alloc_budget(header);
                if (draconic_rt_gc_alloc_budget() != header) {
                    fprintf(stderr, "budget getter mismatch\n");
                    return 5;
                }

                b = draconic_rt_alloc_object();
                if (b) {
                    fprintf(stderr, "over-budget alloc should fail closed\n");
                    return 6;
                }
                if (draconic_rt_gc_live_count() != 1) {
                    fprintf(stderr, "live after failed alloc want 1 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 7;
                }

                draconic_rt_gc_root_pop();
                draconic_rt_gc_collect();
                b = draconic_rt_alloc_object();
                if (!b) {
                    fprintf(stderr, "alloc after collect should succeed\n");
                    return 8;
                }

                draconic_rt_gc_set_alloc_budget(1);
                if (draconic_rt_alloc_object()) {
                    fprintf(stderr, "tiny budget still allocated\n");
                    return 9;
                }

                draconic_rt_gc_set_alloc_budget(0);
                if (!draconic_rt_alloc_object()) {
                    fprintf(stderr, "budget 0 should be unlimited\n");
                    return 10;
                }

                budget = header + 8;
                draconic_rt_gc_shutdown();
                draconic_rt_gc_init();
                draconic_rt_gc_set_alloc_threshold(0);
                draconic_rt_gc_set_alloc_budget(budget);
                a = draconic_rt_alloc_string("hi", 2);
                if (!a) {
                    fprintf(stderr, "string under budget failed\n");
                    return 11;
                }
                draconic_rt_gc_root_push(a);
                b = draconic_rt_alloc_string("abcdefgh", 8);
                if (b) {
                    fprintf(stderr, "string over budget should fail closed\n");
                    return 12;
                }

                puts("gc-budget-ok");
                draconic_rt_gc_shutdown();
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
        "clang failed to link gc alloc budget test"
    );

    let output = Command::new(&bin).output().expect("run rt_gc_budget");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "gc alloc budget binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "gc-budget-ok\n", "stdout={stdout:?}");
}
