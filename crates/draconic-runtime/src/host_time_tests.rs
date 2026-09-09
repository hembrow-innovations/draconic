//! Host wall-clock and monotonic time ABI tests.

use super::*;
use std::process::Command;

#[test]
fn host_now_ms_wall_clock() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_now_ms.c");
    let bin = dir.join("rt_host_now_ms");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
int main(void) {
    double a = draconic_rt_host_now_ms();
    double b = draconic_rt_host_now_ms();
    /* After 2020-09 and before year ~2096. */
    if (!(a > 1600000000000.0 && a < 4000000000000.0)) return 1;
    if (!(b >= a)) return 2;
    printf("ok\n");
    return 0;
}
"#,
    )
    .unwrap();
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg(format!("-I{}", header_dir.display()))
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("clang link")
    };
    assert!(status.success(), "link failed");
    let out = Command::new(&bin).output().expect("run");
    assert!(
        out.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ok\n");
}

#[test]
fn host_monotonic_ms_steady() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_monotonic_ms.c");
    let bin = dir.join("rt_host_monotonic_ms");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
int main(void) {
    double a = draconic_rt_host_monotonic_ms();
    double b = draconic_rt_host_monotonic_ms();
    if (!(a >= 0.0)) return 1;
    if (!(b >= a)) return 2;
    if (!((b - a) < 60000.0)) return 3;
    printf("ok\n");
    return 0;
}
"#,
    )
    .unwrap();
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    let status = {
        let mut link = Command::new(&clang);
        link.arg(&main_c)
            .arg(&archive)
            .arg(format!("-I{}", header_dir.display()))
            .arg("-o")
            .arg(&bin);
        apply_runtime_link_flags(&mut link);
        link.status().expect("clang link")
    };
    assert!(status.success(), "link failed");
    let out = Command::new(&bin).output().expect("run");
    assert!(
        out.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ok\n");
}
