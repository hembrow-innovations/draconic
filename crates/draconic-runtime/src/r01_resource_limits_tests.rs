//! R01: combined embed/eval resource limits (parent of R01.01–R01.03).
//!
//! Alloc-budget exhaustion and time-budget interrupt fail closed at the C ABI
//! (`NULL` / exceeded flag), not as a JS exception (ADR-0011).

use super::*;
use std::process::Command;

#[test]
fn r01_combined_alloc_and_time_budgets_fail_closed() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_r01_limits");
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
                DraconicValue *a;
                DraconicValue *b;
                size_t header;

                draconic_rt_gc_init();
                draconic_rt_gc_set_alloc_threshold(0);

                /* Both knobs default to unlimited. */
                if (draconic_rt_gc_alloc_budget() != 0) {
                    fprintf(stderr, "default alloc budget want 0\n");
                    return 1;
                }
                if (draconic_rt_eval_time_budget_ms() != 0) {
                    fprintf(stderr, "default time budget want 0\n");
                    return 2;
                }

                a = draconic_rt_alloc_object();
                if (!a) {
                    fprintf(stderr, "unlimited alloc failed\n");
                    return 3;
                }
                header = draconic_rt_gc_alloc_bytes();
                draconic_rt_gc_root_push(a);

                /* R01.02: over-budget alloc returns NULL; process stays alive. */
                draconic_rt_gc_set_alloc_budget(header);
                b = draconic_rt_alloc_object();
                if (b) {
                    fprintf(stderr, "over-budget alloc should fail closed\n");
                    return 4;
                }

                /* R01.03: time budget exceeded flag; process stays alive. */
                draconic_rt_eval_set_time_budget_ms(1);
                draconic_rt_eval_time_begin();
                draconic_rt_sleep_ms(20.0);
                if (!draconic_rt_eval_time_exceeded()) {
                    fprintf(stderr, "1ms budget should fail closed after sleep\n");
                    return 5;
                }

                /* Combined: alloc still fail-closed while time is exceeded. */
                if (draconic_rt_alloc_object()) {
                    fprintf(stderr, "alloc should stay fail-closed after time exceed\n");
                    return 6;
                }

                /* Unlimited restores both knobs without aborting. */
                draconic_rt_gc_set_alloc_budget(0);
                draconic_rt_eval_set_time_budget_ms(0);
                draconic_rt_eval_time_begin();
                if (!draconic_rt_alloc_object()) {
                    fprintf(stderr, "budget 0 alloc should succeed\n");
                    return 7;
                }
                if (draconic_rt_eval_time_exceeded()) {
                    fprintf(stderr, "budget 0 time should be unlimited\n");
                    return 8;
                }

                puts("r01-limits-ok");
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
        "clang failed to link r01 combined limits test"
    );

    let output = Command::new(&bin).output().expect("run rt_r01_limits");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "r01 combined limits binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "r01-limits-ok\n", "stdout={stdout:?}");
}
