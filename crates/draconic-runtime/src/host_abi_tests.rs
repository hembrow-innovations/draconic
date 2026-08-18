//! H00.02: Host Runtime ABI scaffold — error codes, handles, path boundary.

use super::*;
use std::process::Command;

#[test]
fn host_error_codes_stable_in_abi_and_header() {
    assert_eq!(HOST_OK, 0);
    assert_eq!(HOST_E_INVAL, 1);
    assert_eq!(HOST_E_NOENT, 2);
    assert_eq!(HOST_E_NOSYS, 3);
    assert_eq!(HOST_E_BADF, 4);
    assert_eq!(HOST_E_EXIST, 5);
    assert_eq!(HOST_E_PERM, 6);
    assert_eq!(HOST_E_IO, 7);
    assert_eq!(HOST_E_NOMEM, 8);
    assert_eq!(HOST_E_AGAIN, 9);
    assert_eq!(HOST_E_CONN, 10);
    assert_eq!(HOST_E_ADDR, 11);
    assert_eq!(HOST_HANDLE_INVALID, -1);

    let host_hdr = c_host_runtime_header_source();
    let main_hdr = c_runtime_header_source();
    assert!(
        main_hdr.contains("draconic_rt_host.h"),
        "main runtime header must include host substrate header"
    );
    for name in [
        "DRACONIC_HOST_OK",
        "DRACONIC_HOST_E_INVAL",
        "DRACONIC_HOST_E_NOENT",
        "DRACONIC_HOST_E_NOSYS",
        "DRACONIC_HOST_E_BADF",
        "DRACONIC_HOST_E_EXIST",
        "DRACONIC_HOST_E_PERM",
        "DRACONIC_HOST_E_IO",
        "DRACONIC_HOST_E_NOMEM",
        "DRACONIC_HOST_E_AGAIN",
        "DRACONIC_HOST_E_CONN",
        "DRACONIC_HOST_E_ADDR",
        "DRACONIC_HOST_HANDLE_INVALID",
        "DraconicHostError",
        "DraconicHostHandle",
    ] {
        assert!(
            host_hdr.contains(name),
            "host header must define/declare {name}"
        );
    }
}

#[test]
fn host_symbols_present_in_source_header_and_abi() {
    let src = c_host_runtime_source();
    let host_hdr = c_host_runtime_header_source();
    for sym in HOST_SYMBOLS {
        assert!(src.contains(sym), "host C source must define {sym}");
        assert!(host_hdr.contains(sym), "host header must declare {sym}");
    }
    assert!(
        c_host_runtime_path().is_file(),
        "draconic_rt_host.c must exist on disk"
    );
    assert!(
        c_host_runtime_header_path().is_file(),
        "draconic_rt_host.h must exist on disk"
    );
    assert!(HOST_SYMBOLS.contains(&HOST_HANDLE_IS_VALID_SYMBOL));
    assert!(HOST_SYMBOLS.contains(&HOST_HANDLE_CLOSE_SYMBOL));
    assert!(HOST_SYMBOLS.contains(&HOST_PATH_FROM_UTF8_SYMBOL));
    assert!(HOST_SYMBOLS.contains(&HOST_PATH_FREE_SYMBOL));
}

#[test]
fn host_abi_fn_shapes() {
    assert_eq!(
        HOST_HANDLE_IS_VALID.declare(),
        "declare i32 @draconic_rt_host_handle_is_valid(i64)"
    );
    assert_eq!(
        HOST_HANDLE_CLOSE.declare(),
        "declare i32 @draconic_rt_host_handle_close(i64)"
    );
    assert_eq!(
        HOST_PATH_FROM_UTF8.declare(),
        "declare i32 @draconic_rt_host_path_from_utf8(ptr, i64, ptr)"
    );
    assert_eq!(
        HOST_PATH_FREE.declare(),
        "declare void @draconic_rt_host_path_free(ptr)"
    );
}

