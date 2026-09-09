//! GC heap allocation, stress, and mark-trace native tests.

use super::*;
use std::process::Command;

#[test]
fn gc_allocates_string_and_object_on_heap() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_gc_hello");
    std::fs::write(
        &main_c,
        r#"
        #include <stdio.h>
        #include <string.h>
        #include <stdint.h>

        typedef struct DraconicValue DraconicValue;

        void draconic_rt_gc_init(void);
        void draconic_rt_gc_shutdown(void);
        DraconicValue *draconic_rt_alloc_string(const char *data, size_t len);
        DraconicValue *draconic_rt_alloc_object(void);
        void draconic_rt_gc_root_push(DraconicValue *v);
        void draconic_rt_gc_root_pop(void);
        void draconic_rt_gc_collect(void);
        size_t draconic_rt_gc_live_count(void);
        const char *draconic_rt_string_data(DraconicValue *v);
        size_t draconic_rt_string_len(DraconicValue *v);
        int draconic_rt_is_object(DraconicValue *v);
        int draconic_rt_is_string(DraconicValue *v);

        int main(void) {
            draconic_rt_gc_init();

            DraconicValue *s = draconic_rt_alloc_string("hello", 5);
            if (!s || !draconic_rt_is_string(s)) {
                fprintf(stderr, "string alloc failed\n");
                return 1;
            }
            if (draconic_rt_string_len(s) != 5
                || memcmp(draconic_rt_string_data(s), "hello", 5) != 0) {
                fprintf(stderr, "string contents wrong\n");
                return 2;
            }

            DraconicValue *o = draconic_rt_alloc_object();
            if (!o || !draconic_rt_is_object(o)) {
                fprintf(stderr, "object alloc failed\n");
                return 3;
            }

            if (draconic_rt_gc_live_count() != 2) {
                fprintf(stderr, "live count want 2 got %zu\n",
                        draconic_rt_gc_live_count());
                return 4;
            }

            /* Root the string; leave object unrooted so collect reclaims it. */
            draconic_rt_gc_root_push(s);
            draconic_rt_gc_collect();

            if (draconic_rt_gc_live_count() != 1) {
                fprintf(stderr, "after collect live want 1 got %zu\n",
                        draconic_rt_gc_live_count());
                return 5;
            }
            if (draconic_rt_string_len(s) != 5
                || memcmp(draconic_rt_string_data(s), "hello", 5) != 0) {
                fprintf(stderr, "rooted string corrupted after collect\n");
                return 6;
            }

            draconic_rt_gc_root_pop();
            draconic_rt_gc_collect();
            if (draconic_rt_gc_live_count() != 0) {
                fprintf(stderr, "after unroot+collect live want 0 got %zu\n",
                        draconic_rt_gc_live_count());
                return 7;
            }

            puts("gc-hello-ok");
            draconic_rt_gc_shutdown();
            return 0;
        }
        "#,
    )
    .unwrap();

    let status = Command::new(&clang)
        .arg(&main_c)
        .arg(c_runtime_path())
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("spawn clang");
    assert!(status.success(), "clang failed to link runtime GC hello");

    let output = Command::new(&bin).output().expect("run rt_gc_hello");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "gc hello binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "gc-hello-ok\n", "stdout={stdout:?}");
}

