//! R04.02: process abort / panic — fail-closed native faults kill the process.

use super::*;
use std::process::Command;

#[test]
fn abort_declare() {
    assert_eq!(ABORT.declare(), "declare void @draconic_rt_abort()");
}

#[test]
fn abort_kills_process() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_abort");
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
                fprintf(stdout, "before\n");
                fflush(stdout);
                draconic_rt_abort();
                fprintf(stdout, "after\n");
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
    assert!(status.success(), "clang failed to link abort test");

    let output = Command::new(&bin).output().expect("run rt_abort");
    assert!(
        !output.status.success(),
        "abort must kill the process: {:?}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("after"),
        "abort must not return; stdout={stdout:?}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("draconic_rt: abort"),
        "abort should report on stderr; stderr={stderr:?}"
    );
}
