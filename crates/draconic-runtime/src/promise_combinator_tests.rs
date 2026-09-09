//! Promise all / race / allSettled / any via the job queue.

use super::*;
use std::process::Command;

#[test]
fn promise_all_array_via_job_queue() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_promise_all");
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

        static int g_empty_len = -1;
        static int g_all_len = -1;
        static int g_a0 = -1;
        static int g_a1 = -1;
        static int g_mixed0 = -1;
        static int g_mixed1 = -1;
        static int g_rejected = 0;

        static void *on_empty(void *data, void *value) {
            (void)data;
            g_empty_len = (int)draconic_rt_array_len((DraconicValue *)value);
            return value;
        }

        static void *on_all(void *data, void *value) {
            (void)data;
            DraconicValue *arr = (DraconicValue *)value;
            g_all_len = (int)draconic_rt_array_len(arr);
            g_a0 = (int)(intptr_t)draconic_rt_array_get(arr, 0);
            g_a1 = (int)(intptr_t)draconic_rt_array_get(arr, 1);
            return value;
        }

        static void *on_mixed(void *data, void *value) {
            (void)data;
            DraconicValue *arr = (DraconicValue *)value;
            g_mixed0 = (int)(intptr_t)draconic_rt_array_get(arr, 0);
            g_mixed1 = (int)(intptr_t)draconic_rt_array_get(arr, 1);
            return value;
        }

        static void *on_reject_ok(void *data, void *value) {
            (void)data; (void)value;
            g_rejected = -1;
            return value;
        }

        static void *on_reject_err(void *data, void *reason) {
            (void)data;
            g_rejected = (int)(intptr_t)reason;
            return reason;
        }

        int main(void) {
            DraconicValue *empty = draconic_rt_array_new(0);
            DraconicValue *p_empty = draconic_rt_promise_all(empty);
            (void)draconic_rt_promise_then(p_empty, on_empty, NULL, NULL, NULL);

            DraconicValue *a = draconic_rt_array_new(2);
            DraconicValue *p0 = draconic_rt_promise_new();
            DraconicValue *p1 = draconic_rt_promise_new();
            draconic_rt_promise_resolve(p0, (void *)(intptr_t)10);
            draconic_rt_promise_resolve(p1, (void *)(intptr_t)20);
            draconic_rt_array_set(a, 0, p0);
            draconic_rt_array_set(a, 1, p1);
            DraconicValue *p_all = draconic_rt_promise_all(a);
            (void)draconic_rt_promise_then(p_all, on_all, NULL, NULL, NULL);

            DraconicValue *m = draconic_rt_array_new(2);
            draconic_rt_array_set(m, 0, (void *)(intptr_t)1);
            DraconicValue *pm = draconic_rt_promise_new();
            draconic_rt_promise_resolve(pm, (void *)(intptr_t)2);
            draconic_rt_array_set(m, 1, pm);
            DraconicValue *p_mixed = draconic_rt_promise_all(m);
            (void)draconic_rt_promise_then(p_mixed, on_mixed, NULL, NULL, NULL);

            DraconicValue *r = draconic_rt_array_new(2);
            DraconicValue *ok = draconic_rt_promise_new();
            DraconicValue *bad = draconic_rt_promise_new();
            draconic_rt_promise_resolve(ok, (void *)(intptr_t)1);
            draconic_rt_promise_reject(bad, (void *)(intptr_t)7);
            draconic_rt_array_set(r, 0, ok);
            draconic_rt_array_set(r, 1, bad);
            DraconicValue *p_rej = draconic_rt_promise_all(r);
            (void)draconic_rt_promise_then(p_rej, on_reject_ok, NULL, on_reject_err, NULL);

            draconic_rt_job_drain();

            if (g_empty_len != 0) {
                fprintf(stderr, "emptyLen want 0 got %d\n", g_empty_len);
                return 1;
            }
            if (g_all_len != 2 || g_a0 != 10 || g_a1 != 20) {
                fprintf(stderr, "all want 2,10,20 got %d,%d,%d\n", g_all_len, g_a0, g_a1);
                return 2;
            }
            if (g_mixed0 != 1 || g_mixed1 != 2) {
                fprintf(stderr, "mixed want 1,2 got %d,%d\n", g_mixed0, g_mixed1);
                return 3;
            }
            if (g_rejected != 7) {
                fprintf(stderr, "rejected want 7 got %d\n", g_rejected);
                return 4;
            }

            draconic_rt_print_str("promise-all-ok");
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
    assert!(status.success(), "clang failed to link promise all test");

    let output = Command::new(&bin).output().expect("run rt_promise_all");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "promise all binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "promise-all-ok\n", "stdout={stdout:?}");
}

