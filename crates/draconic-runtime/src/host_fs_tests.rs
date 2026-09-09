//! Host filesystem ABI tests.

use super::*;
use std::process::Command;

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

#[test]
fn host_fs_exists_and_stat() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_fs_h0403.c");
    let bin = dir.join("rt_host_fs_h0403");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    let hello = dir.join("hello.txt");
    std::fs::write(&hello, b"hello-h0403").unwrap();
    let sub = dir.join("subdir");
    std::fs::create_dir_all(&sub).unwrap();
    let hello_path = hello.to_string_lossy().replace('\\', "\\\\");
    let sub_path = sub.to_string_lossy().replace('\\', "\\\\");
    let missing_path = dir
        .join("__no_such_h0403__")
        .to_string_lossy()
        .replace('\\', "\\\\");

    std::fs::write(
        &main_c,
        format!(
            r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>

        int main(void) {{
            int64_t size = 0;
            int32_t is_file = 0;
            int32_t is_dir = 0;
            double mtime = 0.0;
            DraconicHostError err;

            if (draconic_rt_host_fs_exists("{hello_path}") != 1) return 1;
            if (draconic_rt_host_fs_exists("{sub_path}") != 1) return 2;
            if (draconic_rt_host_fs_exists("{missing_path}") != 0) return 3;
            if (draconic_rt_host_fs_exists(NULL) != 0) return 4;
            if (draconic_rt_host_fs_exists("") != 0) return 5;

            err = draconic_rt_host_fs_stat("{hello_path}", &size, &is_file, &is_dir, &mtime);
            if (err != DRACONIC_HOST_OK) return 6;
            if (size != 11) return 7;
            if (is_file != 1) return 8;
            if (is_dir != 0) return 9;
            if (!(mtime > 0.0)) return 10;

            err = draconic_rt_host_fs_stat("{sub_path}", &size, &is_file, &is_dir, &mtime);
            if (err != DRACONIC_HOST_OK) return 11;
            if (is_file != 0) return 12;
            if (is_dir != 1) return 13;

            err = draconic_rt_host_fs_stat("{missing_path}", &size, &is_file, &is_dir, &mtime);
            if (err != DRACONIC_HOST_E_NOENT) return 14;

            err = draconic_rt_host_fs_stat(NULL, &size, &is_file, &is_dir, &mtime);
            if (err != DRACONIC_HOST_E_INVAL) return 15;

            puts("fs-h0403-ok");
            return 0;
        }}
        "#
        ),
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
    assert!(status.success(), "clang failed for fs H04.03 smoke");

    let output = Command::new(&bin).output().expect("run fs h0403");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "fs H04.03 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "fs-h0403-ok\n");
}

#[test]
fn host_fs_dir_ops() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_fs_h0404.c");
    let bin = dir.join("rt_host_fs_h0404");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    let base = dir.join("h0404");
    let base_path = base.to_string_lossy().replace('\\', "\\\\");
    let nested = base.join("a").join("b");
    let nested_path = nested.to_string_lossy().replace('\\', "\\\\");
    let file_path = base
        .join("only.txt")
        .to_string_lossy()
        .replace('\\', "\\\\");
    let child = base.join("child");
    let child_path = child.to_string_lossy().replace('\\', "\\\\");

    std::fs::write(
        &main_c,
        format!(
            r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <stdlib.h>
        #include <string.h>

        int main(void) {{
            DraconicHostError err;
            char **names = NULL;
            int64_t count = 0;

            err = draconic_rt_host_fs_mkdir_all("{nested_path}");
            if (err != DRACONIC_HOST_OK) return 1;
            if (draconic_rt_host_fs_exists("{nested_path}") != 1) return 2;

            err = draconic_rt_host_fs_mkdir("{child_path}");
            if (err != DRACONIC_HOST_OK) return 3;
            err = draconic_rt_host_fs_mkdir("{child_path}");
            if (err != DRACONIC_HOST_E_EXIST) return 4;

            {{
                FILE *f = fopen("{file_path}", "w");
                if (!f) return 5;
                fputs("x", f);
                fclose(f);
            }}

            err = draconic_rt_host_fs_readdir("{base_path}", &names, &count);
            if (err != DRACONIC_HOST_OK) return 6;
            if (count < 2) return 7; /* child dir + only.txt at least */
            {{
                int found = 0;
                for (int64_t i = 0; i < count; i++) {{
                    if (names[i] && strcmp(names[i], "only.txt") == 0) found = 1;
                    free(names[i]);
                }}
                free(names);
                names = NULL;
                if (!found) return 8;
            }}

            err = draconic_rt_host_fs_remove_file("{file_path}");
            if (err != DRACONIC_HOST_OK) return 9;
            if (draconic_rt_host_fs_exists("{file_path}") != 0) return 10;

            err = draconic_rt_host_fs_rmdir("{child_path}");
            if (err != DRACONIC_HOST_OK) return 11;
            if (draconic_rt_host_fs_exists("{child_path}") != 0) return 12;

            err = draconic_rt_host_fs_mkdir(NULL);
            if (err != DRACONIC_HOST_E_INVAL) return 13;
            err = draconic_rt_host_fs_readdir("{base_path}/__no_such__", &names, &count);
            if (err != DRACONIC_HOST_E_NOENT) return 14;

            puts("fs-h0404-ok");
            return 0;
        }}
        "#
        ),
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
    assert!(status.success(), "clang failed for fs H04.04 smoke");

    let output = Command::new(&bin).output().expect("run fs h0404");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "fs H04.04 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "fs-h0404-ok\n");
}

