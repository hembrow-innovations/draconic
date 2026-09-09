//! Host process, env, cwd, and OS identity ABI tests.

use super::*;
use std::process::Command;

#[test]
fn host_process_argv_user_args() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_argv.c");
    let bin = dir.join("rt_host_argv");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
#include <string.h>
int main(int argc, char **argv) {
    draconic_rt_host_process_set_argv(argc, argv);
    int n = draconic_rt_host_process_user_argc();
    printf("%d\n", n);
    for (int i = 0; i < n; i++) {
        const char *a = draconic_rt_host_process_user_arg(i);
        printf("%s\n", a ? a : "");
    }
    if (draconic_rt_host_process_user_arg(n) != NULL) return 2;
    if (draconic_rt_host_process_user_arg(-1) != NULL) return 3;
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
    let out = Command::new(&bin)
        .args(["alpha", "beta"])
        .output()
        .expect("run");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "2\nalpha\nbeta\n",
        "stdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn host_process_env_get_set_delete() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_env.c");
    let bin = dir.join("rt_host_env");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(void) {
    const char *k = "DRACONIC_RT_HOST_ENV_TEST";
    char *v;
    if (draconic_rt_host_env_set(k, "alpha") != 0) return 1;
    v = draconic_rt_host_env_get(k);
    if (!v || strcmp(v, "alpha") != 0) { free(v); return 2; }
    free(v);
    if (draconic_rt_host_env_get("DRACONIC_RT_HOST_ENV_MISSING_XYZ") != NULL) return 3;
    if (draconic_rt_host_env_delete(k) != 0) return 4;
    if (draconic_rt_host_env_get(k) != NULL) return 5;
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
fn host_process_exit_code_and_exit() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_exit.c");
    let bin = dir.join("rt_host_exit");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
int main(void) {
    if (draconic_rt_host_process_get_exit_code() != 0) return 1;
    draconic_rt_host_process_set_exit_code(5);
    if (draconic_rt_host_process_get_exit_code() != 5) return 2;
    /* Immediate terminate with 7 (never returns). */
    draconic_rt_host_process_exit(7);
    return 99;
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
    assert_eq!(
        out.status.code(),
        Some(7),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn host_process_pid_ppid() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_pid.c");
    let bin = dir.join("rt_host_pid");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
int main(void) {
    int32_t p = draconic_rt_host_process_pid();
    int32_t pp = draconic_rt_host_process_ppid();
    if (p <= 0) return 1;
    if (pp < 0) return 2;
    printf("%d\n%d\n", (int)p, (int)pp);
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
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut lines = stdout.lines();
    let p: i32 = lines.next().expect("pid line").parse().expect("pid int");
    let pp: i32 = lines.next().expect("ppid line").parse().expect("ppid int");
    assert!(p > 0, "pid={p}");
    assert!(pp >= 0, "ppid={pp}");
    // Child binary has its own pid; ppid should be this test process.
    assert_eq!(pp as u32, std::process::id());
}

#[test]
fn host_cwd_chdir() {
    // H16.01: getcwd + chdir + restore.
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_cwd.c");
    let bin = dir.join("rt_host_cwd");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(void) {
    char *saved = draconic_rt_host_cwd();
    if (!saved || !saved[0]) return 1;
    if (draconic_rt_host_chdir("/") != DRACONIC_HOST_OK) { free(saved); return 2; }
    char *root = draconic_rt_host_cwd();
    if (!root || strcmp(root, "/") != 0) { free(saved); free(root); return 3; }
    free(root);
    if (draconic_rt_host_chdir(saved) != DRACONIC_HOST_OK) { free(saved); return 4; }
    char *back = draconic_rt_host_cwd();
    if (!back || strcmp(back, saved) != 0) { free(saved); free(back); return 5; }
    free(saved);
    free(back);
    if (draconic_rt_host_chdir("/no/such/draconic_h1601_path_xyz") != DRACONIC_HOST_E_NOENT) {
        return 6;
    }
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
        "code={:?} stderr={} stdout={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ok");
}

#[test]
fn host_hostname_os_type_arch() {
    // H16.02: hostname + platform + arch strings are non-empty.
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_hostname.c");
    let bin = dir.join("rt_host_hostname");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(void) {
    char *h = draconic_rt_host_hostname();
    char *t = draconic_rt_host_os_type();
    char *a = draconic_rt_host_os_arch();
    if (!h || !h[0]) { free(h); free(t); free(a); return 1; }
    if (!t || !t[0]) { free(h); free(t); free(a); return 2; }
    if (!a || !a[0]) { free(h); free(t); free(a); return 3; }
#if defined(__APPLE__)
    if (strcmp(t, "darwin") != 0) { free(h); free(t); free(a); return 4; }
#elif defined(__linux__)
    if (strcmp(t, "linux") != 0) { free(h); free(t); free(a); return 4; }
#elif defined(_WIN32)
    if (strcmp(t, "win32") != 0) { free(h); free(t); free(a); return 4; }
#endif
    free(h);
    free(t);
    free(a);
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
        "code={:?} stderr={} stdout={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ok");
}

#[test]
fn host_temp_home_dir() {
    // H16.03: temp + home directory paths are non-empty.
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_temp_home.c");
    let bin = dir.join("rt_host_temp_home");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(void) {
    char *t = draconic_rt_host_temp_dir();
    char *h = draconic_rt_host_home_dir();
    if (!t || !t[0]) { free(t); free(h); return 1; }
    if (!h || !h[0]) { free(t); free(h); return 2; }
    free(t);
    free(h);
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
        "code={:?} stderr={} stdout={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ok");
}

#[test]
fn host_process_run_argv_cwd_env() {
    // H15.01: spawn+wait exit code; cwd; env subset merge.
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_process_run.c");
    let bin = dir.join("rt_host_process_run");
    // Unique marker file so cwd is proven without relying on /tmp symlink path.
    let marker_dir = dir.join("cwd_marker_dir");
    std::fs::create_dir_all(&marker_dir).unwrap();
    std::fs::write(marker_dir.join("H1501_MARKER"), b"x").unwrap();
    let marker_dir_c = marker_dir.to_string_lossy().replace('\\', "\\\\");
    std::fs::write(
        &main_c,
        format!(
            r#"
#include "draconic_rt_host.h"
#include <stdio.h>
int main(void) {{
    const char *argv_exit[] = {{ "/bin/sh", "-c", "exit 42" }};
    int32_t c = draconic_rt_host_process_run(3, argv_exit, NULL, 0, NULL, NULL);
    if (c != 42) {{ printf("exit=%d\n", (int)c); return 1; }}

    const char *argv_cwd[] = {{ "/bin/sh", "-c", "test -f H1501_MARKER" }};
    c = draconic_rt_host_process_run(3, argv_cwd, "{marker_dir_c}", 0, NULL, NULL);
    if (c != 0) {{ printf("cwd=%d\n", (int)c); return 2; }}

    const char *argv_env[] = {{ "/bin/sh", "-c", "test \"$DRACONIC_H1501_ENV\" = hello" }};
    const char *ek[] = {{ "DRACONIC_H1501_ENV" }};
    const char *ev[] = {{ "hello" }};
    c = draconic_rt_host_process_run(3, argv_env, NULL, 1, ek, ev);
    if (c != 0) {{ printf("env=%d\n", (int)c); return 3; }}

    c = draconic_rt_host_process_run(0, NULL, NULL, 0, NULL, NULL);
    if (c != -1) {{ printf("bad=%d\n", (int)c); return 4; }}

    printf("ok\n");
    return 0;
}}
"#,
            marker_dir_c = marker_dir_c
        ),
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
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ok");
}

#[test]
fn host_process_wait_async_via_job_drain() {
    // H15.03: process_wait_async → Promise; settle via process_poll in job_drain.
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_process_wait_async.c");
    let bin = dir.join("rt_host_process_wait_async");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include "draconic_rt.h"
#include <stdio.h>
#include <stdint.h>
static int64_t g_code = -1;
static void *on_exit(void *data, void *value) {
    (void)data;
    g_code = (int64_t)(intptr_t)value;
    return value;
}
int main(void) {
    const char *argv[] = { "/bin/sh", "-c", "exit 7" };
    int32_t h = draconic_rt_host_process_spawn(3, argv, NULL, 0, NULL, NULL);
    if (h < 1) { printf("spawn=%d\n", (int)h); return 1; }
    DraconicValue *p = draconic_rt_host_process_wait_async(h);
    if (!p) { printf("promise\n"); return 2; }
    (void)draconic_rt_promise_then(p, on_exit, NULL, NULL, NULL);
    draconic_rt_job_drain();
    if (g_code != 7) { printf("code=%lld\n", (long long)g_code); return 3; }
    if (draconic_rt_host_process_close(h) != 0) { printf("close\n"); return 4; }
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
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ok");
}

#[test]
fn host_process_spawn_capture_kill() {
    // H15.02: spawn pipes; stdin write; stdout/stderr capture; kill+wait.
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_process_spawn.c");
    let bin = dir.join("rt_host_process_spawn");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(void) {
    const char *argv_cat[] = { "/bin/sh", "-c", "cat; printf 'err-msg\\n' 1>&2" };
    int32_t h = draconic_rt_host_process_spawn(3, argv_cat, NULL, 0, NULL, NULL);
    if (h < 1) { printf("spawn=%d\n", (int)h); return 1; }
    if (draconic_rt_host_process_stdin_write(h, "hello", -1) != 0) {
        printf("stdin\n"); return 2;
    }
    int32_t code = draconic_rt_host_process_wait(h);
    if (code != 0) { printf("code=%d\n", (int)code); return 3; }
    char *out = NULL;
    char *err = NULL;
    if (draconic_rt_host_process_stdout(h, &out) != 0 || !out) {
        printf("stdout\n"); return 4;
    }
    if (strcmp(out, "hello") != 0) { printf("out=%s\n", out); free(out); return 5; }
    free(out);
    if (draconic_rt_host_process_stderr(h, &err) != 0 || !err) {
        printf("stderr\n"); return 6;
    }
    if (strcmp(err, "err-msg\n") != 0) { printf("err=%s\n", err); free(err); return 7; }
    free(err);
    if (draconic_rt_host_process_close(h) != 0) { printf("close\n"); return 8; }

    const char *argv_sleep[] = { "/bin/sh", "-c", "sleep 30" };
    h = draconic_rt_host_process_spawn(3, argv_sleep, NULL, 0, NULL, NULL);
    if (h < 1) { printf("spawn2=%d\n", (int)h); return 9; }
    if (draconic_rt_host_process_kill(h) != 0) { printf("kill\n"); return 10; }
    code = draconic_rt_host_process_wait(h);
    if (code != 143) { printf("killcode=%d\n", (int)code); return 11; }
    if (draconic_rt_host_process_close(h) != 0) { printf("close2\n"); return 12; }

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
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ok");
}