/// N09.01: allocate many heap values, root a subset (≤64), collect, assert live_count.
#[test]
fn gc_stress_allocate_retain_drop_many_values() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_gc_stress");
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
        #include <stdint.h>

        /* Root stack max is 64 — keep K well under that. */
        enum { N_ALLOC = 512, K_ROOT = 32 };

        int main(void) {
            char buf[32];
            DraconicValue *kept[K_ROOT];
            size_t i;

            draconic_rt_gc_init();
            /* N09.01 measures explicit collect; disable N09.05 auto-collect. */
            draconic_rt_gc_set_alloc_threshold(0);

            /* Wave 1: many allocs (mix strings + empty objects); root only K. */
            for (i = 0; i < N_ALLOC; i++) {
                DraconicValue *v;
                if ((i & 1u) == 0) {
                    int n = snprintf(buf, sizeof(buf), "s%zu", i);
                    if (n < 0) {
                        fprintf(stderr, "snprintf failed\n");
                        return 1;
                    }
                    v = draconic_rt_alloc_string(buf, (size_t)n);
                    if (!v || !draconic_rt_is_string(v)) {
                        fprintf(stderr, "string alloc failed at %zu\n", i);
                        return 2;
                    }
                } else {
                    v = draconic_rt_alloc_object();
                    if (!v || !draconic_rt_is_object(v)) {
                        fprintf(stderr, "object alloc failed at %zu\n", i);
                        return 3;
                    }
                }
                if (i < K_ROOT) {
                    kept[i] = v;
                    draconic_rt_gc_root_push(v);
                }
            }

            if (draconic_rt_gc_live_count() != (size_t)N_ALLOC) {
                fprintf(stderr, "pre-collect live want %d got %zu\n",
                        N_ALLOC, draconic_rt_gc_live_count());
                return 4;
            }

            draconic_rt_gc_collect();

            if (draconic_rt_gc_live_count() != (size_t)K_ROOT) {
                fprintf(stderr, "after collect live want %d got %zu\n",
                        K_ROOT, draconic_rt_gc_live_count());
                return 5;
            }

            /* Rooted string at index 0 must remain intact. */
            if (!draconic_rt_is_string(kept[0])
                || draconic_rt_string_len(kept[0]) != 2
                || memcmp(draconic_rt_string_data(kept[0]), "s0", 2) != 0) {
                fprintf(stderr, "rooted string corrupted after collect\n");
                return 6;
            }

            for (i = 0; i < K_ROOT; i++) {
                draconic_rt_gc_root_pop();
            }
            draconic_rt_gc_collect();
            if (draconic_rt_gc_live_count() != 0) {
                fprintf(stderr, "after unroot live want 0 got %zu\n",
                        draconic_rt_gc_live_count());
                return 7;
            }

            /* Wave 2: churn again — prove heap recovers without crash/leak. */
            for (i = 0; i < N_ALLOC; i++) {
                DraconicValue *v;
                if ((i & 1u) == 0) {
                    int n = snprintf(buf, sizeof(buf), "t%zu", i);
                    if (n < 0) {
                        fprintf(stderr, "snprintf failed wave2\n");
                        return 8;
                    }
                    v = draconic_rt_alloc_string(buf, (size_t)n);
                } else {
                    v = draconic_rt_alloc_object();
                }
                if (!v) {
                    fprintf(stderr, "wave2 alloc failed at %zu\n", i);
                    return 9;
                }
                if (i < K_ROOT) {
                    kept[i] = v;
                    draconic_rt_gc_root_push(v);
                }
            }

            draconic_rt_gc_collect();
            if (draconic_rt_gc_live_count() != (size_t)K_ROOT) {
                fprintf(stderr, "wave2 collect live want %d got %zu\n",
                        K_ROOT, draconic_rt_gc_live_count());
                return 10;
            }
            if (!draconic_rt_is_string(kept[0])
                || draconic_rt_string_len(kept[0]) != 2
                || memcmp(draconic_rt_string_data(kept[0]), "t0", 2) != 0) {
                fprintf(stderr, "wave2 rooted string corrupted\n");
                return 11;
            }

            for (i = 0; i < K_ROOT; i++) {
                draconic_rt_gc_root_pop();
            }
            draconic_rt_gc_collect();
            if (draconic_rt_gc_live_count() != 0) {
                fprintf(stderr, "wave2 unroot live want 0 got %zu\n",
                        draconic_rt_gc_live_count());
                return 12;
            }

            puts("gc-stress-ok");
            draconic_rt_gc_shutdown();
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
    assert!(status.success(), "clang failed to link gc stress test");

    let output = Command::new(&bin).output().expect("run rt_gc_stress");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "gc stress binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "gc-stress-ok\n", "stdout={stdout:?}");
}

