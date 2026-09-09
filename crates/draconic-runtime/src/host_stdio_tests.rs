//! Host stdout, stderr, and stdin ABI tests.

use super::*;
use std::process::Command;

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