#[test]
fn static_lib_includes_host_object() {
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    // `nm` lists archive members; host symbols must be present after multi-file ar.
    let nm = Command::new("nm")
        .arg(&archive)
        .output()
        .expect("nm on archive");
    let out = String::from_utf8_lossy(&nm.stdout);
    let err = String::from_utf8_lossy(&nm.stderr);
    assert!(
        nm.status.success() || !out.is_empty(),
        "nm failed: status={:?} stderr={err}",
        nm.status
    );
    for sym in [
        "draconic_rt_host_handle_is_valid",
        "draconic_rt_host_handle_close",
        "draconic_rt_host_path_from_utf8",
        "draconic_rt_host_path_free",
        "draconic_rt_host_process_set_argv",
        "draconic_rt_host_process_user_argc",
        "draconic_rt_host_process_user_arg",
        "draconic_rt_host_env_get",
        "draconic_rt_host_env_set",
        "draconic_rt_host_env_delete",
        "draconic_rt_host_process_exit",
        "draconic_rt_host_process_set_exit_code",
        "draconic_rt_host_process_get_exit_code",
        "draconic_rt_host_process_pid",
        "draconic_rt_host_process_ppid",
        "draconic_rt_host_stdout_write",
        "draconic_rt_host_stderr_write",
        "draconic_rt_host_stdin_read_line",
        "draconic_rt_host_stdin_read_bytes",
        "draconic_rt_host_path_normalize",
        "draconic_rt_host_path_join",
        "draconic_rt_host_path_dirname",
        "draconic_rt_host_path_basename",
        "draconic_rt_host_path_extname",
        "draconic_rt_host_path_is_absolute",
    ] {
        assert!(
            out.contains(sym),
            "archive must contain host symbol {sym}\nnm out={out}"
        );
    }
}

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
    let status = Command::new(&clang)
        .arg(&main_c)
        .arg(&archive)
        .arg(format!("-I{}", header_dir.display()))
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("clang link");
    assert!(status.success(), "link failed");
    let out = Command::new(&bin)
        .args(["alpha", "beta"])
        .output()
        .expect("run");
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));
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
    let status = Command::new(&clang)
        .arg(&main_c)
        .arg(&archive)
        .arg(format!("-I{}", header_dir.display()))
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("clang link");
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
    let status = Command::new(&clang)
        .arg(&main_c)
        .arg(&archive)
        .arg(format!("-I{}", header_dir.display()))
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("clang link");
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
    let status = Command::new(&clang)
        .arg(&main_c)
        .arg(&archive)
        .arg(format!("-I{}", header_dir.display()))
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("clang link");
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
    let p: i32 = lines
        .next()
        .expect("pid line")
        .parse()
        .expect("pid int");
    let pp: i32 = lines
        .next()
        .expect("ppid line")
        .parse()
        .expect("ppid int");
    assert!(p > 0, "pid={p}");
    assert!(pp >= 0, "ppid={pp}");
    // Child binary has its own pid; ppid should be this test process.
    assert_eq!(pp as u32, std::process::id());
}

#[test]
fn host_stdout_write_bytes() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_stdout.c");
    let bin = dir.join("rt_host_stdout");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
int main(void) {
    const uint8_t msg[] = { 'h', 'i', '\n', 0 };
    if (draconic_rt_host_stdout_write(msg, 3) != DRACONIC_HOST_OK) return 1;
    if (draconic_rt_host_stdout_write(NULL, 0) != DRACONIC_HOST_OK) return 2;
    if (draconic_rt_host_stdout_write(NULL, 1) != DRACONIC_HOST_E_INVAL) return 3;
    return 0;
}
"#,
    )
    .unwrap();
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    let status = Command::new(&clang)
        .arg(&main_c)
        .arg(&archive)
        .arg(format!("-I{}", header_dir.display()))
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("clang link");
    assert!(status.success(), "link failed");
    let out = Command::new(&bin).output().expect("run");
    assert!(
        out.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "hi\n");
}