#[test]
fn promise_race_via_job_queue() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_promise_race");
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

        static int g_winner = -1;
        static int g_mixed = -1;
        static int g_rejected = 0;

        static void *on_winner(void *data, void *value) {
            (void)data;
            g_winner = (int)(intptr_t)value;
            return value;
        }

        static void *on_mixed(void *data, void *value) {
            (void)data;
            g_mixed = (int)(intptr_t)value;
            return value;
        }

        static void *on_reject_ok(void *data, void *value) {
            (void)data; (void)value;
            g_rejected = -1;
            return value;
        }

        static void *on_reject_err(void *data, void *reason) {
            (void)data;
            g_rejected = (int)(intptr_t)reason;
            return reason;
        }

        int main(void) {
            DraconicValue *a = draconic_rt_array_new(2);
            DraconicValue *p0 = draconic_rt_promise_new();
            DraconicValue *p1 = draconic_rt_promise_new();
            draconic_rt_promise_resolve(p0, (void *)(intptr_t)10);
            draconic_rt_promise_resolve(p1, (void *)(intptr_t)20);
            draconic_rt_array_set(a, 0, p0);
            draconic_rt_array_set(a, 1, p1);
            DraconicValue *p_race = draconic_rt_promise_race(a);
            (void)draconic_rt_promise_then(p_race, on_winner, NULL, NULL, NULL);

            DraconicValue *m = draconic_rt_array_new(2);
            draconic_rt_array_set(m, 0, (void *)(intptr_t)1);
            DraconicValue *pm = draconic_rt_promise_new();
            draconic_rt_promise_resolve(pm, (void *)(intptr_t)2);
            draconic_rt_array_set(m, 1, pm);
            DraconicValue *p_mixed = draconic_rt_promise_race(m);
            (void)draconic_rt_promise_then(p_mixed, on_mixed, NULL, NULL, NULL);

            DraconicValue *r = draconic_rt_array_new(2);
            DraconicValue *bad = draconic_rt_promise_new();
            DraconicValue *ok = draconic_rt_promise_new();
            draconic_rt_promise_reject(bad, (void *)(intptr_t)7);
            draconic_rt_promise_resolve(ok, (void *)(intptr_t)1);
            draconic_rt_array_set(r, 0, bad);
            draconic_rt_array_set(r, 1, ok);
            DraconicValue *p_rej = draconic_rt_promise_race(r);
            (void)draconic_rt_promise_then(p_rej, on_reject_ok, NULL, on_reject_err, NULL);

            draconic_rt_job_drain();

            if (g_winner != 10) {
                fprintf(stderr, "winner want 10 got %d\n", g_winner);
                return 1;
            }
            if (g_mixed != 1) {
                fprintf(stderr, "mixed want 1 got %d\n", g_mixed);
                return 2;
            }
            if (g_rejected != 7) {
                fprintf(stderr, "rejected want 7 got %d\n", g_rejected);
                return 3;
            }

            draconic_rt_print_str("promise-race-ok");
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
    assert!(status.success(), "clang failed to link promise race test");

    let output = Command::new(&bin).output().expect("run rt_promise_race");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "promise race binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "promise-race-ok\n", "stdout={stdout:?}");
}

