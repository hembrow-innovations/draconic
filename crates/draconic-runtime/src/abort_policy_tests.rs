//! R04: process abort / panic — fail-closed native faults kill the process.
//! Canonical `draconic_rt_abort` and Runtime invariant failures never become JS values.

use super::*;
use std::path::{Path, PathBuf};
use std::process::Command;

fn link_runtime_c(name: &str, source: &str) -> PathBuf {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join(name);
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(&main_c, source).unwrap();

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
    assert!(status.success(), "clang failed to link {name}");
    bin
}

fn assert_process_aborts(bin: &Path, stderr_needle: &str) {
    let output = Command::new(bin).output().expect("run abort harness");
    assert!(
        !output.status.success(),
        "abort-class fault must kill the process: {:?}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("after"),
        "abort must not return; stdout={stdout:?}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(stderr_needle),
        "abort should report {stderr_needle:?} on stderr; stderr={stderr:?}"
    );
}

#[test]
fn abort_declare() {
    assert_eq!(ABORT.declare(), "declare void @draconic_rt_abort()");
}

#[test]
fn abort_kills_process() {
    let bin = link_runtime_c(
        "rt_abort",
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
    );
    assert_process_aborts(&bin, "draconic_rt: abort");
}

#[test]
fn abort_emits_backtrace() {
    let bin = link_runtime_c(
        "rt_abort_backtrace",
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
    );
    let output = Command::new(&bin)
        .output()
        .expect("run abort backtrace harness");
    assert!(
        !output.status.success(),
        "abort-class fault must kill the process: {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("draconic_rt: abort"),
        "abort should report canonical message; stderr={stderr:?}"
    );
    assert!(
        stderr.contains("draconic_rt: backtrace"),
        "abort-class fault must emit a backtrace; stderr={stderr:?}"
    );
    let frame_lines = stderr
        .lines()
        .filter(|l| l.starts_with("  ") || l.contains("0x") || l.contains("main"))
        .count();
    assert!(
        frame_lines >= 1,
        "backtrace must include at least one frame; stderr={stderr:?}"
    );
}

#[test]
fn invariant_root_stack_underflow_aborts_process() {
    let bin = link_runtime_c(
        "rt_root_underflow",
        r#"
            #include "draconic_rt.h"
            #include <stdio.h>

            int main(void) {
                draconic_rt_gc_init();
                fprintf(stdout, "before\n");
                fflush(stdout);
                draconic_rt_gc_root_pop();
                fprintf(stdout, "after\n");
                return 0;
            }
            "#,
    );
    assert_process_aborts(&bin, "draconic_rt: root stack underflow");
}
