//! GC cycle collection, root-stack growth, and alloc-pressure native tests.

use super::*;
use std::process::Command;

/// N09.03: mark-sweep must reclaim unrooted cycles and keep rooted cycles live.
///
/// Graphs: object↔object props, array↔array elems, object.[[Prototype]] cycle,
/// and a 3-node ring. Unroot + collect → live_count 0; root one member → whole
/// cycle stays live and readable.
#[test]
fn gc_cycles_mutual_refs_collect_and_retain() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_gc_cycles");
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

            /* --- 1. Object mutual cycle (a.other = b, b.other = a); unrooted --- */
            {
                DraconicValue *a = draconic_rt_alloc_object();
                DraconicValue *b = draconic_rt_alloc_object();
                DraconicValue *label = draconic_rt_alloc_string("ab", 2);
                if (!a || !b || !label) {
                    fprintf(stderr, "obj cycle alloc failed\n");
                    return 1;
                }
                draconic_rt_object_set(a, "other", b);
                draconic_rt_object_set(b, "other", a);
                draconic_rt_object_set(a, "label", label);
                if (draconic_rt_gc_live_count() != 3) {
                    fprintf(stderr, "obj cycle pre live want 3 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 2;
                }
                /* No roots — whole cycle is garbage. */
                draconic_rt_gc_collect();
                if (draconic_rt_gc_live_count() != 0) {
                    fprintf(stderr, "obj cycle unrooted live want 0 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 3;
                }
            }

            /* --- 2. Same mutual cycle, root one node → both + label stay live --- */
            {
                DraconicValue *a = draconic_rt_alloc_object();
                DraconicValue *b = draconic_rt_alloc_object();
                DraconicValue *label = draconic_rt_alloc_string("keep", 4);
                if (!a || !b || !label) {
                    fprintf(stderr, "rooted obj cycle alloc failed\n");
                    return 4;
                }
                draconic_rt_object_set(a, "other", b);
                draconic_rt_object_set(b, "other", a);
                draconic_rt_object_set(b, "label", label);
                draconic_rt_gc_root_push(a);
                draconic_rt_gc_collect();
                if (draconic_rt_gc_live_count() != 3) {
                    fprintf(stderr, "rooted obj cycle live want 3 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 5;
                }
                DraconicValue *got_b =
                    (DraconicValue *)draconic_rt_object_get(a, "other");
                if (!draconic_rt_is_object(got_b)) {
                    fprintf(stderr, "cycle edge a->b lost\n");
                    return 6;
                }
                DraconicValue *got_a =
                    (DraconicValue *)draconic_rt_object_get(got_b, "other");
                if (got_a != a) {
                    fprintf(stderr, "cycle edge b->a broken\n");
                    return 7;
                }
                DraconicValue *got_label =
                    (DraconicValue *)draconic_rt_object_get(got_b, "label");
                if (!draconic_rt_is_string(got_label)
                    || draconic_rt_string_len(got_label) != 4
                    || memcmp(draconic_rt_string_data(got_label), "keep", 4) != 0) {
                    fprintf(stderr, "cycle payload label corrupted\n");
                    return 8;
                }
                draconic_rt_gc_root_pop();
                draconic_rt_gc_collect();
                if (draconic_rt_gc_live_count() != 0) {
                    fprintf(stderr, "after unroot obj cycle live want 0 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 9;
                }
            }

            /* --- 3. Array mutual cycle via elems[0] --- */
            {
                DraconicValue *x = draconic_rt_array_new(1);
                DraconicValue *y = draconic_rt_array_new(1);
                if (!x || !y) {
                    fprintf(stderr, "array cycle alloc failed\n");
                    return 10;
                }
                draconic_rt_array_set(x, 0, y);
                draconic_rt_array_set(y, 0, x);
                if (draconic_rt_gc_live_count() != 2) {
                    fprintf(stderr, "array cycle pre live want 2 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 11;
                }
                draconic_rt_gc_collect();
                if (draconic_rt_gc_live_count() != 0) {
                    fprintf(stderr, "array cycle unrooted live want 0 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 12;
                }

                x = draconic_rt_array_new(1);
                y = draconic_rt_array_new(1);
                if (!x || !y) {
                    fprintf(stderr, "rooted array cycle alloc failed\n");
                    return 13;
                }
                draconic_rt_array_set(x, 0, y);
                draconic_rt_array_set(y, 0, x);
                draconic_rt_gc_root_push(x);
                draconic_rt_gc_collect();
                if (draconic_rt_gc_live_count() != 2) {
                    fprintf(stderr, "rooted array cycle live want 2 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 14;
                }
                if ((DraconicValue *)draconic_rt_array_get(x, 0) != y
                    || (DraconicValue *)draconic_rt_array_get(y, 0) != x) {
                    fprintf(stderr, "array cycle edges corrupted\n");
                    return 15;
                }
                draconic_rt_gc_root_pop();
                draconic_rt_gc_collect();
                if (draconic_rt_gc_live_count() != 0) {
                    fprintf(stderr, "after unroot array cycle live want 0 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 16;
                }
            }

            /* --- 4. Prototype cycle: a.[[Prototype]] = b, b.[[Prototype]] = a --- */
            {
                DraconicValue *a = draconic_rt_alloc_object();
                DraconicValue *b = draconic_rt_alloc_object();
                if (!a || !b) {
                    fprintf(stderr, "proto cycle alloc failed\n");
                    return 17;
                }
                draconic_rt_object_set_proto(a, b);
                draconic_rt_object_set_proto(b, a);
                draconic_rt_gc_collect();
                if (draconic_rt_gc_live_count() != 0) {
                    fprintf(stderr, "proto cycle unrooted live want 0 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 18;
                }

                a = draconic_rt_alloc_object();
                b = draconic_rt_alloc_object();
                if (!a || !b) {
                    fprintf(stderr, "rooted proto cycle alloc failed\n");
                    return 19;
                }
                draconic_rt_object_set_proto(a, b);
                draconic_rt_object_set_proto(b, a);
                draconic_rt_gc_root_push(a);
                draconic_rt_gc_collect();
                if (draconic_rt_gc_live_count() != 2) {
                    fprintf(stderr, "rooted proto cycle live want 2 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 20;
                }
                if (draconic_rt_object_get_proto(a) != b
                    || draconic_rt_object_get_proto(b) != a) {
                    fprintf(stderr, "proto cycle edges corrupted\n");
                    return 21;
                }
                draconic_rt_gc_root_pop();
                draconic_rt_gc_collect();
                if (draconic_rt_gc_live_count() != 0) {
                    fprintf(stderr, "after unroot proto cycle live want 0 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 22;
                }
            }

            /* --- 5. Three-node ring + payload; root middle --- */
            {
                DraconicValue *p = draconic_rt_alloc_object();
                DraconicValue *q = draconic_rt_alloc_object();
                DraconicValue *r = draconic_rt_alloc_object();
                DraconicValue *pay = draconic_rt_alloc_string("ring", 4);
                if (!p || !q || !r || !pay) {
                    fprintf(stderr, "ring alloc failed\n");
                    return 23;
                }
                draconic_rt_object_set(p, "next", q);
                draconic_rt_object_set(q, "next", r);
                draconic_rt_object_set(r, "next", p);
                draconic_rt_object_set(r, "pay", pay);
                draconic_rt_gc_root_push(q);
                draconic_rt_gc_collect();
                if (draconic_rt_gc_live_count() != 4) {
                    fprintf(stderr, "rooted ring live want 4 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 24;
                }
                DraconicValue *got_r =
                    (DraconicValue *)draconic_rt_object_get(q, "next");
                DraconicValue *got_p =
                    (DraconicValue *)draconic_rt_object_get(got_r, "next");
                DraconicValue *got_q =
                    (DraconicValue *)draconic_rt_object_get(got_p, "next");
                if (got_q != q) {
                    fprintf(stderr, "ring walk broken\n");
                    return 25;
                }
                DraconicValue *got_pay =
                    (DraconicValue *)draconic_rt_object_get(got_r, "pay");
                if (!draconic_rt_is_string(got_pay)
                    || draconic_rt_string_len(got_pay) != 4
                    || memcmp(draconic_rt_string_data(got_pay), "ring", 4) != 0) {
                    fprintf(stderr, "ring payload corrupted\n");
                    return 26;
                }
                draconic_rt_gc_root_pop();
                draconic_rt_gc_collect();
                if (draconic_rt_gc_live_count() != 0) {
                    fprintf(stderr, "after unroot ring live want 0 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 27;
                }
            }

            puts("gc-cycles-ok");
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
    assert!(status.success(), "clang failed to link gc cycles test");

    let output = Command::new(&bin).output().expect("run rt_gc_cycles");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "gc cycles binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "gc-cycles-ok\n", "stdout={stdout:?}");
}

/// N09.04: root stack must grow past the historic fixed 64 limit without abort.
///
/// Push N_ROOT (>64) distinct heap values, collect (all stay live), verify
/// payloads, pop all, collect → live_count 0. Nested deep push/pop churn
/// must not corrupt the stack.
#[test]
fn gc_root_stack_grows_beyond_fixed_limit() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_gc_root_stack");
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

        /* Historic fixed limit was 64 — exceed it. */
        enum { N_ROOT = 200 };

        int main(void) {
            char buf[32];
            DraconicValue *kept[N_ROOT];
            size_t i;

            draconic_rt_gc_init();

            /* --- 1. Push N_ROOT roots (forces growth past 64) --- */
            for (i = 0; i < N_ROOT; i++) {
                int n = snprintf(buf, sizeof(buf), "r%zu", i);
                if (n < 0) {
                    fprintf(stderr, "snprintf failed\n");
                    return 1;
                }
                kept[i] = draconic_rt_alloc_string(buf, (size_t)n);
                if (!kept[i] || !draconic_rt_is_string(kept[i])) {
                    fprintf(stderr, "alloc failed at %zu\n", i);
                    return 2;
                }
                draconic_rt_gc_root_push(kept[i]);
            }

            if (draconic_rt_gc_live_count() != (size_t)N_ROOT) {
                fprintf(stderr, "pre-collect live want %d got %zu\n",
                        N_ROOT, draconic_rt_gc_live_count());
                return 3;
            }

            draconic_rt_gc_collect();

            if (draconic_rt_gc_live_count() != (size_t)N_ROOT) {
                fprintf(stderr, "after collect live want %d got %zu\n",
                        N_ROOT, draconic_rt_gc_live_count());
                return 4;
            }

            /* Spot-check first, mid (past old 64), and last roots. */
            if (!draconic_rt_is_string(kept[0])
                || draconic_rt_string_len(kept[0]) != 2
                || memcmp(draconic_rt_string_data(kept[0]), "r0", 2) != 0) {
                fprintf(stderr, "root 0 corrupted\n");
                return 5;
            }
            if (!draconic_rt_is_string(kept[100])
                || draconic_rt_string_len(kept[100]) != 4
                || memcmp(draconic_rt_string_data(kept[100]), "r100", 4) != 0) {
                fprintf(stderr, "root 100 corrupted\n");
                return 6;
            }
            if (!draconic_rt_is_string(kept[N_ROOT - 1])
                || draconic_rt_string_len(kept[N_ROOT - 1]) != 4
                || memcmp(draconic_rt_string_data(kept[N_ROOT - 1]), "r199", 4) != 0) {
                fprintf(stderr, "root last corrupted\n");
                return 7;
            }

            for (i = 0; i < N_ROOT; i++) {
                draconic_rt_gc_root_pop();
            }
            draconic_rt_gc_collect();
            if (draconic_rt_gc_live_count() != 0) {
                fprintf(stderr, "after unroot live want 0 got %zu\n",
                        draconic_rt_gc_live_count());
                return 8;
            }

            /* --- 2. Nested deep push/pop churn (grow, shrink, grow again) --- */
            for (i = 0; i < 80; i++) {
                int n = snprintf(buf, sizeof(buf), "a%zu", i);
                if (n < 0) {
                    fprintf(stderr, "snprintf a failed\n");
                    return 9;
                }
                DraconicValue *v = draconic_rt_alloc_string(buf, (size_t)n);
                if (!v) {
                    fprintf(stderr, "wave a alloc failed\n");
                    return 10;
                }
                draconic_rt_gc_root_push(v);
            }
            for (i = 0; i < 40; i++) {
                draconic_rt_gc_root_pop();
            }
            draconic_rt_gc_collect();
            if (draconic_rt_gc_live_count() != 40) {
                fprintf(stderr, "mid churn live want 40 got %zu\n",
                        draconic_rt_gc_live_count());
                return 11;
            }
            for (i = 0; i < 120; i++) {
                int n = snprintf(buf, sizeof(buf), "b%zu", i);
                if (n < 0) {
                    fprintf(stderr, "snprintf b failed\n");
                    return 12;
                }
                DraconicValue *v = draconic_rt_alloc_string(buf, (size_t)n);
                if (!v) {
                    fprintf(stderr, "wave b alloc failed\n");
                    return 13;
                }
                draconic_rt_gc_root_push(v);
            }
            /* 40 retained + 120 new = 160 roots */
            draconic_rt_gc_collect();
            if (draconic_rt_gc_live_count() != 160) {
                fprintf(stderr, "deep churn live want 160 got %zu\n",
                        draconic_rt_gc_live_count());
                return 14;
            }
            for (i = 0; i < 160; i++) {
                draconic_rt_gc_root_pop();
            }
            draconic_rt_gc_collect();
            if (draconic_rt_gc_live_count() != 0) {
                fprintf(stderr, "final unroot live want 0 got %zu\n",
                        draconic_rt_gc_live_count());
                return 15;
            }

            puts("gc-root-stack-ok");
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
    assert!(status.success(), "clang failed to link gc root stack test");

    let output = Command::new(&bin).output().expect("run rt_gc_root_stack");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "gc root stack binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "gc-root-stack-ok\n", "stdout={stdout:?}");
}

/// N09.05: alloc-path threshold triggers collect without explicit gc_collect.
///
/// With a low threshold, many unrooted allocs must reclaim garbage so
/// live_count stays bounded near the rooted set; rooted payloads survive.
#[test]
fn gc_auto_collect_on_alloc_pressure() {
    let clang = test_which_clang().expect("clang required for runtime native tests");
    let dir = test_tempfile_dir();
    let archive = build_runtime_static_lib(&dir).expect("build static lib");
    let main_c = dir.join("main.c");
    let bin = dir.join("rt_gc_auto");
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

        enum { THRESHOLD = 32, N_ALLOC = 400, K_ROOT = 8 };

        int main(void) {
            char buf[32];
            DraconicValue *kept[K_ROOT];
            size_t i;
            size_t peak_live = 0;
            size_t live;

            draconic_rt_gc_init();
            draconic_rt_gc_set_alloc_threshold(THRESHOLD);
            if (draconic_rt_gc_alloc_threshold() != (size_t)THRESHOLD) {
                fprintf(stderr, "threshold getter mismatch\n");
                return 1;
            }

            /* Root a small set first so they stay live across auto-collects. */
            for (i = 0; i < K_ROOT; i++) {
                int n = snprintf(buf, sizeof(buf), "keep%zu", i);
                if (n < 0) {
                    fprintf(stderr, "snprintf keep failed\n");
                    return 2;
                }
                kept[i] = draconic_rt_alloc_string(buf, (size_t)n);
                if (!kept[i]) {
                    fprintf(stderr, "keep alloc failed\n");
                    return 3;
                }
                draconic_rt_gc_root_push(kept[i]);
            }

            /* Flood unrooted garbage — no explicit collect. */
            for (i = 0; i < N_ALLOC; i++) {
                DraconicValue *v;
                if ((i & 1u) == 0) {
                    int n = snprintf(buf, sizeof(buf), "g%zu", i);
                    if (n < 0) {
                        fprintf(stderr, "snprintf garbage failed\n");
                        return 4;
                    }
                    v = draconic_rt_alloc_string(buf, (size_t)n);
                } else {
                    v = draconic_rt_alloc_object();
                }
                if (!v) {
                    fprintf(stderr, "garbage alloc failed at %zu\n", i);
                    return 5;
                }
                live = draconic_rt_gc_live_count();
                if (live > peak_live) {
                    peak_live = live;
                }
            }

            live = draconic_rt_gc_live_count();
            /* Must have auto-collected: live far below K_ROOT + N_ALLOC. */
            if (live > (size_t)(K_ROOT + THRESHOLD + 8)) {
                fprintf(stderr, "live not bounded: live=%zu peak=%zu\n",
                        live, peak_live);
                return 6;
            }
            if (peak_live > (size_t)(K_ROOT + THRESHOLD + 8)) {
                fprintf(stderr, "peak not bounded: peak=%zu\n", peak_live);
                return 7;
            }
            /* Roots must still be intact without any explicit collect. */
            for (i = 0; i < K_ROOT; i++) {
                char expect[32];
                int n = snprintf(expect, sizeof(expect), "keep%zu", i);
                if (n < 0
                    || !draconic_rt_is_string(kept[i])
                    || draconic_rt_string_len(kept[i]) != (size_t)n
                    || memcmp(draconic_rt_string_data(kept[i]), expect, (size_t)n) != 0) {
                    fprintf(stderr, "rooted keep%zu corrupted\n", i);
                    return 8;
                }
            }

            /* Threshold 0 disables auto-collect: live can grow unbounded. */
            draconic_rt_gc_set_alloc_threshold(0);
            {
                size_t before = draconic_rt_gc_live_count();
                for (i = 0; i < 80; i++) {
                    if (!draconic_rt_alloc_object()) {
                        fprintf(stderr, "disable-path alloc failed\n");
                        return 9;
                    }
                }
                live = draconic_rt_gc_live_count();
                if (live < before + 80) {
                    fprintf(stderr, "threshold 0 still collected: before=%zu live=%zu\n",
                            before, live);
                    return 10;
                }
            }

            for (i = 0; i < K_ROOT; i++) {
                draconic_rt_gc_root_pop();
            }
            draconic_rt_gc_collect();
            if (draconic_rt_gc_live_count() != 0) {
                fprintf(stderr, "final live want 0 got %zu\n",
                        draconic_rt_gc_live_count());
                return 11;
            }

            puts("gc-auto-ok");
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
    assert!(
        status.success(),
        "clang failed to link gc auto-collect test"
    );

    let output = Command::new(&bin).output().expect("run rt_gc_auto");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "gc auto-collect binary failed: {:?}\nstderr={stderr}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "gc-auto-ok\n", "stdout={stdout:?}");
}