#[test]
fn promise_all_settled_via_job_queue() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_promise_all_settled");
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
        #include <string.h>

        static int g_empty_len = -1;
        static int g_settled_len = -1;
        static const char *g_s0 = NULL;
        static int g_v0 = -1;
        static const char *g_s1 = NULL;
        static int g_r1 = -1;
        static const char *g_m0 = NULL;
        static int g_mv0 = -1;
        static const char *g_m1 = NULL;
        static int g_mv1 = -1;

        static void *on_empty(void *data, void *value) {
            (void)data;
            g_empty_len = (int)draconic_rt_array_len((DraconicValue *)value);
            return value;
        }

        static void *on_settled(void *data, void *value) {
            (void)data;
            DraconicValue *arr = (DraconicValue *)value;
            g_settled_len = (int)draconic_rt_array_len(arr);
            DraconicValue *e0 = (DraconicValue *)draconic_rt_array_get(arr, 0);
            DraconicValue *e1 = (DraconicValue *)draconic_rt_array_get(arr, 1);
            g_s0 = (const char *)draconic_rt_object_get(e0, "status");
            g_v0 = (int)(intptr_t)draconic_rt_object_get(e0, "value");
            g_s1 = (const char *)draconic_rt_object_get(e1, "status");
            g_r1 = (int)(intptr_t)draconic_rt_object_get(e1, "reason");
            return value;
        }

        static void *on_mixed(void *data, void *value) {
            (void)data;
            DraconicValue *arr = (DraconicValue *)value;
            DraconicValue *e0 = (DraconicValue *)draconic_rt_array_get(arr, 0);
            DraconicValue *e1 = (DraconicValue *)draconic_rt_array_get(arr, 1);
            g_m0 = (const char *)draconic_rt_object_get(e0, "status");
            g_mv0 = (int)(intptr_t)draconic_rt_object_get(e0, "value");
            g_m1 = (const char *)draconic_rt_object_get(e1, "status");
            g_mv1 = (int)(intptr_t)draconic_rt_object_get(e1, "value");
            return value;
        }

        int main(void) {
            DraconicValue *empty = draconic_rt_array_new(0);
            DraconicValue *p_empty = draconic_rt_promise_all_settled(empty);
            (void)draconic_rt_promise_then(p_empty, on_empty, NULL, NULL, NULL);

            DraconicValue *a = draconic_rt_array_new(2);
            DraconicValue *p0 = draconic_rt_promise_new();
            DraconicValue *p1 = draconic_rt_promise_new();
            draconic_rt_promise_resolve(p0, (void *)(intptr_t)10);
            draconic_rt_promise_reject(p1, (void *)(intptr_t)7);
            draconic_rt_array_set(a, 0, p0);
            draconic_rt_array_set(a, 1, p1);
            DraconicValue *p_set = draconic_rt_promise_all_settled(a);
            (void)draconic_rt_promise_then(p_set, on_settled, NULL, NULL, NULL);

            DraconicValue *m = draconic_rt_array_new(2);
            draconic_rt_array_set(m, 0, (void *)(intptr_t)1);
            DraconicValue *pm = draconic_rt_promise_new();
            draconic_rt_promise_resolve(pm, (void *)(intptr_t)2);
            draconic_rt_array_set(m, 1, pm);
            DraconicValue *p_mixed = draconic_rt_promise_all_settled(m);
            (void)draconic_rt_promise_then(p_mixed, on_mixed, NULL, NULL, NULL);

            draconic_rt_job_drain();

            if (g_empty_len != 0) {
                fprintf(stderr, "emptyLen want 0 got %d\n", g_empty_len);
                return 1;
            }
            if (g_settled_len != 2) {
                fprintf(stderr, "settledLen want 2 got %d\n", g_settled_len);
                return 2;
            }
            if (!g_s0 || strcmp(g_s0, "fulfilled") != 0 || g_v0 != 10) {
                fprintf(stderr, "s0/v0 bad: %s %d\n", g_s0 ? g_s0 : "(null)", g_v0);
                return 3;
            }
            if (!g_s1 || strcmp(g_s1, "rejected") != 0 || g_r1 != 7) {
                fprintf(stderr, "s1/r1 bad: %s %d\n", g_s1 ? g_s1 : "(null)", g_r1);
                return 4;
            }
            if (!g_m0 || strcmp(g_m0, "fulfilled") != 0 || g_mv0 != 1) {
                fprintf(stderr, "mixed0 bad: %s %d\n", g_m0 ? g_m0 : "(null)", g_mv0);
                return 5;
            }
            if (!g_m1 || strcmp(g_m1, "fulfilled") != 0 || g_mv1 != 2) {
                fprintf(stderr, "mixed1 bad: %s %d\n", g_m1 ? g_m1 : "(null)", g_mv1);
                return 6;
            }

            draconic_rt_print_str("promise-all-settled-ok");
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
        "clang failed to link promise allSettled test"
    );

    let output = Command::new(&bin)
        .output()
        .expect("run rt_promise_all_settled");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "promise allSettled binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "promise-all-settled-ok\n", "stdout={stdout:?}");
}

