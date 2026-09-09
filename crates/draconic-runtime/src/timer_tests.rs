//! Timers, drain-until-due, sleep, and yield native tests.

use super::*;
use std::process::Command;

#[test]
fn timer_set_clear_via_job_drain() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_timer.c");
    let bin = dir.join("rt_timer");
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

        static int g_fired;
        static int g_cancelled;
        static int g_nested;

        static void on_nested(void *data) {
            (void)data;
            g_nested = 1;
        }

        static void on_outer(void *data) {
            (void)data;
            draconic_rt_timer_set(on_nested, NULL, 0.0);
        }

        static void on_fire(void *data) {
            (void)data;
            g_fired = 1;
        }

        static void on_cancel(void *data) {
            (void)data;
            g_cancelled = 1;
        }

        int main(void) {
            int64_t id_fire = draconic_rt_timer_set(on_fire, NULL, 0.0);
            int64_t id_cancel = draconic_rt_timer_set(on_cancel, NULL, 0.0);
            if (id_fire <= 0 || id_cancel <= 0) {
                fprintf(stderr, "timer ids invalid\n");
                return 1;
            }
            draconic_rt_timer_clear(id_cancel);
            draconic_rt_timer_set(on_outer, NULL, 0.0);
            draconic_rt_job_drain();
            if (g_fired != 1) {
                fprintf(stderr, "fired want 1 got %d\n", g_fired);
                return 2;
            }
            if (g_cancelled != 0) {
                fprintf(stderr, "cancelled want 0 got %d\n", g_cancelled);
                return 3;
            }
            if (g_nested != 1) {
                fprintf(stderr, "nested want 1 got %d\n", g_nested);
                return 4;
            }
            puts("timer-ok");
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
    assert!(status.success(), "clang failed to link timer test");

    let output = Command::new(&bin).output().expect("run rt_timer");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "timer binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "timer-ok\n", "stdout={stdout:?}");
}

#[test]
fn timer_set_interval_clear_via_job_drain() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_interval.c");
    let bin = dir.join("rt_interval");
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

        static int g_ticks;
        static int g_cancelled;
        static int64_t g_id;

        static void on_tick(void *data) {
            (void)data;
            g_ticks++;
            if (g_ticks >= 3) {
                draconic_rt_timer_clear(g_id);
            }
        }

        static void on_cancel(void *data) {
            (void)data;
            g_cancelled = 1;
        }

        int main(void) {
            g_id = draconic_rt_timer_set_interval(on_tick, NULL, 0.0);
            int64_t cid = draconic_rt_timer_set_interval(on_cancel, NULL, 0.0);
            if (g_id <= 0 || cid <= 0) {
                fprintf(stderr, "interval ids invalid\n");
                return 1;
            }
            draconic_rt_timer_clear(cid);
            draconic_rt_job_drain();
            if (g_ticks != 3) {
                fprintf(stderr, "ticks want 3 got %d\n", g_ticks);
                return 2;
            }
            if (g_cancelled != 0) {
                fprintf(stderr, "cancelled want 0 got %d\n", g_cancelled);
                return 3;
            }
            puts("interval-ok");
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
    assert!(status.success(), "clang failed to link interval test");

    let output = Command::new(&bin).output().expect("run rt_interval");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "interval binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "interval-ok\n", "stdout={stdout:?}");
}

/// H05.05: job_drain waits for a future timer (OS sleep), fires it, and
/// does not return early while the timer is still pending.
#[test]
fn timer_drain_waits_for_future_due() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_timer_wait.c");
    let bin = dir.join("rt_timer_wait");
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
#if defined(_WIN32)
        #include <windows.h>
        static double wall_ms(void) {
            FILETIME ft;
            ULARGE_INTEGER u;
            const uint64_t epoch_diff_100ns = 116444736000000000ULL;
            GetSystemTimeAsFileTime(&ft);
            u.LowPart = ft.dwLowDateTime;
            u.HighPart = ft.dwHighDateTime;
            if (u.QuadPart < epoch_diff_100ns) return 0.0;
            return (double)((u.QuadPart - epoch_diff_100ns) / 10000ULL);
        }
