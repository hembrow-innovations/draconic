//! Promise then / construct / finally via the job queue.

use super::*;
use std::process::Command;

#[test]
fn promise_resolve_reject_then_via_job_queue() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_promise");
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

        static int g_resolved;
        static int g_rejected;
        static int g_chained;
        static int g_late;
        static int g_double;

        static void *on_resolve(void *data, void *value) {
            (void)data;
            g_resolved = (int)(intptr_t)value;
            return value;
        }

        static void *on_reject(void *data, void *reason) {
            (void)data;
            g_rejected = (int)(intptr_t)reason;
            return reason;
        }

        static void *on_chain(void *data, void *value) {
            (void)data;
            return (void *)(intptr_t)((int)(intptr_t)value + 1);
        }

        static void *on_chained(void *data, void *value) {
            (void)data;
            g_chained = (int)(intptr_t)value;
            return value;
        }

        static void *on_late(void *data, void *value) {
            (void)data;
            g_late = (int)(intptr_t)value;
            return value;
        }

        static void *on_double(void *data, void *value) {
            (void)data;
            g_double += (int)(intptr_t)value;
            return value;
        }

        int main(void) {
            DraconicValue *p = draconic_rt_promise_new();
            if (!p || !draconic_rt_is_promise(p)) {
                fprintf(stderr, "promise_new/is_promise failed\n");
                return 1;
            }
            if (draconic_rt_promise_state(p) != 0) {
                fprintf(stderr, "want pending\n");
                return 2;
            }

            /* then before settle: reactions run after drain */
            (void)draconic_rt_promise_then(p, on_resolve, NULL, NULL, NULL);
            draconic_rt_promise_resolve(p, (void *)(intptr_t)42);
            if (draconic_rt_promise_state(p) != 1) {
                fprintf(stderr, "want fulfilled\n");
                return 3;
            }
            if ((intptr_t)draconic_rt_promise_result(p) != 42) {
                fprintf(stderr, "result want 42\n");
                return 4;
            }
            if (g_resolved != 0) {
                fprintf(stderr, "reaction must not run before drain\n");
                return 5;
            }
            if (draconic_rt_job_pending() == 0) {
                fprintf(stderr, "settle should enqueue reaction job\n");
                return 6;
            }
            draconic_rt_job_drain();
            if (g_resolved != 42) {
                fprintf(stderr, "resolved want 42 got %d\n", g_resolved);
                return 7;
            }

            /* reject path */
            DraconicValue *q = draconic_rt_promise_new();
            (void)draconic_rt_promise_then(q, NULL, NULL, on_reject, NULL);
            draconic_rt_promise_reject(q, (void *)(intptr_t)7);
            draconic_rt_job_drain();
            if (draconic_rt_promise_state(q) != 2) {
                fprintf(stderr, "want rejected\n");
                return 8;
            }
            if (g_rejected != 7) {
                fprintf(stderr, "rejected want 7 got %d\n", g_rejected);
                return 9;
            }

            /* chain: then returns derived promise; callback return settles it */
            DraconicValue *c0 = draconic_rt_promise_new();
            DraconicValue *c1 = draconic_rt_promise_then(c0, on_chain, NULL, NULL, NULL);
            DraconicValue *c2 = draconic_rt_promise_then(c1, on_chained, NULL, NULL, NULL);
            if (!draconic_rt_is_promise(c1) || !draconic_rt_is_promise(c2)) {
                fprintf(stderr, "then must return promise\n");
                return 10;
            }
            draconic_rt_promise_resolve(c0, (void *)(intptr_t)1);
            draconic_rt_job_drain();
            if (g_chained != 2) {
                fprintf(stderr, "chained want 2 got %d\n", g_chained);
                return 11;
            }

            /* then after already settled still schedules a job */
            DraconicValue *late = draconic_rt_promise_new();
            draconic_rt_promise_resolve(late, (void *)(intptr_t)99);
            (void)draconic_rt_promise_then(late, on_late, NULL, NULL, NULL);
            if (g_late != 0) {
                fprintf(stderr, "late then must not run sync\n");
                return 12;
            }
            draconic_rt_job_drain();
            if (g_late != 99) {
                fprintf(stderr, "late want 99 got %d\n", g_late);
                return 13;
            }

            /* double resolve is a no-op; reaction fires once */
            DraconicValue *d = draconic_rt_promise_new();
            (void)draconic_rt_promise_then(d, on_double, NULL, NULL, NULL);
            draconic_rt_promise_resolve(d, (void *)(intptr_t)5);
            draconic_rt_promise_resolve(d, (void *)(intptr_t)100);
            draconic_rt_job_drain();
            if (g_double != 5) {
                fprintf(stderr, "double want 5 got %d\n", g_double);
                return 14;
            }
            if ((intptr_t)draconic_rt_promise_result(d) != 5) {
                fprintf(stderr, "double result want 5\n");
                return 15;
            }

            puts("promise-ok");
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
    assert!(status.success(), "clang failed to link promise test");

    let output = Command::new(&bin).output().expect("run rt_promise");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "promise binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "promise-ok\n", "stdout={stdout:?}");
}