#[test]
fn promise_any_via_job_queue() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_promise_any");
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
        #include <string.h>

        static int g_winner = -1;
        static int g_mixed = -1;
        static int g_all_rej = 0;
        static const char *g_err_name = NULL;
        static int g_err_len = -1;
        static int g_empty_rej = 0;
        static const char *g_empty_name = NULL;
        static int g_empty_len = -1;

        static void *on_winner(void *data, void *value) {
            (void)data;
            g_winner = (int)(intptr_t)value;
            return value;
        }

        static void *on_mixed(void *data, void *value) {
            (void)data;
            g_mixed = (int)(intptr_t)value;
            return value;
        }

        static void *on_all_rej_ok(void *data, void *value) {
            (void)data; (void)value;
            g_all_rej = -1;
            return value;
        }

        static void *on_all_rej_err(void *data, void *reason) {
            (void)data;
            g_all_rej = 1;
            DraconicValue *e = (DraconicValue *)reason;
            g_err_name = (const char *)draconic_rt_object_get(e, "name");
            DraconicValue *errs = (DraconicValue *)draconic_rt_object_get(e, "errors");
            g_err_len = (int)draconic_rt_array_len(errs);
            return reason;
        }

        static void *on_empty_ok(void *data, void *value) {
            (void)data; (void)value;
            g_empty_rej = -1;
            return value;
        }

        static void *on_empty_err(void *data, void *reason) {
            (void)data;
            g_empty_rej = 1;
            DraconicValue *e = (DraconicValue *)reason;
            g_empty_name = (const char *)draconic_rt_object_get(e, "name");
            DraconicValue *errs = (DraconicValue *)draconic_rt_object_get(e, "errors");
            g_empty_len = (int)draconic_rt_array_len(errs);
            return reason;
        }

        int main(void) {
            DraconicValue *a = draconic_rt_array_new(2);
            DraconicValue *p0 = draconic_rt_promise_new();
            DraconicValue *p1 = draconic_rt_promise_new();
            draconic_rt_promise_resolve(p0, (void *)(intptr_t)10);
            draconic_rt_promise_resolve(p1, (void *)(intptr_t)20);
            draconic_rt_array_set(a, 0, p0);
            draconic_rt_array_set(a, 1, p1);
            DraconicValue *p_win = draconic_rt_promise_any(a);
            (void)draconic_rt_promise_then(p_win, on_winner, NULL, NULL, NULL);

            DraconicValue *m = draconic_rt_array_new(2);
            draconic_rt_array_set(m, 0, (void *)(intptr_t)1);
            DraconicValue *pm = draconic_rt_promise_new();
            draconic_rt_promise_resolve(pm, (void *)(intptr_t)2);
            draconic_rt_array_set(m, 1, pm);
            DraconicValue *p_mixed = draconic_rt_promise_any(m);
            (void)draconic_rt_promise_then(p_mixed, on_mixed, NULL, NULL, NULL);

            DraconicValue *r = draconic_rt_array_new(2);
            DraconicValue *r0 = draconic_rt_promise_new();
            DraconicValue *r1 = draconic_rt_promise_new();
            draconic_rt_promise_reject(r0, (void *)(intptr_t)7);
            draconic_rt_promise_reject(r1, (void *)(intptr_t)9);
            draconic_rt_array_set(r, 0, r0);
            draconic_rt_array_set(r, 1, r1);
            DraconicValue *p_rej = draconic_rt_promise_any(r);
            (void)draconic_rt_promise_then(p_rej, on_all_rej_ok, NULL, on_all_rej_err, NULL);

            DraconicValue *empty = draconic_rt_array_new(0);
            DraconicValue *p_empty = draconic_rt_promise_any(empty);
            (void)draconic_rt_promise_then(p_empty, on_empty_ok, NULL, on_empty_err, NULL);

            draconic_rt_job_drain();

            if (g_winner != 10) {
                fprintf(stderr, "winner want 10 got %d\n", g_winner);
                return 1;
            }
            if (g_mixed != 1) {
                fprintf(stderr, "mixed want 1 got %d\n", g_mixed);
                return 2;
            }
            if (g_all_rej != 1) {
                fprintf(stderr, "allRejected want 1 got %d\n", g_all_rej);
                return 3;
            }
            if (!g_err_name || strcmp(g_err_name, "AggregateError") != 0 || g_err_len != 2) {
                fprintf(stderr, "err bad: %s %d\n", g_err_name ? g_err_name : "(null)", g_err_len);
                return 4;
            }
            if (g_empty_rej != 1) {
                fprintf(stderr, "emptyRejected want 1 got %d\n", g_empty_rej);
                return 5;
            }
            if (!g_empty_name || strcmp(g_empty_name, "AggregateError") != 0 || g_empty_len != 0) {
                fprintf(stderr, "empty err bad: %s %d\n",
                    g_empty_name ? g_empty_name : "(null)", g_empty_len);
                return 6;
            }

            draconic_rt_print_str("promise-any-ok");
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
    assert!(status.success(), "clang failed to link promise any test");

    let output = Command::new(&bin).output().expect("run rt_promise_any");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "promise any binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "promise-any-ok\n", "stdout={stdout:?}");
}
