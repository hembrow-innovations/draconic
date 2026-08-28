//! C01.04: OS thread backing for native workers.

use super::*;
use std::process::Command;

#[test]
fn host_worker_os_thread_declare() {
    assert_eq!(
        HOST_WORKER_OS_THREAD.declare(),
        "declare i32 @draconic_rt_host_worker_os_thread(i32)"
    );
}

#[test]
fn host_worker_spawn_runs_on_distinct_os_thread() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_worker_os");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt_host.h"
        #include <stdio.h>

        int main(void) {
            int32_t h = draconic_rt_host_worker_spawn(0, NULL);
            if (h < 1) {
                fprintf(stderr, "spawn\n");
                return 1;
            }
            int32_t os = draconic_rt_host_worker_os_thread(h);
            if (os != 1) {
                fprintf(stderr, "os_thread want 1 got %d\n", (int)os);
                return 2;
            }
            int32_t r = draconic_rt_host_worker_join(h);
            if (r != 0) {
                fprintf(stderr, "join %d\n", (int)r);
                return 3;
            }
            int32_t after = draconic_rt_host_worker_os_thread(h);
            if (after != -1) {
                fprintf(stderr, "after want -1 got %d\n", (int)after);
                return 4;
            }
            int32_t h2 = draconic_rt_host_worker_spawn(0, NULL);
            if (h2 < 1) return 5;
            if (draconic_rt_host_worker_os_thread(h2) != 1) return 6;
            if (draconic_rt_host_worker_terminate(h2) != 0) return 7;
            if (draconic_rt_host_worker_os_thread(h2) != -1) return 8;
            if (draconic_rt_host_worker_join(h2) != -1) return 9;
            if (draconic_rt_host_worker_os_thread(0) != -1) return 10;
            puts("worker-os-thread-ok");
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
    assert!(status.success(), "clang failed for worker C01.04 smoke");

    let output = Command::new(&bin).output().expect("run worker os thread");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "worker C01.04 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "worker-os-thread-ok\n"
    );
}
