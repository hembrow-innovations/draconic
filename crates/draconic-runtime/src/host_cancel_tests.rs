//! C05.01: cancel token / Abort-like signal.
//! C05.02: withTimeout / clearWithTimeout race vs timer.

use super::*;
use std::process::Command;

#[test]
fn host_cancel_make_declare() {
    assert_eq!(
        HOST_CANCEL_MAKE.declare(),
        "declare i32 @draconic_rt_host_cancel_make()"
    );
}

#[test]
fn host_cancel_abort_declare() {
    assert_eq!(
        HOST_CANCEL_ABORT.declare(),
        "declare i32 @draconic_rt_host_cancel_abort(i32)"
    );
}

#[test]
fn host_cancel_aborted_declare() {
    assert_eq!(
        HOST_CANCEL_ABORTED.declare(),
        "declare i32 @draconic_rt_host_cancel_aborted(i32)"
    );
}

#[test]
fn host_cancel_link_declare() {
    assert_eq!(
        HOST_CANCEL_LINK.declare(),
        "declare i32 @draconic_rt_host_cancel_link(i32, i32)"
    );
}

#[test]
fn host_cancel_timeout_declare() {
    assert_eq!(
        HOST_CANCEL_TIMEOUT.declare(),
        "declare i32 @draconic_rt_host_cancel_timeout(double)"
    );
}

#[test]
fn host_cancel_clear_timeout_declare() {
    assert_eq!(
        HOST_CANCEL_CLEAR_TIMEOUT.declare(),
        "declare i32 @draconic_rt_host_cancel_clear_timeout(i32)"
    );
}

#[test]
fn host_cancel_abort_and_link() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_cancel");
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
            int32_t tok = draconic_rt_host_cancel_make();
            if (tok < 1) return 1;
            if (draconic_rt_host_cancel_aborted(tok) != 0) return 2;
            if (draconic_rt_host_cancel_abort(tok) != 0) return 3;
            if (draconic_rt_host_cancel_aborted(tok) != 1) return 4;
            if (draconic_rt_host_cancel_abort(tok) != 0) return 5;
            if (draconic_rt_host_cancel_aborted(0) != -1) return 6;
            if (draconic_rt_host_cancel_abort(0) != -1) return 7;

            int32_t parent = draconic_rt_host_cancel_make();
            int32_t child = draconic_rt_host_cancel_make();
            int32_t other = draconic_rt_host_cancel_make();
            if (parent < 1 || child < 1 || other < 1) return 8;
            if (draconic_rt_host_cancel_link(child, parent) != 0) return 9;
            if (draconic_rt_host_cancel_abort(parent) != 0) return 10;
            if (draconic_rt_host_cancel_aborted(child) != 1) return 11;
            if (draconic_rt_host_cancel_aborted(other) != 0) return 12;

            int32_t already = draconic_rt_host_cancel_make();
            int32_t late = draconic_rt_host_cancel_make();
            if (already < 1 || late < 1) return 13;
            if (draconic_rt_host_cancel_abort(already) != 0) return 14;
            if (draconic_rt_host_cancel_link(late, already) != 0) return 15;
            if (draconic_rt_host_cancel_aborted(late) != 1) return 16;
            if (draconic_rt_host_cancel_link(0, parent) != -1) return 17;

            puts("cancel-ok");
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
    assert!(status.success(), "clang failed for cancel C05.01 smoke");

    let output = Command::new(&bin).output().expect("run cancel");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "cancel C05.01 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "cancel-ok\n");
}

#[test]
fn host_cancel_timeout_race() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_cancel_timeout");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt_host.h"
        #include "draconic_rt.h"
        #include <stdio.h>

        int main(void) {
            int32_t fire = draconic_rt_host_cancel_timeout(0.0);
            if (fire < 1) return 1;
            if (draconic_rt_host_cancel_aborted(fire) != 0) return 2;
            draconic_rt_job_drain();
            if (draconic_rt_host_cancel_aborted(fire) != 1) return 3;

            int32_t work = draconic_rt_host_cancel_timeout(0.0);
            if (work < 1) return 4;
            if (draconic_rt_host_cancel_clear_timeout(work) != 0) return 5;
            draconic_rt_job_drain();
            if (draconic_rt_host_cancel_aborted(work) != 0) return 6;
            if (draconic_rt_host_cancel_clear_timeout(work) != 0) return 7;
            if (draconic_rt_host_cancel_clear_timeout(0) != -1) return 8;

            puts("timeout-ok");
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
    assert!(status.success(), "clang failed for cancel C05.02 smoke");

    let output = Command::new(&bin).output().expect("run cancel timeout");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "cancel C05.02 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "timeout-ok\n");
}
