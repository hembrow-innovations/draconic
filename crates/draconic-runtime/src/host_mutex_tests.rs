//! C03.02: Runtime-internal mutex for shared tables (not user-facing shared heap).

use super::*;
use std::process::Command;

#[test]
fn host_internal_mutex_make_declare() {
    assert_eq!(
        HOST_INTERNAL_MUTEX_MAKE.declare(),
        "declare i32 @draconic_rt_host_internal_mutex_make()"
    );
}

#[test]
fn host_internal_mutex_lock_declare() {
    assert_eq!(
        HOST_INTERNAL_MUTEX_LOCK.declare(),
        "declare i32 @draconic_rt_host_internal_mutex_lock(i32)"
    );
}

#[test]
fn host_internal_mutex_unlock_declare() {
    assert_eq!(
        HOST_INTERNAL_MUTEX_UNLOCK.declare(),
        "declare i32 @draconic_rt_host_internal_mutex_unlock(i32)"
    );
}

#[test]
fn host_internal_mutex_serializes_and_protects_channel_worker_tables() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_mutex");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt_host.h"
        #include <pthread.h>
        #include <stdint.h>
        #include <stdio.h>

        #define NTH 8
        #define NINC 1000
        #define NSEND 200

        static int32_t g_mu;
        static int g_count = 0;
        static int32_t g_ch;
        static int32_t g_handles[NTH];

        static void *inc_worker(void *arg) {
            int i;
            (void)arg;
            for (i = 0; i < NINC; i++) {
                if (draconic_rt_host_internal_mutex_lock(g_mu) != 0) return (void *)1;
                g_count += 1;
                if (draconic_rt_host_internal_mutex_unlock(g_mu) != 0) return (void *)2;
            }
            return NULL;
        }

        static void *send_worker(void *arg) {
            int i;
            int32_t base = (int32_t)(intptr_t)arg;
            for (i = 0; i < NSEND; i++) {
                if (draconic_rt_host_channel_send_f64(g_ch, (double)(base * NSEND + i)) != 0) {
                    return (void *)3;
                }
            }
            return NULL;
        }

        static void *spawn_worker(void *arg) {
            int idx = (int)(intptr_t)arg;
            int32_t h = draconic_rt_host_worker_spawn(0, NULL);
            g_handles[idx] = h;
            return NULL;
        }

        int main(void) {
            pthread_t th[NTH];
            int i, j;
            double v;
            int seen;

            g_mu = draconic_rt_host_internal_mutex_make();
            if (g_mu < 1) {
                fprintf(stderr, "mutex make\n");
                return 1;
            }
            if (draconic_rt_host_internal_mutex_lock(0) != -1) return 2;
            if (draconic_rt_host_internal_mutex_unlock(0) != -1) return 3;
            if (draconic_rt_host_internal_mutex_lock(g_mu) != 0) return 4;
            if (draconic_rt_host_internal_mutex_unlock(g_mu) != 0) return 5;

            for (i = 0; i < NTH; i++) {
                if (pthread_create(&th[i], NULL, inc_worker, NULL) != 0) {
                    fprintf(stderr, "pthread inc\n");
                    return 6;
                }
            }
            for (i = 0; i < NTH; i++) {
                void *ret = NULL;
                pthread_join(th[i], &ret);
                if (ret != NULL) {
                    fprintf(stderr, "inc ret\n");
                    return 7;
                }
            }
            if (g_count != NTH * NINC) {
                fprintf(stderr, "count want %d got %d\n", NTH * NINC, g_count);
                return 8;
            }

            g_ch = draconic_rt_host_channel_make(0);
            if (g_ch < 1) {
                fprintf(stderr, "channel make\n");
                return 9;
            }
            for (i = 0; i < NTH; i++) {
                if (pthread_create(&th[i], NULL, send_worker, (void *)(intptr_t)i) != 0) {
                    fprintf(stderr, "pthread send\n");
                    return 10;
                }
            }
            for (i = 0; i < NTH; i++) {
                void *ret = NULL;
                pthread_join(th[i], &ret);
                if (ret != NULL) {
                    fprintf(stderr, "send ret\n");
                    return 11;
                }
            }
            seen = 0;
            while (draconic_rt_host_channel_recv_f64(g_ch, &v) == 0) {
                seen++;
            }
            if (seen != NTH * NSEND) {
                fprintf(stderr, "recv want %d got %d\n", NTH * NSEND, seen);
                return 12;
            }

            for (i = 0; i < NTH; i++) {
                g_handles[i] = 0;
                if (pthread_create(&th[i], NULL, spawn_worker, (void *)(intptr_t)i) != 0) {
                    fprintf(stderr, "pthread spawn\n");
                    return 13;
                }
            }
            for (i = 0; i < NTH; i++) {
                pthread_join(th[i], NULL);
            }
            for (i = 0; i < NTH; i++) {
                if (g_handles[i] < 1) {
                    fprintf(stderr, "spawn handle %d\n", i);
                    return 14;
                }
                for (j = i + 1; j < NTH; j++) {
                    if (g_handles[i] == g_handles[j]) {
                        fprintf(stderr, "dup handle %d\n", (int)g_handles[i]);
                        return 15;
                    }
                }
                if (draconic_rt_host_worker_join(g_handles[i]) != 0) {
                    fprintf(stderr, "join %d\n", (int)g_handles[i]);
                    return 16;
                }
            }

            puts("mutex-ok");
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
    assert!(status.success(), "clang failed for mutex C03.02 smoke");

    let output = Command::new(&bin).output().expect("run mutex");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "mutex C03.02 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "mutex-ok\n");
}
