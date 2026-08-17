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
    ] {
        assert!(
            out.contains(sym),
            "archive must contain host symbol {sym}\nnm out={out}"
        );
    }
}

/// Clang link smoke: path boundary + handle close/is_valid (no real fs/tcp).
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
