//! H00.03: I/O bytes boundary — Uint8Array/ArrayBuffer as OS read/write buffers.

use super::*;
use std::process::Command;

#[test]
fn host_bytes_symbols_present_in_source_header_and_abi() {
    let src = c_host_runtime_source();
    let host_hdr = c_host_runtime_header_source();
    for sym in [
        HOST_BYTES_FROM_RAW_SYMBOL,
        HOST_BYTES_VIEW_SYMBOL,
        HOST_BYTES_ALLOC_SYMBOL,
        HOST_BYTES_STORAGE_FREE_SYMBOL,
        HOST_BYTES_COPY_IN_SYMBOL,
        HOST_BYTES_COPY_OUT_SYMBOL,
    ] {
        assert!(src.contains(sym), "host C source must define {sym}");
        assert!(host_hdr.contains(sym), "host header must declare {sym}");
        assert!(HOST_SYMBOLS.contains(&sym), "HOST_SYMBOLS must list {sym}");
    }
    assert!(
        host_hdr.contains("DraconicHostBytes"),
        "host header must define DraconicHostBytes view struct"
    );
}

#[test]
fn host_bytes_abi_fn_shapes() {
    assert_eq!(
        HOST_BYTES_FROM_RAW.declare(),
        "declare i32 @draconic_rt_host_bytes_from_raw(ptr, i64, ptr)"
    );
    assert_eq!(
        HOST_BYTES_VIEW.declare(),
        "declare i32 @draconic_rt_host_bytes_view(ptr, i64, i64, ptr)"
    );
    assert_eq!(
        HOST_BYTES_ALLOC.declare(),
        "declare i32 @draconic_rt_host_bytes_alloc(i64, ptr)"
    );
    assert_eq!(
        HOST_BYTES_STORAGE_FREE.declare(),
        "declare void @draconic_rt_host_bytes_storage_free(ptr)"
    );
    assert_eq!(
        HOST_BYTES_COPY_IN.declare(),
        "declare i32 @draconic_rt_host_bytes_copy_in(ptr, ptr, i64, ptr)"
    );
    assert_eq!(
        HOST_BYTES_COPY_OUT.declare(),
        "declare i32 @draconic_rt_host_bytes_copy_out(ptr, ptr, i64, ptr)"
    );
}

#[test]
fn static_lib_includes_host_bytes_symbols() {
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
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
        "draconic_rt_host_bytes_from_raw",
        "draconic_rt_host_bytes_view",
        "draconic_rt_host_bytes_alloc",
        "draconic_rt_host_bytes_storage_free",
        "draconic_rt_host_bytes_copy_in",
        "draconic_rt_host_bytes_copy_out",
    ] {
        assert!(
            out.contains(sym),
            "archive must contain host bytes symbol {sym}\nnm out={out}"
        );
    }
}