#[test]
fn host_fs_rename_and_copy() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_fs_h0405.c");
    let bin = dir.join("rt_host_fs_h0405");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    let base = dir.join("h0405");
    std::fs::create_dir_all(&base).unwrap();
    let src = base.join("src.txt").to_string_lossy().replace('\\', "\\\\");
    let ren_dst = base.join("ren.txt").to_string_lossy().replace('\\', "\\\\");
    let cp_src = base
        .join("cp_src.txt")
        .to_string_lossy()
        .replace('\\', "\\\\");
    let cp_dst = base
        .join("cp_dst.txt")
        .to_string_lossy()
        .replace('\\', "\\\\");

    std::fs::write(
        &main_c,
        format!(
            r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <stdlib.h>
        #include <string.h>

        int main(void) {{
            DraconicHostError err;
            char *text = NULL;

            err = draconic_rt_host_fs_write_text("{src}", "ren-h0405");
            if (err != DRACONIC_HOST_OK) return 1;
            err = draconic_rt_host_fs_rename_file("{src}", "{ren_dst}");
            if (err != DRACONIC_HOST_OK) return 2;
            if (draconic_rt_host_fs_exists("{src}") != 0) return 3;
            err = draconic_rt_host_fs_read_text("{ren_dst}", &text);
            if (err != DRACONIC_HOST_OK) return 4;
            if (!text || strcmp(text, "ren-h0405") != 0) return 5;
            free(text);
            text = NULL;

            err = draconic_rt_host_fs_write_text("{cp_src}", "cp-h0405");
            if (err != DRACONIC_HOST_OK) return 6;
            err = draconic_rt_host_fs_copy_file("{cp_src}", "{cp_dst}");
            if (err != DRACONIC_HOST_OK) return 7;
            err = draconic_rt_host_fs_read_text("{cp_src}", &text);
            if (err != DRACONIC_HOST_OK) return 8;
            if (!text || strcmp(text, "cp-h0405") != 0) return 9;
            free(text);
            text = NULL;
            err = draconic_rt_host_fs_read_text("{cp_dst}", &text);
            if (err != DRACONIC_HOST_OK) return 10;
            if (!text || strcmp(text, "cp-h0405") != 0) return 11;
            free(text);

            err = draconic_rt_host_fs_rename_file(NULL, "{ren_dst}");
            if (err != DRACONIC_HOST_E_INVAL) return 12;
            err = draconic_rt_host_fs_copy_file("{cp_src}/__missing__", "{cp_dst}");
            if (err != DRACONIC_HOST_E_NOENT) return 13;

            puts("fs-h0405-ok");
            return 0;
        }}
        "#
        ),
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
    assert!(status.success(), "clang failed for fs H04.05 smoke");

    let output = Command::new(&bin).output().expect("run fs h0405");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "fs H04.05 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "fs-h0405-ok\n");
}

#[test]
fn host_fs_open_handle_rw_seek_close() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_fs_h0406.c");
    let bin = dir.join("rt_host_fs_h0406");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    let path = dir
        .join("h0406.txt")
        .to_string_lossy()
        .replace('\\', "\\\\");

    std::fs::write(
        &main_c,
        format!(
            r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <stdlib.h>
        #include <string.h>

        int main(void) {{
            DraconicHostError err;
            DraconicHostHandle h = DRACONIC_HOST_HANDLE_INVALID;
            uint8_t *data = NULL;
            size_t len = 0;
            int64_t pos = -1;

            err = draconic_rt_host_fs_open("{path}", "w+", &h);
            if (err != DRACONIC_HOST_OK) return 1;
            if (!draconic_rt_host_handle_is_valid(h)) return 2;

            err = draconic_rt_host_fs_handle_write(h, (const uint8_t *)"hello-h0406", 11);
            if (err != DRACONIC_HOST_OK) return 3;

            err = draconic_rt_host_fs_handle_seek(h, 0, 0, &pos);
            if (err != DRACONIC_HOST_OK) return 4;
            if (pos != 0) return 5;

            err = draconic_rt_host_fs_handle_read(h, 64, &data, &len);
            if (err != DRACONIC_HOST_OK) return 6;
            if (len != 11 || !data || memcmp(data, "hello-h0406", 11) != 0) return 7;
            free(data);
            data = NULL;

            err = draconic_rt_host_fs_handle_seek(h, 6, 0, &pos);
            if (err != DRACONIC_HOST_OK) return 8;
            if (pos != 6) return 9;
            err = draconic_rt_host_fs_handle_read(h, 64, &data, &len);
            if (err != DRACONIC_HOST_OK) return 10;
            if (len != 5 || !data || memcmp(data, "h0406", 5) != 0) return 11;
            free(data);

            err = draconic_rt_host_handle_close(h);
            if (err != DRACONIC_HOST_OK) return 12;
            if (draconic_rt_host_handle_is_valid(h)) return 13;
            err = draconic_rt_host_handle_close(h);
            if (err != DRACONIC_HOST_E_BADF) return 14;

            err = draconic_rt_host_fs_open("{path}/__missing_parent__/x", "r", &h);
            if (err != DRACONIC_HOST_E_NOENT) return 15;

            err = draconic_rt_host_fs_open(NULL, "r", &h);
            if (err != DRACONIC_HOST_E_INVAL) return 16;
            err = draconic_rt_host_fs_open("{path}", "zz", &h);
            if (err != DRACONIC_HOST_E_INVAL) return 17;

            puts("fs-h0406-ok");
            return 0;
        }}
        "#
        ),
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
    assert!(status.success(), "clang failed for fs H04.06 smoke");

    let output = Command::new(&bin).output().expect("run fs h0406");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "fs H04.06 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "fs-h0406-ok\n");
}