#[test]
fn host_stderr_write_bytes() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_stderr.c");
    let bin = dir.join("rt_host_stderr");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
int main(void) {
    const uint8_t msg[] = { 'e', 'r', '\n', 0 };
    if (draconic_rt_host_stderr_write(msg, 3) != DRACONIC_HOST_OK) return 1;
    if (draconic_rt_host_stderr_write(NULL, 0) != DRACONIC_HOST_OK) return 2;
    if (draconic_rt_host_stderr_write(NULL, 1) != DRACONIC_HOST_E_INVAL) return 3;
    return 0;
}
"#,
    )
    .unwrap();
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    let status = Command::new(&clang)
        .arg(&main_c)
        .arg(&archive)
        .arg(format!("-I{}", header_dir.display()))
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("clang link");
    assert!(status.success(), "link failed");
    let out = Command::new(&bin).output().expect("run");
    assert!(
        out.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&out.stderr), "er\n");
}

#[test]
fn host_stdin_read_line_and_bytes() {
    use std::io::Write;
    use std::process::Stdio;

    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_stdin.c");
    let bin = dir.join("rt_host_stdin");
    std::fs::write(
        &main_c,
        r#"
#include "draconic_rt_host.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(void) {
    char *line = draconic_rt_host_stdin_read_line();
    if (!line) return 1;
    if (strcmp(line, "hi") != 0) { free(line); return 2; }
    free(line);
    uint8_t *data = NULL;
    size_t n = 0;
    if (draconic_rt_host_stdin_read_bytes(3, &data, &n) != DRACONIC_HOST_OK) return 3;
    if (n != 3 || !data) return 4;
    if (data[0] != 'A' || data[1] != 'B' || data[2] != 'C') { free(data); return 5; }
    free(data);
    line = draconic_rt_host_stdin_read_line();
    if (line != NULL) { free(line); return 6; }
    return 0;
}
"#,
    )
    .unwrap();
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();
    let status = Command::new(&clang)
        .arg(&main_c)
        .arg(&archive)
        .arg(format!("-I{}", header_dir.display()))
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("clang link");
    assert!(status.success(), "link failed");
    let mut child = Command::new(&bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    {
        let mut sin = child.stdin.take().expect("stdin");
        sin.write_all(b"hi\nABC").expect("write stdin");
    }
    let out = child.wait_with_output().expect("wait");
    assert!(
        out.status.success(),
        "exit={:?} stderr={} stdout={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn host_abi_path_and_handles_link_smoke() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_host_abi");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <string.h>
        #include <stdlib.h>

        int main(void) {
            char *path = NULL;
            DraconicHostError err;

            /* Valid UTF-8 path → OK, NUL-terminated copy. */
            err = draconic_rt_host_path_from_utf8("tmp/x", 5, &path);
            if (err != DRACONIC_HOST_OK || !path) {
                fprintf(stderr, "path_from_utf8 want OK got %d path=%p\n",
                        (int)err, (void *)path);
                return 1;
            }
            if (strcmp(path, "tmp/x") != 0) {
                fprintf(stderr, "path contents wrong: %s\n", path);
                return 2;
            }
            draconic_rt_host_path_free(path);
            path = NULL;

            /* Empty path is valid (zero-length relative). */
            err = draconic_rt_host_path_from_utf8("", 0, &path);
            if (err != DRACONIC_HOST_OK || !path || path[0] != '\0') {
                fprintf(stderr, "empty path failed err=%d\n", (int)err);
                return 3;
            }
            draconic_rt_host_path_free(path);
            path = NULL;

            /* Embedded NUL rejected. */
            err = draconic_rt_host_path_from_utf8("a\0b", 3, &path);
            if (err != DRACONIC_HOST_E_INVAL || path != NULL) {
                fprintf(stderr, "embedded NUL want E_INVAL got %d\n", (int)err);
                return 4;
            }

            /* Invalid UTF-8 rejected (overlong / bare continuation). */
            {
                const char bad[] = { (char)0x80, 0 };
                err = draconic_rt_host_path_from_utf8(bad, 1, &path);
                if (err != DRACONIC_HOST_E_INVAL || path != NULL) {
                    fprintf(stderr, "bad utf8 want E_INVAL got %d\n", (int)err);
                    return 5;
                }
            }

            /* NULL out_path → E_INVAL. */
            err = draconic_rt_host_path_from_utf8("x", 1, NULL);
            if (err != DRACONIC_HOST_E_INVAL) {
                fprintf(stderr, "null out want E_INVAL got %d\n", (int)err);
                return 6;
            }

            /* NULL data with len>0 → E_INVAL. */
            err = draconic_rt_host_path_from_utf8(NULL, 1, &path);
            if (err != DRACONIC_HOST_E_INVAL || path != NULL) {
                fprintf(stderr, "null data want E_INVAL got %d\n", (int)err);
                return 7;
            }

            /* Handles: invalid is never valid; close → E_BADF. */
            if (draconic_rt_host_handle_is_valid(DRACONIC_HOST_HANDLE_INVALID)) {
                fprintf(stderr, "INVALID handle must not be valid\n");
                return 8;
            }
            if (draconic_rt_host_handle_is_valid(0)) {
                fprintf(stderr, "handle 0 must not be valid\n");
                return 9;
            }
            err = draconic_rt_host_handle_close(DRACONIC_HOST_HANDLE_INVALID);
            if (err != DRACONIC_HOST_E_BADF) {
                fprintf(stderr, "close INVALID want E_BADF got %d\n", (int)err);
                return 10;
            }

            /* Non-UTF8 multi-byte path with valid UTF-8 (emoji dir) OK. */
            {
                /* U+1F4C1 📁 = F0 9F 93 81 */
                const char *emoji = "\xF0\x9F\x93\x81";
                err = draconic_rt_host_path_from_utf8(emoji, 4, &path);
                if (err != DRACONIC_HOST_OK || !path
                    || memcmp(path, emoji, 4) != 0 || path[4] != '\0') {
                    fprintf(stderr, "emoji path failed err=%d\n", (int)err);
                    return 11;
                }
                draconic_rt_host_path_free(path);
            }

            puts("host-abi-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = Command::new(&clang)
        .arg(&main_c)
        .arg(&archive)
        .arg("-I")
        .arg(&header_dir)
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("spawn clang");
    assert!(
        status.success(),
        "clang failed to link host ABI smoke against libdraconic_rt.a"
    );

    let output = Command::new(&bin).output().expect("run rt_host_abi");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "host ABI binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "host-abi-ok\n", "stdout={stdout:?}");
}

#[test]
fn host_path_dirname_basename_extname_is_absolute() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_path_h0302.c");
    let bin = dir.join("rt_host_path_h0302");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <string.h>
        #include <stdlib.h>

        static int expect_str(const char *got, const char *want, const char *label) {
            if (!got) {
                fprintf(stderr, "%s: null\n", label);
                return 0;
            }
            if (strcmp(got, want) != 0) {
                fprintf(stderr, "%s: got \"%s\" want \"%s\"\n", label, got, want);
                return 0;
            }
            return 1;
        }

        int main(void) {
            char *s;

            s = draconic_rt_host_path_dirname("/foo/bar/baz");
            if (!expect_str(s, "/foo/bar", "dirname abs")) return 1;
            free(s);
            s = draconic_rt_host_path_dirname("foo");
            if (!expect_str(s, ".", "dirname rel")) return 2;
            free(s);
            s = draconic_rt_host_path_dirname("foo\\bar\\baz");
            if (!expect_str(s, "foo/bar", "dirname backslash")) return 3;
            free(s);

            s = draconic_rt_host_path_basename("/foo/bar/baz.txt");
            if (!expect_str(s, "baz.txt", "basename")) return 4;
            free(s);
            s = draconic_rt_host_path_basename("/");
            if (!expect_str(s, "", "basename root")) return 5;
            free(s);

            s = draconic_rt_host_path_extname("index.coffee.md");
            if (!expect_str(s, ".md", "extname multi")) return 6;
            free(s);
            s = draconic_rt_host_path_extname(".index");
            if (!expect_str(s, "", "extname dotfile")) return 7;
            free(s);
            s = draconic_rt_host_path_extname("index.");
            if (!expect_str(s, ".", "extname trailing dot")) return 8;
            free(s);

            if (draconic_rt_host_path_is_absolute("/foo") != 1) return 9;
            if (draconic_rt_host_path_is_absolute("foo") != 0) return 10;
            if (draconic_rt_host_path_is_absolute("\\foo") != 1) return 11;
            if (draconic_rt_host_path_is_absolute("") != 0) return 12;

            puts("path-h0302-ok");
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = Command::new(&clang)
        .arg(&main_c)
        .arg(&archive)
        .arg("-I")
        .arg(&header_dir)
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("spawn clang");
    assert!(status.success(), "clang failed for path H03.02 smoke");

    let output = Command::new(&bin).output().expect("run path h0302");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "path H03.02 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "path-h0302-ok\n"
    );
}

#[test]
fn host_fs_read_text_and_bytes() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_fs_h0401.c");
    let bin = dir.join("rt_host_fs_h0401");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    let hello = dir.join("hello.txt");
    std::fs::write(&hello, b"hello-h0401").unwrap();
    let empty = dir.join("empty.txt");
    std::fs::write(&empty, b"").unwrap();
    let hello_path = hello.to_string_lossy().replace('\\', "\\\\");
    let empty_path = empty.to_string_lossy().replace('\\', "\\\\");
    let missing_path = dir
        .join("__no_such_h0401__")
        .to_string_lossy()
        .replace('\\', "\\\\");

    std::fs::write(
        &main_c,
        format!(
            r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <string.h>
        #include <stdlib.h>

        int main(void) {{
            char *text = NULL;
            uint8_t *data = NULL;
            size_t len = 0;
            DraconicHostError err;

            err = draconic_rt_host_fs_read_text("{hello_path}", &text);
            if (err != DRACONIC_HOST_OK) return 1;
            if (!text || strcmp(text, "hello-h0401") != 0) return 2;
            free(text);

            err = draconic_rt_host_fs_read_file("{hello_path}", &data, &len);
            if (err != DRACONIC_HOST_OK) return 3;
            if (len != 11 || !data || memcmp(data, "hello-h0401", 11) != 0) return 4;
            free(data);

            err = draconic_rt_host_fs_read_text("{empty_path}", &text);
            if (err != DRACONIC_HOST_OK) return 5;
            if (!text || text[0] != '\0') return 6;
            free(text);

            err = draconic_rt_host_fs_read_file("{empty_path}", &data, &len);
            if (err != DRACONIC_HOST_OK) return 7;
            if (len != 0 || data != NULL) return 8;

            err = draconic_rt_host_fs_read_text("{missing_path}", &text);
            if (err != DRACONIC_HOST_E_NOENT) return 9;
            if (text != NULL) return 10;

            err = draconic_rt_host_fs_read_file(NULL, &data, &len);
            if (err != DRACONIC_HOST_E_INVAL) return 11;

            puts("fs-h0401-ok");
            return 0;
        }}
        "#
        ),
    )
    .unwrap();

    let status = Command::new(&clang)
        .arg(&main_c)
        .arg(&archive)
        .arg("-I")
        .arg(&header_dir)
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("spawn clang");
    assert!(status.success(), "clang failed for fs H04.01 smoke");

    let output = Command::new(&bin).output().expect("run fs h0401");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "fs H04.01 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "fs-h0401-ok\n");
}

