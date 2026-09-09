//! Host DNS lookup ABI tests.

use super::*;
use std::process::Command;

#[test]
fn host_dns_lookup_loopback_and_failure() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main_dns_h0901.c");
    let bin = dir.join("rt_host_dns_h0901");
    let header_dir = c_runtime_header_path()
        .parent()
        .expect("header parent")
        .to_path_buf();

    std::fs::write(
        &main_c,
        r#"
        #include "draconic_rt.h"
        #include <stdio.h>
        #include <stdint.h>
        #include <string.h>
        #include <stdlib.h>

        int main(void) {
            DraconicHostError err;
            char **addrs = NULL;
            int64_t count = 0;
            int found = 0;
            int64_t i;

            err = draconic_rt_host_dns_lookup("127.0.0.1", &addrs, &count);
            if (err != DRACONIC_HOST_OK) return 1;
            if (count < 1 || addrs == NULL) return 2;
            for (i = 0; i < count; i++) {
                if (addrs[i] && strcmp(addrs[i], "127.0.0.1") == 0) found = 1;
                free(addrs[i]);
            }
            free(addrs);
            addrs = NULL;
            if (!found) return 3;

            err = draconic_rt_host_dns_lookup(
                "this-host-definitely-does-not-exist.invalid", &addrs, &count);
            if (err != DRACONIC_HOST_E_ADDR) return 4;
            if (addrs != NULL || count != 0) return 5;

            err = draconic_rt_host_dns_lookup("", &addrs, &count);
            if (err != DRACONIC_HOST_E_INVAL) return 6;
            err = draconic_rt_host_dns_lookup(NULL, &addrs, &count);
            if (err != DRACONIC_HOST_E_INVAL) return 7;
            err = draconic_rt_host_dns_lookup("127.0.0.1", NULL, &count);
            if (err != DRACONIC_HOST_E_INVAL) return 8;

            puts("dns-h0901-ok");
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
    assert!(status.success(), "clang failed for dns H09.01 smoke");

    let output = Command::new(&bin).output().expect("run dns h0901");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "dns H09.01 binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "dns-h0901-ok\n");
}
