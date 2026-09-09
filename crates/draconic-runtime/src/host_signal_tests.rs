//! Host signal watch, ignore, restore, and raise ABI tests.

use super::*;
use std::process::Command;

#[test]
fn host_signal_watch_raise_via_job_drain() {
    // H14.01: watch + raise → poll enqueues handler; job_drain runs it.
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_signal_watch.c");
    let bin = dir.join("rt_host_signal_watch");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt.h"
#include "draconic_rt_host.h"
#include <stdio.h>
static int g_fired;
static void on_term(void *data) {
    (void)data;
    g_fired = 1;
}
int main(void) {
    if (draconic_rt_host_signal_watch(DRACONIC_HOST_SIG_TERM, on_term, NULL) != DRACONIC_HOST_OK)
        return 1;
    if (draconic_rt_host_signal_raise(DRACONIC_HOST_SIG_TERM) != DRACONIC_HOST_OK)
        return 2;
    draconic_rt_job_drain();
    if (g_fired != 1) return 3;
    printf("%d\n", g_fired);
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
    assert_eq!(String::from_utf8_lossy(&out.stdout), "1\n");
}

#[cfg(unix)]

#[test]
fn host_signal_default_terminate_without_watch() {
    // H14.01 default: no watch → OS SIG_DFL terminate on SIGTERM (subprocess).
    use std::os::unix::process::ExitStatusExt;
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_signal_default.c");
    let bin = dir.join("rt_host_signal_default");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
int main(void) {
    /* No onSignal/watch: raise SIGTERM → default terminate. */
    (void)draconic_rt_host_signal_raise(DRACONIC_HOST_SIG_TERM);
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
        !out.status.success(),
        "expected terminate; stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let sig = out.status.signal();
    assert_eq!(
        sig,
        Some(15),
        "SIGTERM; status={:?} sig={sig:?}",
        out.status
    );
}

#[cfg(unix)]

#[test]
fn host_signal_ignore_survives_raise() {
    // H14.02: ignore → raise does not terminate; no handler job.
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_signal_ignore.c");
    let bin = dir.join("rt_host_signal_ignore");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt.h"
#include "draconic_rt_host.h"
#include <stdio.h>
static int g_fired;
static void on_term(void *data) {
    (void)data;
    g_fired = 1;
}
int main(void) {
    if (draconic_rt_host_signal_watch(DRACONIC_HOST_SIG_TERM, on_term, NULL) != DRACONIC_HOST_OK)
        return 1;
    if (draconic_rt_host_signal_ignore(DRACONIC_HOST_SIG_TERM) != DRACONIC_HOST_OK)
        return 2;
    if (draconic_rt_host_signal_raise(DRACONIC_HOST_SIG_TERM) != DRACONIC_HOST_OK)
        return 3;
    draconic_rt_job_drain();
    if (g_fired != 0) return 4;
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

#[cfg(unix)]

#[test]
fn host_signal_restore_default_terminates() {
    // H14.02: ignore then restore → raise terminates (SIG_DFL).
    use std::os::unix::process::ExitStatusExt;
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_signal_restore.c");
    let bin = dir.join("rt_host_signal_restore");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
int main(void) {
    if (draconic_rt_host_signal_ignore(DRACONIC_HOST_SIG_TERM) != DRACONIC_HOST_OK)
        return 1;
    if (draconic_rt_host_signal_restore(DRACONIC_HOST_SIG_TERM) != DRACONIC_HOST_OK)
        return 2;
    (void)draconic_rt_host_signal_raise(DRACONIC_HOST_SIG_TERM);
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
        !out.status.success(),
        "expected terminate; stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let sig = out.status.signal();
    assert_eq!(
        sig,
        Some(15),
        "SIGTERM; status={:?} sig={sig:?}",
        out.status
    );
}