#[test]
fn host_fs_write_append_text_and_bytes() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_fs_h0402.c");
    let bin = dir.join("rt_host_fs_h0402");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    let out_path = dir.join("out.txt");
    let out_path_s = out_path.to_string_lossy().replace('\\', "\\\\");
    let bin_path = dir.join("out.bin");
    let bin_path_s = bin_path.to_string_lossy().replace('\\', "\\\\");
    let missing_parent = dir
        .join("no_such_dir")
        .join("nested.txt")
        .to_string_lossy()
        .replace('\\', "\\\\");

    std::fs::write(
        &main_c,
        format!(
            r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <string.h>
        #include <stdlib.h>

        int main(void) {{
            char *text = NULL;
            uint8_t *data = NULL;
            size_t len = 0;
            DraconicHostError err;

            err = draconic_rt_host_fs_write_text("{out_path_s}", "wt-h0402");
            if (err != DRACONIC_HOST_OK) return 1;
            err = draconic_rt_host_fs_read_text("{out_path_s}", &text);
            if (err != DRACONIC_HOST_OK) return 2;
            if (!text || strcmp(text, "wt-h0402") != 0) return 3;
            free(text); text = NULL;

            err = draconic_rt_host_fs_write_text("{out_path_s}", "long-content");
            if (err != DRACONIC_HOST_OK) return 4;
            err = draconic_rt_host_fs_write_text("{out_path_s}", "short");
            if (err != DRACONIC_HOST_OK) return 5;
            err = draconic_rt_host_fs_read_text("{out_path_s}", &text);
            if (err != DRACONIC_HOST_OK) return 6;
            if (!text || strcmp(text, "short") != 0) return 7;
            free(text); text = NULL;

            err = draconic_rt_host_fs_write_text("{out_path_s}", "A");
            if (err != DRACONIC_HOST_OK) return 8;
            err = draconic_rt_host_fs_append_text("{out_path_s}", "B");
            if (err != DRACONIC_HOST_OK) return 9;
            err = draconic_rt_host_fs_append_text("{out_path_s}", "C");
            if (err != DRACONIC_HOST_OK) return 10;
            err = draconic_rt_host_fs_read_text("{out_path_s}", &text);
            if (err != DRACONIC_HOST_OK) return 11;
            if (!text || strcmp(text, "ABC") != 0) return 12;
            free(text); text = NULL;

            err = draconic_rt_host_fs_write_file("{bin_path_s}", (const uint8_t *)"xy", 2);
            if (err != DRACONIC_HOST_OK) return 13;
            err = draconic_rt_host_fs_append_file("{bin_path_s}", (const uint8_t *)"z", 1);
            if (err != DRACONIC_HOST_OK) return 14;
            err = draconic_rt_host_fs_read_file("{bin_path_s}", &data, &len);
            if (err != DRACONIC_HOST_OK) return 15;
            if (len != 3 || !data || memcmp(data, "xyz", 3) != 0) return 16;
            free(data); data = NULL;

            err = draconic_rt_host_fs_write_text("{out_path_s}", "");
            if (err != DRACONIC_HOST_OK) return 17;
            err = draconic_rt_host_fs_read_text("{out_path_s}", &text);
            if (err != DRACONIC_HOST_OK) return 18;
            if (!text || text[0] != '\0') return 19;
            free(text); text = NULL;

            err = draconic_rt_host_fs_write_text("{missing_parent}", "x");
            if (err != DRACONIC_HOST_E_NOENT) return 20;

            err = draconic_rt_host_fs_write_text(NULL, "x");
            if (err != DRACONIC_HOST_E_INVAL) return 21;

            puts("fs-h0402-ok");
            return 0;
        }}
        "#
        ),
    )
    .unwrap();

    let status = Command::new(&clang)
        .arg(&main_c)
        .arg(&archive)
        .arg("-I")
        .arg(&header_dir)
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("spawn clang");
    assert!(status.success(), "clang failed for fs H04.02 smoke");

    let output = Command::new(&bin).output().expect("run fs h0402");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "fs H04.02 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "fs-h0402-ok\n");
}
