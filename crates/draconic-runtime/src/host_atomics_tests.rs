//! C06: shared-memory atomics integer buffer.

use super::*;
use std::process::Command;

#[test]
fn host_shared_make_declare() {
    assert_eq!(
        HOST_SHARED_MAKE.declare(),
        "declare i32 @draconic_rt_host_shared_make(i32)"
    );
}

#[test]
fn host_shared_wait_declare() {
    assert_eq!(
        HOST_SHARED_WAIT.declare(),
        "declare i32 @draconic_rt_host_shared_wait(i32, i32, i32, double)"
    );
}

#[test]
fn host_shared_ops_wait_notify() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_shared");
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

        static int32_t g_mem;

        static void *waiter(void *arg) {
            (void)arg;
            int32_t r = draconic_rt_host_shared_wait(g_mem, 0, 0, 2000.0);
            return (void *)(intptr_t)r;
        }

        int main(void) {
            int32_t m = draconic_rt_host_shared_make(2);
            if (m < 1) return 1;
            if (draconic_rt_host_shared_store(m, 0, 7) != 0) return 2;
            if (draconic_rt_host_shared_load(m, 0) != 7) return 3;
            if (draconic_rt_host_shared_add(m, 0, 3) != 7) return 4;
            if (draconic_rt_host_shared_load(m, 0) != 10) return 5;
            if (draconic_rt_host_shared_cmpxchg(m, 0, 10, 42) != 10) return 6;
            if (draconic_rt_host_shared_cmpxchg(m, 0, 10, 99) != 42) return 7;
            if (draconic_rt_host_shared_load(m, 0) != 42) return 8;
            if (draconic_rt_host_shared_store(0, 0, 1) != -1) return 9;
            if (draconic_rt_host_shared_wait(m, 0, 7, 1.0) != 1) return 10;
            if (draconic_rt_host_shared_store(m, 0, 7) != 0) return 11;
            if (draconic_rt_host_shared_wait(m, 0, 7, 1.0) != 2) return 12;
            if (draconic_rt_host_shared_notify(m, 0) != 0) return 13;
            if (draconic_rt_host_shared_wait(0, 0, 0, 1.0) != -1) return 14;

            g_mem = draconic_rt_host_shared_make(1);
            if (g_mem < 1) return 15;
            if (draconic_rt_host_shared_store(g_mem, 0, 0) != 0) return 16;
            pthread_t th;
            if (pthread_create(&th, NULL, waiter, NULL) != 0) return 17;
            while (draconic_rt_host_shared_notify(g_mem, 0) == 0) {
            }
            if (draconic_rt_host_shared_store(g_mem, 0, 1) != 0) return 18;
            if (draconic_rt_host_shared_notify(g_mem, 0) < 0) return 19;
            void *ret = NULL;
            if (pthread_join(th, &ret) != 0) return 20;
            int32_t wr = (int32_t)(intptr_t)ret;
            if (wr != 0 && wr != 1) return 21;
            puts("shared-ok");
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
    assert!(status.success(), "clang failed for shared C06 smoke");

    let output = Command::new(&bin).output().expect("run shared");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "shared C06 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "shared-ok\n");
}