/// Clang link smoke: ArrayBuffer-like storage + Uint8Array-like views as OS buffers.
#[test]
fn host_bytes_io_boundary_link_smoke() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_host_bytes");
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
            DraconicHostError err;
            DraconicHostBytes ab;   /* ArrayBuffer-like full storage */
            DraconicHostBytes u8;   /* Uint8Array-like view */
            DraconicHostBytes mid;  /* sliced view (byteOffset/byteLength) */
            uint8_t *storage = NULL;
            uint8_t os_in[8];
            uint8_t os_out[8];
            size_t n = 0;
            size_t i;

            /* --- alloc zeroed storage (ArrayBuffer backing) --- */
            err = draconic_rt_host_bytes_alloc(8, &storage);
            if (err != DRACONIC_HOST_OK || !storage) {
                fprintf(stderr, "alloc want OK got %d storage=%p\n",
                        (int)err, (void *)storage);
                return 1;
            }
            for (i = 0; i < 8; i++) {
                if (storage[i] != 0) {
                    fprintf(stderr, "alloc not zeroed at %zu\n", i);
                    return 2;
                }
            }

            err = draconic_rt_host_bytes_from_raw(storage, 8, &ab);
            if (err != DRACONIC_HOST_OK || ab.data != storage || ab.len != 8) {
                fprintf(stderr, "from_raw full failed err=%d\n", (int)err);
                return 3;
            }

            /* Empty buffer is a valid OS buffer (len 0). */
            err = draconic_rt_host_bytes_from_raw(NULL, 0, &u8);
            if (err != DRACONIC_HOST_OK || u8.data != NULL || u8.len != 0) {
                fprintf(stderr, "empty from_raw failed err=%d\n", (int)err);
                return 4;
            }

            /* NULL data with len>0 → E_INVAL. */
            err = draconic_rt_host_bytes_from_raw(NULL, 4, &u8);
            if (err != DRACONIC_HOST_E_INVAL) {
                fprintf(stderr, "null+len want E_INVAL got %d\n", (int)err);
                return 5;
            }

            /* NULL out → E_INVAL. */
            err = draconic_rt_host_bytes_from_raw(storage, 8, NULL);
            if (err != DRACONIC_HOST_E_INVAL) {
                fprintf(stderr, "null out want E_INVAL got %d\n", (int)err);
                return 6;
            }

            /* Uint8Array over whole ArrayBuffer. */
            err = draconic_rt_host_bytes_view(&ab, 0, 8, &u8);
            if (err != DRACONIC_HOST_OK || u8.data != storage || u8.len != 8) {
                fprintf(stderr, "view full failed err=%d\n", (int)err);
                return 7;
            }

            /* Subarray view: byteOffset=2, byteLength=3 → storage[2..5). */
            err = draconic_rt_host_bytes_view(&ab, 2, 3, &mid);
            if (err != DRACONIC_HOST_OK || mid.data != storage + 2 || mid.len != 3) {
                fprintf(stderr, "view slice failed err=%d data=%p len=%zu\n",
                        (int)err, (void *)mid.data, mid.len);
                return 8;
            }

            /* OOB view → E_INVAL. */
            err = draconic_rt_host_bytes_view(&ab, 6, 4, &mid);
            if (err != DRACONIC_HOST_E_INVAL) {
                fprintf(stderr, "oob view want E_INVAL got %d\n", (int)err);
                return 9;
            }

            /* OS read path: copy_in fills user buffer (may include 0x00 / 0xFF). */
            os_in[0] = 0x00;
            os_in[1] = 0xFF;
            os_in[2] = 0x01;
            os_in[3] = 0x02;
            os_in[4] = 0x7F;
            n = 99;
            err = draconic_rt_host_bytes_view(&ab, 0, 5, &u8);
            if (err != DRACONIC_HOST_OK) {
                fprintf(stderr, "view for copy_in failed %d\n", (int)err);
                return 10;
            }
            err = draconic_rt_host_bytes_copy_in(&u8, os_in, 5, &n);
            if (err != DRACONIC_HOST_OK || n != 5) {
                fprintf(stderr, "copy_in failed err=%d n=%zu\n", (int)err, n);
                return 11;
            }
            if (storage[0] != 0x00 || storage[1] != 0xFF || storage[2] != 0x01
                || storage[3] != 0x02 || storage[4] != 0x7F) {
                fprintf(stderr, "copy_in contents wrong\n");
                return 12;
            }

            /* Partial fill: src longer than view → capped to view len. */
            {
                uint8_t big[4] = { 9, 8, 7, 6 };
                err = draconic_rt_host_bytes_view(&ab, 2, 2, &mid);
                if (err != DRACONIC_HOST_OK) return 13;
                n = 0;
                err = draconic_rt_host_bytes_copy_in(&mid, big, 4, &n);
                if (err != DRACONIC_HOST_OK || n != 2) {
                    fprintf(stderr, "partial copy_in err=%d n=%zu\n", (int)err, n);
                    return 14;
                }
                if (storage[2] != 9 || storage[3] != 8) {
                    fprintf(stderr, "partial copy_in contents\n");
                    return 15;
                }
            }

            /* OS write path: copy_out drains buffer (binary-safe, not C string). */
            memset(os_out, 0xAA, sizeof(os_out));
            err = draconic_rt_host_bytes_from_raw(storage, 5, &u8);
            if (err != DRACONIC_HOST_OK) return 16;
            n = 0;
            err = draconic_rt_host_bytes_copy_out(&u8, os_out, sizeof(os_out), &n);
            if (err != DRACONIC_HOST_OK || n != 5) {
                fprintf(stderr, "copy_out failed err=%d n=%zu\n", (int)err, n);
                return 17;
            }
            /* storage[0..5) = 00 FF 09 08 7F after earlier writes */
            if (os_out[0] != 0x00 || os_out[1] != 0xFF || os_out[2] != 9
                || os_out[3] != 8 || os_out[4] != 0x7F) {
                fprintf(stderr, "copy_out contents wrong\n");
                return 18;
            }
            if (os_out[5] != 0xAA) {
                fprintf(stderr, "copy_out overran dest\n");
                return 19;
            }

            /* Empty copy_in/out OK with n=0. */
            err = draconic_rt_host_bytes_from_raw(NULL, 0, &u8);
            if (err != DRACONIC_HOST_OK) return 20;
            n = 7;
            err = draconic_rt_host_bytes_copy_in(&u8, os_in, 3, &n);
            if (err != DRACONIC_HOST_OK || n != 0) {
                fprintf(stderr, "empty copy_in err=%d n=%zu\n", (int)err, n);
                return 21;
            }
            n = 7;
            err = draconic_rt_host_bytes_copy_out(&u8, os_out, 3, &n);
            if (err != DRACONIC_HOST_OK || n != 0) {
                fprintf(stderr, "empty copy_out err=%d n=%zu\n", (int)err, n);
                return 22;
            }

            /* alloc len 0 → OK, data NULL. */
            {
                uint8_t *z = (uint8_t *)0x1;
                err = draconic_rt_host_bytes_alloc(0, &z);
                if (err != DRACONIC_HOST_OK || z != NULL) {
                    fprintf(stderr, "alloc0 err=%d z=%p\n", (int)err, (void *)z);
                    return 23;
                }
            }

            /* NULL out_data for alloc → E_INVAL. */
            err = draconic_rt_host_bytes_alloc(4, NULL);
            if (err != DRACONIC_HOST_E_INVAL) {
                fprintf(stderr, "alloc null out want E_INVAL got %d\n", (int)err);
                return 24;
            }

            draconic_rt_host_bytes_storage_free(storage);
            /* free(NULL) must be safe. */
            draconic_rt_host_bytes_storage_free(NULL);

            puts("host-bytes-ok");
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
    assert!(
        status.success(),
        "clang failed to link host bytes smoke against libdraconic_rt.a"
    );

    let output = Command::new(&bin).output().expect("run rt_host_bytes");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "host bytes binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "host-bytes-ok\n", "stdout={stdout:?}");
}
