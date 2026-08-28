//! C03.01: `once` / thread-safe init primitive.

use super::*;
use std::process::Command;

#[test]
fn host_once_make_declare() {
    assert_eq!(
        HOST_ONCE_MAKE.declare(),
        "declare i32 @draconic_rt_host_once_make()"
    );
}

#[test]
fn host_once_run_declare() {
    assert_eq!(
        HOST_ONCE_RUN.declare(),
        "declare i32 @draconic_rt_host_once_run(i32, ptr)"
    );
}

#[test]
fn host_once_run_once_and_concurrent() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_once");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt_host.h"
        #include <pthread.h>
        #include <stdio.h>

        static int g_count = 0;

        static void bump(void) {
            g_count += 1;
        }

        static void *worker(void *arg) {
            int32_t h = *(int32_t *)arg;
            (void)draconic_rt_host_once_run(h, bump);
            return NULL;
        }

        int main(void) {
            int32_t o = draconic_rt_host_once_make();
            if (o < 1) {
                fprintf(stderr, "make\n");
                return 1;
            }
            int32_t a = draconic_rt_host_once_run(o, bump);
            int32_t b = draconic_rt_host_once_run(o, bump);
            if (a != 1 || b != 0 || g_count != 1) {
                fprintf(stderr, "seq a=%d b=%d count=%d\n", (int)a, (int)b, g_count);
                return 2;
            }
            int32_t o2 = draconic_rt_host_once_make();
            if (o2 < 1 || o2 == o) {
                fprintf(stderr, "make2\n");
                return 3;
            }
            if (draconic_rt_host_once_run(o2, bump) != 1) return 4;
            if (g_count != 2) {
                fprintf(stderr, "independent count=%d\n", g_count);
                return 5;
            }
            if (draconic_rt_host_once_run(0, bump) != -1) return 6;
            if (draconic_rt_host_once_run(o, NULL) != 0) return 7;

            int32_t o3 = draconic_rt_host_once_make();
            if (o3 < 1) return 8;
            g_count = 0;
            pthread_t th[8];
            int i;
            for (i = 0; i < 8; i++) {
                if (pthread_create(&th[i], NULL, worker, &o3) != 0) {
                    fprintf(stderr, "pthread\n");
                    return 9;
                }
            }
            for (i = 0; i < 8; i++) {
                pthread_join(th[i], NULL);
            }
            if (g_count != 1) {
                fprintf(stderr, "concurrent count=%d\n", g_count);
                return 10;
            }
            puts("once-ok");
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
    assert!(status.success(), "clang failed for once C03.01 smoke");

    let output = Command::new(&bin).output().expect("run once");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "once C03.01 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "once-ok\n");
}