#else
        #include <sys/time.h>
        static double wall_ms(void) {
            struct timeval tv;
            if (gettimeofday(&tv, NULL) != 0) return 0.0;
            return ((double)tv.tv_sec * 1000.0) + ((double)tv.tv_usec / 1000.0);
        }
#endif

        static int g_fired;

        static void on_fire(void *data) {
            (void)data;
            g_fired = 1;
        }

        int main(void) {
            const double delay_ms = 40.0;
            double t0 = wall_ms();
            int64_t id = draconic_rt_timer_set(on_fire, NULL, delay_ms);
            if (id <= 0) {
                fprintf(stderr, "timer id invalid\n");
                return 1;
            }
            draconic_rt_job_drain();
            double elapsed = wall_ms() - t0;
            if (g_fired != 1) {
                fprintf(stderr, "fired want 1 got %d\n", g_fired);
                return 2;
            }
            /* Must have waited ~delay (not return immediately). */
            if (elapsed < 25.0) {
                fprintf(stderr, "elapsed too small: %g (busy-return?)\n", elapsed);
                return 3;
            }
            /* Must not busy-spin for seconds. */
            if (elapsed > 2000.0) {
                fprintf(stderr, "elapsed too large: %g (spin?)\n", elapsed);
                return 4;
            }
            puts("timer-wait-ok");
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
    assert!(status.success(), "clang failed to link timer wait test");

    let output = Command::new(&bin).output().expect("run rt_timer_wait");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "timer wait binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "timer-wait-ok\n", "stdout={stdout:?}");
}

/// H16.04: public OS sleep / yield for timer tests — sleep blocks ~ms; yield returns;
/// non-positive / NaN sleep is a no-op.
#[test]
fn sleep_ms_and_yield_os() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_sleep_yield.c");
    let bin = dir.join("rt_sleep_yield");
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
#if defined(_WIN32)
        #include <windows.h>
        static double wall_ms(void) {
            FILETIME ft;
            ULARGE_INTEGER u;
            const uint64_t epoch_diff_100ns = 116444736000000000ULL;
            GetSystemTimeAsFileTime(&ft);
            u.LowPart = ft.dwLowDateTime;
            u.HighPart = ft.dwHighDateTime;
            if (u.QuadPart < epoch_diff_100ns) return 0.0;
            return (double)((u.QuadPart - epoch_diff_100ns) / 10000ULL);
        }
#else
        #include <sys/time.h>
        static double wall_ms(void) {
            struct timeval tv;
            if (gettimeofday(&tv, NULL) != 0) return 0.0;
            return ((double)tv.tv_sec * 1000.0) + ((double)tv.tv_usec / 1000.0);
        }
#endif

        int main(void) {
            double t0, elapsed;

            /* yield must return promptly */
            t0 = wall_ms();
            draconic_rt_yield();
            elapsed = wall_ms() - t0;
            if (elapsed > 500.0) {
                fprintf(stderr, "yield too slow: %g\n", elapsed);
                return 1;
            }

            /* sleep ~40ms blocks at least ~25ms and not seconds */
            t0 = wall_ms();
            draconic_rt_sleep_ms(40.0);
            elapsed = wall_ms() - t0;
            if (elapsed < 25.0) {
                fprintf(stderr, "sleep elapsed too small: %g\n", elapsed);
                return 2;
            }
            if (elapsed > 2000.0) {
                fprintf(stderr, "sleep elapsed too large: %g\n", elapsed);
                return 3;
            }

            /* non-positive / NaN: immediate return */
            t0 = wall_ms();
            draconic_rt_sleep_ms(0.0);
            draconic_rt_sleep_ms(-1.0);
            draconic_rt_sleep_ms(0.0 / 0.0);
            elapsed = wall_ms() - t0;
            if (elapsed > 100.0) {
                fprintf(stderr, "noop sleep too slow: %g\n", elapsed);
                return 4;
            }

            puts("sleep-yield-ok");
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
    assert!(status.success(), "clang failed to link sleep/yield test");

    let output = Command::new(&bin).output().expect("run rt_sleep_yield");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "sleep/yield binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "sleep-yield-ok\n", "stdout={stdout:?}");
}