#[test]
fn promise_construct_executor_then_via_job_queue() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_promise_construct");
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

        static int g_resolved;
        static int g_rejected;
        static int g_chained;

        static void exec_resolve(void *data,
            DraconicPromiseSettleFn resolve, void *resolve_cap,
            DraconicPromiseSettleFn reject, void *reject_cap) {
            (void)data; (void)reject; (void)reject_cap;
            resolve(resolve_cap, (void *)(intptr_t)42);
        }

        static void exec_reject(void *data,
            DraconicPromiseSettleFn resolve, void *resolve_cap,
            DraconicPromiseSettleFn reject, void *reject_cap) {
            (void)data; (void)resolve; (void)resolve_cap;
            reject(reject_cap, (void *)(intptr_t)7);
        }

        static void exec_one(void *data,
            DraconicPromiseSettleFn resolve, void *resolve_cap,
            DraconicPromiseSettleFn reject, void *reject_cap) {
            (void)data; (void)reject; (void)reject_cap;
            resolve(resolve_cap, (void *)(intptr_t)1);
        }

        static void *on_resolve(void *data, void *value) {
            (void)data;
            g_resolved = (int)(intptr_t)value;
            return value;
        }

        static void *on_reject(void *data, void *reason) {
            (void)data;
            g_rejected = (int)(intptr_t)reason;
            return reason;
        }

        static void *on_chain(void *data, void *value) {
            (void)data;
            return (void *)(intptr_t)((int)(intptr_t)value + 1);
        }

        static void *on_chained(void *data, void *value) {
            (void)data;
            g_chained = (int)(intptr_t)value;
            return value;
        }

        int main(void) {
            DraconicValue *p = draconic_rt_promise_construct(exec_resolve, NULL);
            if (!p || !draconic_rt_is_promise(p)) {
                fprintf(stderr, "construct failed\n");
                return 1;
            }
            if (draconic_rt_promise_state(p) != 1) {
                fprintf(stderr, "sync resolve in executor should fulfill\n");
                return 2;
            }
            (void)draconic_rt_promise_then(p, on_resolve, NULL, NULL, NULL);
            draconic_rt_job_drain();
            if (g_resolved != 42) {
                fprintf(stderr, "resolved want 42 got %d\n", g_resolved);
                return 3;
            }

            DraconicValue *q = draconic_rt_promise_construct(exec_reject, NULL);
            (void)draconic_rt_promise_then(q, NULL, NULL, on_reject, NULL);
            draconic_rt_job_drain();
            if (g_rejected != 7) {
                fprintf(stderr, "rejected want 7 got %d\n", g_rejected);
                return 4;
            }

            DraconicValue *c0 = draconic_rt_promise_construct(exec_one, NULL);
            DraconicValue *c1 = draconic_rt_promise_then(c0, on_chain, NULL, NULL, NULL);
            (void)draconic_rt_promise_then(c1, on_chained, NULL, NULL, NULL);
            draconic_rt_job_drain();
            if (g_chained != 2) {
                fprintf(stderr, "chained want 2 got %d\n", g_chained);
                return 5;
            }

            draconic_rt_print_str("construct-ok");
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
        "clang failed to link promise construct test"
    );

    let output = Command::new(&bin)
        .output()
        .expect("run rt_promise_construct");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "construct binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "construct-ok\n", "stdout={stdout:?}");
}

#[test]
fn promise_finally_pass_through_via_job_queue() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_promise_finally");
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

        static int g_fulfilled_side;
        static int g_rejected_side;
        static int g_resolved;
        static int g_caught;

        static void *on_fulfilled_side(void *data, void *value) {
            (void)data; (void)value;
            g_fulfilled_side = 1;
            return (void *)(intptr_t)999;
        }

        static void *on_rejected_side(void *data, void *reason) {
            (void)data; (void)reason;
            g_rejected_side = 1;
            return (void *)(intptr_t)888;
        }

        static void *on_resolve(void *data, void *value) {
            (void)data;
            g_resolved = (int)(intptr_t)value;
            return value;
        }

        static void *on_catch(void *data, void *reason) {
            (void)data;
            g_caught = (int)(intptr_t)reason;
            return reason;
        }

        int main(void) {
            DraconicValue *p = draconic_rt_promise_new();
            DraconicValue *pf = draconic_rt_promise_finally(p, on_fulfilled_side, NULL);
            (void)draconic_rt_promise_then(pf, on_resolve, NULL, NULL, NULL);
            draconic_rt_promise_resolve(p, (void *)(intptr_t)42);
            draconic_rt_job_drain();
            if (g_fulfilled_side != 1) {
                fprintf(stderr, "fulfilled side want 1 got %d\n", g_fulfilled_side);
                return 1;
            }
            if (g_resolved != 42) {
                fprintf(stderr, "resolved want 42 got %d (callback return must not replace)\n", g_resolved);
                return 2;
            }

            DraconicValue *q = draconic_rt_promise_new();
            DraconicValue *qf = draconic_rt_promise_finally(q, on_rejected_side, NULL);
            (void)draconic_rt_promise_then(qf, NULL, NULL, on_catch, NULL);
            draconic_rt_promise_reject(q, (void *)(intptr_t)7);
            draconic_rt_job_drain();
            if (g_rejected_side != 1) {
                fprintf(stderr, "rejected side want 1 got %d\n", g_rejected_side);
                return 3;
            }
            if (g_caught != 7) {
                fprintf(stderr, "caught want 7 got %d\n", g_caught);
                return 4;
            }

            /* already settled */
            g_fulfilled_side = 0;
            g_resolved = 0;
            DraconicValue *r = draconic_rt_promise_new();
            draconic_rt_promise_resolve(r, (void *)(intptr_t)11);
            DraconicValue *rf = draconic_rt_promise_finally(r, on_fulfilled_side, NULL);
            (void)draconic_rt_promise_then(rf, on_resolve, NULL, NULL, NULL);
            draconic_rt_job_drain();
            if (g_fulfilled_side != 1 || g_resolved != 11) {
                fprintf(stderr, "settled finally failed side=%d resolved=%d\n",
                    g_fulfilled_side, g_resolved);
                return 5;
            }

            draconic_rt_print_str("finally-ok");
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
        "clang failed to link promise finally test"
    );

    let output = Command::new(&bin).output().expect("run rt_promise_finally");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "finally binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "finally-ok\n", "stdout={stdout:?}");
}
