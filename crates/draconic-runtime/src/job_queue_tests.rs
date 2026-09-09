//! Job-queue ABI and FIFO drain native tests.

use super::*;
use std::process::Command;

#[test]
fn c_runtime_exports_job_queue_abi() {
    let src = c_runtime_source();
    let hdr = c_runtime_header_source();
    for sym in JOB_QUEUE_SYMBOLS {
        assert!(src.contains(sym), "C runtime source must define {sym}");
        assert!(hdr.contains(sym), "C runtime header must declare {sym}");
    }
}

#[test]
fn c_runtime_exports_promise_abi() {
    let src = c_runtime_source();
    let hdr = c_runtime_header_source();
    for sym in PROMISE_SYMBOLS {
        assert!(src.contains(sym), "C runtime source must define {sym}");
        assert!(hdr.contains(sym), "C runtime header must declare {sym}");
    }
}

#[test]
fn job_queue_fifo_drain_and_nested_enqueue() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_job_queue");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>

        static int g_order[8];
        static size_t g_n;

        static void push_order(int v) {
            if (g_n < 8) {
                g_order[g_n++] = v;
            }
        }

        static void job_a(void *data) {
            (void)data;
            push_order(1);
        }

        static void job_b(void *data) {
            (void)data;
            push_order(2);
            /* Nested enqueue during drain: must run after the current job,
               after already-queued siblings (FIFO of the whole queue). */
            draconic_rt_job_enqueue(job_a, NULL); /* will be 1 again as job 4 */
        }

        static void job_c(void *data) {
            (void)data;
            push_order(3);
        }

        static void job_print_i64(void *data) {
            int64_t v = (int64_t)(intptr_t)data;
            draconic_rt_print_i64(v);
        }

        int main(void) {
            if (draconic_rt_job_pending() != 0) {
                fprintf(stderr, "pending want 0 got %zu\n",
                        draconic_rt_job_pending());
                return 1;
            }

            draconic_rt_job_enqueue(job_a, NULL);
            draconic_rt_job_enqueue(job_b, NULL);
            draconic_rt_job_enqueue(job_c, NULL);

            if (draconic_rt_job_pending() != 3) {
                fprintf(stderr, "pending want 3 got %zu\n",
                        draconic_rt_job_pending());
                return 2;
            }

            draconic_rt_job_drain();

            if (draconic_rt_job_pending() != 0) {
                fprintf(stderr, "after drain pending want 0 got %zu\n",
                        draconic_rt_job_pending());
                return 3;
            }

            /* Expected order: A, B (enqueues another A), C, nested A → 1,2,3,1 */
            if (g_n != 4
                || g_order[0] != 1
                || g_order[1] != 2
                || g_order[2] != 3
                || g_order[3] != 1) {
                fprintf(stderr, "order want 1,2,3,1 got");
                for (size_t i = 0; i < g_n; i++) {
                    fprintf(stderr, " %d", g_order[i]);
                }
                fprintf(stderr, "\n");
                return 4;
            }

            /* Second drain is a no-op; print path observes jobs ran. */
            draconic_rt_job_enqueue(job_print_i64, (void *)(intptr_t)42);
            draconic_rt_job_drain();
            draconic_rt_job_drain();

            puts("job-queue-ok");
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
    assert!(status.success(), "clang failed to link job queue test");

    let output = Command::new(&bin).output().expect("run rt_job_queue");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "job queue binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "42\njob-queue-ok\n", "stdout={stdout:?}");
}