/// N09.02: rooted object must keep property-slot heap values live across collect.
///
/// Only the outer object is rooted. Own string-key props, symbol-key props,
/// nested object props, and [[Prototype]] hold other heap values that must
/// survive mark. Unreachable garbage must still be swept.
#[test]
fn gc_mark_traces_rooted_object_property_values() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_gc_mark_props");
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
        #include <stdint.h>

        int main(void) {
            draconic_rt_gc_init();

            /* Graph (only `root` is rooted):
             *   root.name  -> string "alice"
             *   root.child -> child
             *     child.x  -> string "nested"
             *   root[sym]  -> string "sym-val"   (symbol key)
             *   root.[[Prototype]] -> proto
             *     proto.p  -> string "from-proto"
             * Plus one unreachable garbage string.
             */
            DraconicValue *root = draconic_rt_alloc_object();
            DraconicValue *name = draconic_rt_alloc_string("alice", 5);
            DraconicValue *child = draconic_rt_alloc_object();
            DraconicValue *nested = draconic_rt_alloc_string("nested", 6);
            DraconicValue *sym_val = draconic_rt_alloc_string("sym-val", 7);
            DraconicValue *proto = draconic_rt_alloc_object();
            DraconicValue *from_proto = draconic_rt_alloc_string("from-proto", 10);
            DraconicValue *garbage = draconic_rt_alloc_string("garbage", 7);

            if (!root || !name || !child || !nested || !sym_val
                || !proto || !from_proto || !garbage) {
                fprintf(stderr, "alloc failed\n");
                return 1;
            }

            draconic_rt_object_set(root, "name", name);
            draconic_rt_object_set(root, "child", child);
            draconic_rt_object_set(child, "x", nested);
            draconic_rt_object_set_symbol(root, 42, sym_val);
            draconic_rt_object_set(proto, "p", from_proto);
            draconic_rt_object_set_proto(root, proto);

            /* Non-heap prop value must not crash mark (inttoptr-style). */
            draconic_rt_object_set(root, "tag", (void *)(intptr_t)99);

            if (draconic_rt_gc_live_count() != 8) {
                fprintf(stderr, "pre-collect live want 8 got %zu\n",
                        draconic_rt_gc_live_count());
                return 2;
            }

            /* Root only the outer object — not name/child/nested/etc. */
            draconic_rt_gc_root_push(root);
            draconic_rt_gc_collect();

            /* All 7 reachable heap values stay live; garbage is swept. */
            if (draconic_rt_gc_live_count() != 7) {
                fprintf(stderr, "after collect live want 7 got %zu\n",
                        draconic_rt_gc_live_count());
                return 3;
            }

            /* Property values remain readable and uncorrupted. */
            DraconicValue *got_name =
                (DraconicValue *)draconic_rt_object_get(root, "name");
            if (!draconic_rt_is_string(got_name)
                || draconic_rt_string_len(got_name) != 5
                || memcmp(draconic_rt_string_data(got_name), "alice", 5) != 0) {
                fprintf(stderr, "rooted prop name corrupted\n");
                return 4;
            }

            DraconicValue *got_child =
                (DraconicValue *)draconic_rt_object_get(root, "child");
            if (!draconic_rt_is_object(got_child)) {
                fprintf(stderr, "rooted prop child lost\n");
                return 5;
            }
            DraconicValue *got_nested =
                (DraconicValue *)draconic_rt_object_get(got_child, "x");
            if (!draconic_rt_is_string(got_nested)
                || draconic_rt_string_len(got_nested) != 6
                || memcmp(draconic_rt_string_data(got_nested), "nested", 6) != 0) {
                fprintf(stderr, "nested prop value corrupted\n");
                return 6;
            }

            DraconicValue *got_sym =
                (DraconicValue *)draconic_rt_object_get_symbol(root, 42);
            if (!draconic_rt_is_string(got_sym)
                || draconic_rt_string_len(got_sym) != 7
                || memcmp(draconic_rt_string_data(got_sym), "sym-val", 7) != 0) {
                fprintf(stderr, "symbol prop value corrupted\n");
                return 7;
            }

            DraconicValue *got_proto = draconic_rt_object_get_proto(root);
            if (!draconic_rt_is_object(got_proto)) {
                fprintf(stderr, "proto lost after collect\n");
                return 8;
            }
            /* [[Get]] walks prototype — proves proto + its prop stayed live. */
            DraconicValue *got_p =
                (DraconicValue *)draconic_rt_object_get(root, "p");
            if (!draconic_rt_is_string(got_p)
                || draconic_rt_string_len(got_p) != 10
                || memcmp(draconic_rt_string_data(got_p), "from-proto", 10) != 0) {
                fprintf(stderr, "proto prop value corrupted\n");
                return 9;
            }

            if ((intptr_t)draconic_rt_object_get(root, "tag") != 99) {
                fprintf(stderr, "non-heap prop tag corrupted\n");
                return 10;
            }

            draconic_rt_gc_root_pop();
            draconic_rt_gc_collect();
            if (draconic_rt_gc_live_count() != 0) {
                fprintf(stderr, "after unroot live want 0 got %zu\n",
                        draconic_rt_gc_live_count());
                return 11;
            }

            puts("gc-mark-props-ok");
            draconic_rt_gc_shutdown();
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
    assert!(status.success(), "clang failed to link gc mark props test");

    let output = Command::new(&bin).output().expect("run rt_gc_mark_props");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "gc mark props binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "gc-mark-props-ok\n", "stdout={stdout:?}");
}
