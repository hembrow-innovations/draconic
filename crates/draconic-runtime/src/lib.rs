//! Native Runtime: GC + minimal std (N05) + job queue (N06.01) + Promise ABI (N06.02–N06.09); embed later (N07).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Path to the Runtime C translation unit (`draconic_rt.c`).
pub fn c_runtime_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/draconic_rt.c")
}

/// Path to the public Runtime C header (`draconic_rt.h`).
pub fn c_runtime_header_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/draconic_rt.h")
}

/// C source for the Runtime (embedded for tests and tooling).
pub fn c_runtime_source() -> &'static str {
    include_str!("draconic_rt.c")
}

/// C header for the Runtime ABI (embedded for tests and tooling).
pub fn c_runtime_header_source() -> &'static str {
    include_str!("draconic_rt.h")
}

/// Print the Runtime hello line (`hello` + newline) to stdout.
pub fn print_hello() {
    println!("hello");
}

/// C ABI symbol name expected by the LLVM backend stub.
pub const HELLO_SYMBOL: &str = "draconic_rt_hello";
/// C ABI: print a signed 64-bit integer as decimal + newline (N01).
pub const PRINT_I64_SYMBOL: &str = "draconic_rt_print_i64";
/// C ABI: print an unsigned 64-bit integer as decimal + newline (N01).
pub const PRINT_U64_SYMBOL: &str = "draconic_rt_print_u64";
/// C ABI: print a float/double as decimal + newline (N02).
pub const PRINT_F64_SYMBOL: &str = "draconic_rt_print_f64";
/// C ABI: print a bool as `true`/`false` + newline (N02).
pub const PRINT_BOOL_SYMBOL: &str = "draconic_rt_print_bool";
/// C ABI: print a NUL-terminated C string + newline (N06.03).
pub const PRINT_STR_SYMBOL: &str = "draconic_rt_print_str";

/// C ABI: init the GC heap.
pub const GC_INIT_SYMBOL: &str = "draconic_rt_gc_init";
/// C ABI: shut down the GC heap.
pub const GC_SHUTDOWN_SYMBOL: &str = "draconic_rt_gc_shutdown";
/// C ABI: allocate a JS string on the GC heap.
pub const ALLOC_STRING_SYMBOL: &str = "draconic_rt_alloc_string";
/// C ABI: allocate a plain JS object on the GC heap.
pub const ALLOC_OBJECT_SYMBOL: &str = "draconic_rt_alloc_object";
/// C ABI: push a GC root (keeps a value live across collect).
pub const GC_ROOT_PUSH_SYMBOL: &str = "draconic_rt_gc_root_push";
/// C ABI: pop a GC root.
pub const GC_ROOT_POP_SYMBOL: &str = "draconic_rt_gc_root_pop";
/// C ABI: run a tracing collection.
pub const GC_COLLECT_SYMBOL: &str = "draconic_rt_gc_collect";
/// C ABI: live object count on the GC heap (for tests / diagnostics).
pub const GC_LIVE_COUNT_SYMBOL: &str = "draconic_rt_gc_live_count";
/// C ABI: string payload pointer.
pub const STRING_DATA_SYMBOL: &str = "draconic_rt_string_data";
/// C ABI: string length in bytes.
pub const STRING_LEN_SYMBOL: &str = "draconic_rt_string_len";
/// C ABI: tag predicate for strings.
pub const IS_STRING_SYMBOL: &str = "draconic_rt_is_string";
/// C ABI: tag predicate for objects.
pub const IS_OBJECT_SYMBOL: &str = "draconic_rt_is_object";

/// C ABI: enqueue a host job (Promise Jobs / microtasks; N06.01).
pub const JOB_ENQUEUE_SYMBOL: &str = "draconic_rt_job_enqueue";
/// C ABI: drain the job queue until empty (FIFO; nested enqueue runs after current).
pub const JOB_DRAIN_SYMBOL: &str = "draconic_rt_job_drain";
/// C ABI: number of pending (not yet run) jobs.
pub const JOB_PENDING_SYMBOL: &str = "draconic_rt_job_pending";

/// Minimal std I/O + GC ABI symbols that form the N05 Runtime surface.
pub const MINIMAL_STD_AND_GC_SYMBOLS: &[&str] = &[
    HELLO_SYMBOL,
    PRINT_I64_SYMBOL,
    PRINT_U64_SYMBOL,
    PRINT_F64_SYMBOL,
    PRINT_BOOL_SYMBOL,
    PRINT_STR_SYMBOL,
    GC_INIT_SYMBOL,
    GC_SHUTDOWN_SYMBOL,
    ALLOC_STRING_SYMBOL,
    ALLOC_OBJECT_SYMBOL,
    GC_ROOT_PUSH_SYMBOL,
    GC_ROOT_POP_SYMBOL,
    GC_COLLECT_SYMBOL,
    GC_LIVE_COUNT_SYMBOL,
    STRING_DATA_SYMBOL,
    STRING_LEN_SYMBOL,
    IS_STRING_SYMBOL,
    IS_OBJECT_SYMBOL,
];

/// Job queue ABI symbols (N06.01).
pub const JOB_QUEUE_SYMBOLS: &[&str] = &[
    JOB_ENQUEUE_SYMBOL,
    JOB_DRAIN_SYMBOL,
    JOB_PENDING_SYMBOL,
];

/// C ABI: allocate a pending Promise (N06.02).
pub const PROMISE_NEW_SYMBOL: &str = "draconic_rt_promise_new";
/// C ABI: tag predicate for Promise values.
pub const IS_PROMISE_SYMBOL: &str = "draconic_rt_is_promise";
/// C ABI: Promise state — 0 pending, 1 fulfilled, 2 rejected.
pub const PROMISE_STATE_SYMBOL: &str = "draconic_rt_promise_state";
/// C ABI: Promise result (fulfillment value or rejection reason).
pub const PROMISE_RESULT_SYMBOL: &str = "draconic_rt_promise_result";
/// C ABI: fulfill a Promise once (second settle is a no-op).
pub const PROMISE_RESOLVE_SYMBOL: &str = "draconic_rt_promise_resolve";
/// C ABI: reject a Promise once (second settle is a no-op).
pub const PROMISE_REJECT_SYMBOL: &str = "draconic_rt_promise_reject";
/// C ABI: attach then reactions; returns a derived Promise for chaining.
pub const PROMISE_THEN_SYMBOL: &str = "draconic_rt_promise_then";
/// C ABI: `new Promise(executor)` — construct + invoke executor with settle caps (N06.03).
pub const PROMISE_CONSTRUCT_SYMBOL: &str = "draconic_rt_promise_construct";
/// C ABI: `Promise.prototype.finally` — pass-through settle after side-effect callback (N06.05).
pub const PROMISE_FINALLY_SYMBOL: &str = "draconic_rt_promise_finally";
/// C ABI: allocate a JS array of `len` slots (N06.06).
pub const ARRAY_NEW_SYMBOL: &str = "draconic_rt_array_new";
/// C ABI: tag predicate for array values.
pub const IS_ARRAY_SYMBOL: &str = "draconic_rt_is_array";
/// C ABI: array `.length`.
pub const ARRAY_LEN_SYMBOL: &str = "draconic_rt_array_len";
/// C ABI: array index get.
pub const ARRAY_GET_SYMBOL: &str = "draconic_rt_array_get";
/// C ABI: array index set.
pub const ARRAY_SET_SYMBOL: &str = "draconic_rt_array_set";
/// C ABI: `Promise.all(iterable)` — array of promises/values (N06.06).
pub const PROMISE_ALL_SYMBOL: &str = "draconic_rt_promise_all";
/// C ABI: `Promise.race(iterable)` — first settle wins (N06.07).
pub const PROMISE_RACE_SYMBOL: &str = "draconic_rt_promise_race";
/// C ABI: object property get by NUL-terminated key (N06.08).
pub const OBJECT_GET_SYMBOL: &str = "draconic_rt_object_get";
/// C ABI: object property set by NUL-terminated key (N06.08).
pub const OBJECT_SET_SYMBOL: &str = "draconic_rt_object_set";
/// C ABI: `Promise.allSettled(iterable)` — array of status objects (N06.08).
pub const PROMISE_ALL_SETTLED_SYMBOL: &str = "draconic_rt_promise_all_settled";
/// C ABI: `Promise.any(iterable)` — first fulfillment or AggregateError (N06.09).
pub const PROMISE_ANY_SYMBOL: &str = "draconic_rt_promise_any";

/// Promise ABI symbols (N06.02–N06.09).
pub const PROMISE_SYMBOLS: &[&str] = &[
    PROMISE_NEW_SYMBOL,
    IS_PROMISE_SYMBOL,
    PROMISE_STATE_SYMBOL,
    PROMISE_RESULT_SYMBOL,
    PROMISE_RESOLVE_SYMBOL,
    PROMISE_REJECT_SYMBOL,
    PROMISE_THEN_SYMBOL,
    PROMISE_CONSTRUCT_SYMBOL,
    PROMISE_FINALLY_SYMBOL,
    ARRAY_NEW_SYMBOL,
    IS_ARRAY_SYMBOL,
    ARRAY_LEN_SYMBOL,
    ARRAY_GET_SYMBOL,
    ARRAY_SET_SYMBOL,
    PROMISE_ALL_SYMBOL,
    PROMISE_RACE_SYMBOL,
    OBJECT_GET_SYMBOL,
    OBJECT_SET_SYMBOL,
    PROMISE_ALL_SETTLED_SYMBOL,
    PROMISE_ANY_SYMBOL,
];

/// Build `libdraconic_rt.a` in `out_dir` (clang `-c` + `ar`).
///
/// Returns the path to the static archive. Callers link with the archive path
/// (or `-L`/`-ldraconic_rt`) instead of recompiling `draconic_rt.c` each time.
pub fn build_runtime_static_lib(out_dir: &Path) -> Result<PathBuf, String> {
    let clang = find_clang().ok_or_else(|| {
        "clang not found (set CLANG or install a C toolchain)".to_string()
    })?;
    let ar = find_ar().ok_or_else(|| "ar not found (set AR or install binutils)".to_string())?;

    let rt_c = c_runtime_path();
    if !rt_c.is_file() {
        return Err(format!("runtime C source missing: {}", rt_c.display()));
    }

    std::fs::create_dir_all(out_dir).map_err(|e| format!("create out_dir failed: {e}"))?;

    let obj = out_dir.join("draconic_rt.o");
    let archive = out_dir.join("libdraconic_rt.a");

    let compile = Command::new(&clang)
        .arg("-c")
        .arg(&rt_c)
        .arg("-o")
        .arg(&obj)
        .arg("-I")
        .arg(c_runtime_header_path().parent().unwrap_or_else(|| Path::new(".")))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("spawn clang failed: {e}"))?;
    if !compile.status.success() {
        let stderr = String::from_utf8_lossy(&compile.stderr);
        return Err(format!("clang -c failed: {stderr}"));
    }

    let archive_out = Command::new(&ar)
        .arg("rcs")
        .arg(&archive)
        .arg(&obj)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("spawn ar failed: {e}"))?;
    if !archive_out.status.success() {
        let stderr = String::from_utf8_lossy(&archive_out.stderr);
        return Err(format!("ar rcs failed: {stderr}"));
    }

    if !archive.is_file() {
        return Err(format!("static lib missing after ar: {}", archive.display()));
    }
    Ok(archive)
}

fn find_clang() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CLANG") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    for candidate in [
        "clang",
        "/usr/bin/clang",
        "/opt/homebrew/opt/llvm@22/bin/clang",
        "/opt/homebrew/opt/llvm/bin/clang",
    ] {
        let ok = Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return Some(PathBuf::from(candidate));
        }
    }
    None
}

fn find_ar() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("AR") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    for candidate in ["ar", "/usr/bin/ar", "llvm-ar", "/opt/homebrew/opt/llvm/bin/llvm-ar"] {
        let ok = Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or_else(|_| {
                // GNU ar often has no --version; try -V or bare existence via `ar`.
                Command::new(candidate)
                    .arg("-V")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            });
        if ok {
            return Some(PathBuf::from(candidate));
        }
        // macOS ar accepts `rcs` without a version flag; probe with `which`-style run.
        let probe = Command::new(candidate)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if probe.is_ok() {
            return Some(PathBuf::from(candidate));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn c_runtime_exports_hello() {
        let src = c_runtime_source();
        assert!(
            src.contains(HELLO_SYMBOL),
            "C runtime must export {HELLO_SYMBOL}"
        );
        assert!(
            src.contains("puts(\"hello\")"),
            "C runtime must print hello: {src}"
        );
        assert!(c_runtime_path().is_file(), "draconic_rt.c must exist on disk");
    }

    #[test]
    fn c_runtime_exports_print_ints() {
        let src = c_runtime_source();
        assert!(
            src.contains(PRINT_I64_SYMBOL),
            "C runtime must export {PRINT_I64_SYMBOL}"
        );
        assert!(
            src.contains(PRINT_U64_SYMBOL),
            "C runtime must export {PRINT_U64_SYMBOL}"
        );
    }

    #[test]
    fn c_runtime_exports_print_float_bool() {
        let src = c_runtime_source();
        assert!(
            src.contains(PRINT_F64_SYMBOL),
            "C runtime must export {PRINT_F64_SYMBOL}"
        );
        assert!(
            src.contains(PRINT_BOOL_SYMBOL),
            "C runtime must export {PRINT_BOOL_SYMBOL}"
        );
    }

    #[test]
    fn c_runtime_exports_gc_abi() {
        let src = c_runtime_source();
        for sym in [
            GC_INIT_SYMBOL,
            GC_SHUTDOWN_SYMBOL,
            ALLOC_STRING_SYMBOL,
            ALLOC_OBJECT_SYMBOL,
            GC_ROOT_PUSH_SYMBOL,
            GC_ROOT_POP_SYMBOL,
            GC_COLLECT_SYMBOL,
            GC_LIVE_COUNT_SYMBOL,
        ] {
            assert!(src.contains(sym), "C runtime must export {sym}");
        }
    }

    #[test]
    fn minimal_std_and_gc_symbols_present_in_source_and_header() {
        let src = c_runtime_source();
        let hdr = c_runtime_header_source();
        assert!(
            c_runtime_header_path().is_file(),
            "draconic_rt.h must exist on disk"
        );
        for sym in MINIMAL_STD_AND_GC_SYMBOLS {
            assert!(src.contains(sym), "C runtime source must define {sym}");
            assert!(hdr.contains(sym), "C runtime header must declare {sym}");
        }
    }

    #[test]
    fn builds_runtime_static_library() {
        let dir = tempfile_dir();
        let archive = build_runtime_static_lib(&dir).expect("build static lib");
        assert!(
            archive.is_file(),
            "expected archive at {}",
            archive.display()
        );
        assert!(
            archive
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n == "libdraconic_rt.a"),
            "archive name: {}",
            archive.display()
        );
        let meta = std::fs::metadata(&archive).expect("stat archive");
        assert!(meta.len() > 0, "archive must be non-empty");
    }

    #[test]
    fn links_static_lib_gc_and_minimal_std() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
        let archive = build_runtime_static_lib(&dir).expect("build static lib");

        let main_c = dir.join("main.c");
        let bin = dir.join("rt_link_n05");
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

            int main(void) {
                draconic_rt_gc_init();

                DraconicValue *s = draconic_rt_alloc_string("n05", 3);
                if (!s || !draconic_rt_is_string(s)) {
                    fprintf(stderr, "string alloc failed\n");
                    return 1;
                }
                if (draconic_rt_string_len(s) != 3
                    || memcmp(draconic_rt_string_data(s), "n05", 3) != 0) {
                    fprintf(stderr, "string contents wrong\n");
                    return 2;
                }

                DraconicValue *o = draconic_rt_alloc_object();
                if (!o || !draconic_rt_is_object(o)) {
                    fprintf(stderr, "object alloc failed\n");
                    return 3;
                }
                if (draconic_rt_gc_live_count() != 2) {
                    fprintf(stderr, "live want 2 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 4;
                }

                draconic_rt_gc_root_push(s);
                draconic_rt_gc_collect();
                if (draconic_rt_gc_live_count() != 1) {
                    fprintf(stderr, "after collect live want 1 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 5;
                }

                /* Minimal std: print hooks + hello */
                draconic_rt_print_i64(42);
                draconic_rt_print_bool(1);
                draconic_rt_hello();

                draconic_rt_gc_root_pop();
                draconic_rt_gc_collect();
                if (draconic_rt_gc_live_count() != 0) {
                    fprintf(stderr, "after unroot live want 0 got %zu\n",
                            draconic_rt_gc_live_count());
                    return 6;
                }

                draconic_rt_gc_shutdown();
                return 0;
            }
            "#,
        )
        .unwrap();

        /* Link consumer against the archive only — not draconic_rt.c. */
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
            "clang failed to link against libdraconic_rt.a"
        );

        let output = Command::new(&bin).output().expect("run rt_link_n05");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "n05 link binary failed: {:?}\nstderr={stderr}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "42\ntrue\nhello\n", "stdout={stdout:?}");
    }

    #[test]
    fn c_runtime_compiles_and_prints_hello() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
        let main_c = dir.join("main.c");
        let bin = dir.join("rt_hello");
        std::fs::write(
            &main_c,
            r#"
            void draconic_rt_hello(void);
            int main(void) { draconic_rt_hello(); return 0; }
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
        assert!(status.success(), "clang failed to link runtime");

        let output = Command::new(&bin).output().expect("run rt_hello");
        assert!(output.status.success(), "binary failed: {:?}", output.status);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "hello\n", "stdout={stdout:?}");
    }

    #[test]
    fn gc_allocates_string_and_object_on_heap() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
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

    #[test]
    fn print_hello_smoke() {
        print_hello();
    }

    #[test]
    fn c_runtime_exports_job_queue_abi() {
        let src = c_runtime_source();
        let hdr = c_runtime_header_source();
        for sym in JOB_QUEUE_SYMBOLS {
            assert!(src.contains(sym), "C runtime source must define {sym}");
            assert!(hdr.contains(sym), "C runtime header must declare {sym}");
        }
    }

    #[test]
    fn c_runtime_exports_promise_abi() {
        let src = c_runtime_source();
        let hdr = c_runtime_header_source();
        for sym in PROMISE_SYMBOLS {
            assert!(src.contains(sym), "C runtime source must define {sym}");
            assert!(hdr.contains(sym), "C runtime header must declare {sym}");
        }
    }

    #[test]
    fn promise_resolve_reject_then_via_job_queue() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
        let archive = build_runtime_static_lib(&dir).expect("build static lib");
        let main_c = dir.join("main.c");
        let bin = dir.join("rt_promise");
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

            static int g_resolved;
            static int g_rejected;
            static int g_chained;
            static int g_late;
            static int g_double;

            static void *on_resolve(void *data, void *value) {
                (void)data;
                g_resolved = (int)(intptr_t)value;
                return value;
            }

            static void *on_reject(void *data, void *reason) {
                (void)data;
                g_rejected = (int)(intptr_t)reason;
                return reason;
            }

            static void *on_chain(void *data, void *value) {
                (void)data;
                return (void *)(intptr_t)((int)(intptr_t)value + 1);
            }

            static void *on_chained(void *data, void *value) {
                (void)data;
                g_chained = (int)(intptr_t)value;
                return value;
            }

            static void *on_late(void *data, void *value) {
                (void)data;
                g_late = (int)(intptr_t)value;
                return value;
            }

            static void *on_double(void *data, void *value) {
                (void)data;
                g_double += (int)(intptr_t)value;
                return value;
            }

            int main(void) {
                DraconicValue *p = draconic_rt_promise_new();
                if (!p || !draconic_rt_is_promise(p)) {
                    fprintf(stderr, "promise_new/is_promise failed\n");
                    return 1;
                }
                if (draconic_rt_promise_state(p) != 0) {
                    fprintf(stderr, "want pending\n");
                    return 2;
                }

                /* then before settle: reactions run after drain */
                (void)draconic_rt_promise_then(p, on_resolve, NULL, NULL, NULL);
                draconic_rt_promise_resolve(p, (void *)(intptr_t)42);
                if (draconic_rt_promise_state(p) != 1) {
                    fprintf(stderr, "want fulfilled\n");
                    return 3;
                }
                if ((intptr_t)draconic_rt_promise_result(p) != 42) {
                    fprintf(stderr, "result want 42\n");
                    return 4;
                }
                if (g_resolved != 0) {
                    fprintf(stderr, "reaction must not run before drain\n");
                    return 5;
                }
                if (draconic_rt_job_pending() == 0) {
                    fprintf(stderr, "settle should enqueue reaction job\n");
                    return 6;
                }
                draconic_rt_job_drain();
                if (g_resolved != 42) {
                    fprintf(stderr, "resolved want 42 got %d\n", g_resolved);
                    return 7;
                }

                /* reject path */
                DraconicValue *q = draconic_rt_promise_new();
                (void)draconic_rt_promise_then(q, NULL, NULL, on_reject, NULL);
                draconic_rt_promise_reject(q, (void *)(intptr_t)7);
                draconic_rt_job_drain();
                if (draconic_rt_promise_state(q) != 2) {
                    fprintf(stderr, "want rejected\n");
                    return 8;
                }
                if (g_rejected != 7) {
                    fprintf(stderr, "rejected want 7 got %d\n", g_rejected);
                    return 9;
                }

                /* chain: then returns derived promise; callback return settles it */
                DraconicValue *c0 = draconic_rt_promise_new();
                DraconicValue *c1 = draconic_rt_promise_then(c0, on_chain, NULL, NULL, NULL);
                DraconicValue *c2 = draconic_rt_promise_then(c1, on_chained, NULL, NULL, NULL);
                if (!draconic_rt_is_promise(c1) || !draconic_rt_is_promise(c2)) {
                    fprintf(stderr, "then must return promise\n");
                    return 10;
                }
                draconic_rt_promise_resolve(c0, (void *)(intptr_t)1);
                draconic_rt_job_drain();
                if (g_chained != 2) {
                    fprintf(stderr, "chained want 2 got %d\n", g_chained);
                    return 11;
                }

                /* then after already settled still schedules a job */
                DraconicValue *late = draconic_rt_promise_new();
                draconic_rt_promise_resolve(late, (void *)(intptr_t)99);
                (void)draconic_rt_promise_then(late, on_late, NULL, NULL, NULL);
                if (g_late != 0) {
                    fprintf(stderr, "late then must not run sync\n");
                    return 12;
                }
                draconic_rt_job_drain();
                if (g_late != 99) {
                    fprintf(stderr, "late want 99 got %d\n", g_late);
                    return 13;
                }

                /* double resolve is a no-op; reaction fires once */
                DraconicValue *d = draconic_rt_promise_new();
                (void)draconic_rt_promise_then(d, on_double, NULL, NULL, NULL);
                draconic_rt_promise_resolve(d, (void *)(intptr_t)5);
                draconic_rt_promise_resolve(d, (void *)(intptr_t)100);
                draconic_rt_job_drain();
                if (g_double != 5) {
                    fprintf(stderr, "double want 5 got %d\n", g_double);
                    return 14;
                }
                if ((intptr_t)draconic_rt_promise_result(d) != 5) {
                    fprintf(stderr, "double result want 5\n");
                    return 15;
                }

                puts("promise-ok");
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
        assert!(status.success(), "clang failed to link promise test");

        let output = Command::new(&bin).output().expect("run rt_promise");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "promise binary failed: {:?}\nstderr={stderr}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "promise-ok\n", "stdout={stdout:?}");
    }

    #[test]
    fn promise_construct_executor_then_via_job_queue() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
        let archive = build_runtime_static_lib(&dir).expect("build static lib");
        let main_c = dir.join("main.c");
        let bin = dir.join("rt_promise_construct");
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

            static int g_resolved;
            static int g_rejected;
            static int g_chained;

            static void exec_resolve(void *data,
                DraconicPromiseSettleFn resolve, void *resolve_cap,
                DraconicPromiseSettleFn reject, void *reject_cap) {
                (void)data; (void)reject; (void)reject_cap;
                resolve(resolve_cap, (void *)(intptr_t)42);
            }

            static void exec_reject(void *data,
                DraconicPromiseSettleFn resolve, void *resolve_cap,
                DraconicPromiseSettleFn reject, void *reject_cap) {
                (void)data; (void)resolve; (void)resolve_cap;
                reject(reject_cap, (void *)(intptr_t)7);
            }

            static void exec_one(void *data,
                DraconicPromiseSettleFn resolve, void *resolve_cap,
                DraconicPromiseSettleFn reject, void *reject_cap) {
                (void)data; (void)reject; (void)reject_cap;
                resolve(resolve_cap, (void *)(intptr_t)1);
            }

            static void *on_resolve(void *data, void *value) {
                (void)data;
                g_resolved = (int)(intptr_t)value;
                return value;
            }

            static void *on_reject(void *data, void *reason) {
                (void)data;
                g_rejected = (int)(intptr_t)reason;
                return reason;
            }

            static void *on_chain(void *data, void *value) {
                (void)data;
                return (void *)(intptr_t)((int)(intptr_t)value + 1);
            }

            static void *on_chained(void *data, void *value) {
                (void)data;
                g_chained = (int)(intptr_t)value;
                return value;
            }

            int main(void) {
                DraconicValue *p = draconic_rt_promise_construct(exec_resolve, NULL);
                if (!p || !draconic_rt_is_promise(p)) {
                    fprintf(stderr, "construct failed\n");
                    return 1;
                }
                if (draconic_rt_promise_state(p) != 1) {
                    fprintf(stderr, "sync resolve in executor should fulfill\n");
                    return 2;
                }
                (void)draconic_rt_promise_then(p, on_resolve, NULL, NULL, NULL);
                draconic_rt_job_drain();
                if (g_resolved != 42) {
                    fprintf(stderr, "resolved want 42 got %d\n", g_resolved);
                    return 3;
                }

                DraconicValue *q = draconic_rt_promise_construct(exec_reject, NULL);
                (void)draconic_rt_promise_then(q, NULL, NULL, on_reject, NULL);
                draconic_rt_job_drain();
                if (g_rejected != 7) {
                    fprintf(stderr, "rejected want 7 got %d\n", g_rejected);
                    return 4;
                }

                DraconicValue *c0 = draconic_rt_promise_construct(exec_one, NULL);
                DraconicValue *c1 = draconic_rt_promise_then(c0, on_chain, NULL, NULL, NULL);
                (void)draconic_rt_promise_then(c1, on_chained, NULL, NULL, NULL);
                draconic_rt_job_drain();
                if (g_chained != 2) {
                    fprintf(stderr, "chained want 2 got %d\n", g_chained);
                    return 5;
                }

                draconic_rt_print_str("construct-ok");
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
        assert!(status.success(), "clang failed to link promise construct test");

        let output = Command::new(&bin).output().expect("run rt_promise_construct");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "construct binary failed: {:?}\nstderr={stderr}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "construct-ok\n", "stdout={stdout:?}");
    }

    #[test]
    fn promise_all_array_via_job_queue() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
        let archive = build_runtime_static_lib(&dir).expect("build static lib");
        let main_c = dir.join("main.c");
        let bin = dir.join("rt_promise_all");
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

            static int g_empty_len = -1;
            static int g_all_len = -1;
            static int g_a0 = -1;
            static int g_a1 = -1;
            static int g_mixed0 = -1;
            static int g_mixed1 = -1;
            static int g_rejected = 0;

            static void *on_empty(void *data, void *value) {
                (void)data;
                g_empty_len = (int)draconic_rt_array_len((DraconicValue *)value);
                return value;
            }

            static void *on_all(void *data, void *value) {
                (void)data;
                DraconicValue *arr = (DraconicValue *)value;
                g_all_len = (int)draconic_rt_array_len(arr);
                g_a0 = (int)(intptr_t)draconic_rt_array_get(arr, 0);
                g_a1 = (int)(intptr_t)draconic_rt_array_get(arr, 1);
                return value;
            }

            static void *on_mixed(void *data, void *value) {
                (void)data;
                DraconicValue *arr = (DraconicValue *)value;
                g_mixed0 = (int)(intptr_t)draconic_rt_array_get(arr, 0);
                g_mixed1 = (int)(intptr_t)draconic_rt_array_get(arr, 1);
                return value;
            }

            static void *on_reject_ok(void *data, void *value) {
                (void)data; (void)value;
                g_rejected = -1;
                return value;
            }

            static void *on_reject_err(void *data, void *reason) {
                (void)data;
                g_rejected = (int)(intptr_t)reason;
                return reason;
            }

            int main(void) {
                DraconicValue *empty = draconic_rt_array_new(0);
                DraconicValue *p_empty = draconic_rt_promise_all(empty);
                (void)draconic_rt_promise_then(p_empty, on_empty, NULL, NULL, NULL);

                DraconicValue *a = draconic_rt_array_new(2);
                DraconicValue *p0 = draconic_rt_promise_new();
                DraconicValue *p1 = draconic_rt_promise_new();
                draconic_rt_promise_resolve(p0, (void *)(intptr_t)10);
                draconic_rt_promise_resolve(p1, (void *)(intptr_t)20);
                draconic_rt_array_set(a, 0, p0);
                draconic_rt_array_set(a, 1, p1);
                DraconicValue *p_all = draconic_rt_promise_all(a);
                (void)draconic_rt_promise_then(p_all, on_all, NULL, NULL, NULL);

                DraconicValue *m = draconic_rt_array_new(2);
                draconic_rt_array_set(m, 0, (void *)(intptr_t)1);
                DraconicValue *pm = draconic_rt_promise_new();
                draconic_rt_promise_resolve(pm, (void *)(intptr_t)2);
                draconic_rt_array_set(m, 1, pm);
                DraconicValue *p_mixed = draconic_rt_promise_all(m);
                (void)draconic_rt_promise_then(p_mixed, on_mixed, NULL, NULL, NULL);

                DraconicValue *r = draconic_rt_array_new(2);
                DraconicValue *ok = draconic_rt_promise_new();
                DraconicValue *bad = draconic_rt_promise_new();
                draconic_rt_promise_resolve(ok, (void *)(intptr_t)1);
                draconic_rt_promise_reject(bad, (void *)(intptr_t)7);
                draconic_rt_array_set(r, 0, ok);
                draconic_rt_array_set(r, 1, bad);
                DraconicValue *p_rej = draconic_rt_promise_all(r);
                (void)draconic_rt_promise_then(p_rej, on_reject_ok, NULL, on_reject_err, NULL);

                draconic_rt_job_drain();

                if (g_empty_len != 0) {
                    fprintf(stderr, "emptyLen want 0 got %d\n", g_empty_len);
                    return 1;
                }
                if (g_all_len != 2 || g_a0 != 10 || g_a1 != 20) {
                    fprintf(stderr, "all want 2,10,20 got %d,%d,%d\n", g_all_len, g_a0, g_a1);
                    return 2;
                }
                if (g_mixed0 != 1 || g_mixed1 != 2) {
                    fprintf(stderr, "mixed want 1,2 got %d,%d\n", g_mixed0, g_mixed1);
                    return 3;
                }
                if (g_rejected != 7) {
                    fprintf(stderr, "rejected want 7 got %d\n", g_rejected);
                    return 4;
                }

                draconic_rt_print_str("promise-all-ok");
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
        assert!(status.success(), "clang failed to link promise all test");

        let output = Command::new(&bin).output().expect("run rt_promise_all");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "promise all binary failed: {:?}\nstderr={stderr}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "promise-all-ok\n", "stdout={stdout:?}");
    }

    #[test]
    fn promise_race_via_job_queue() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
        let archive = build_runtime_static_lib(&dir).expect("build static lib");
        let main_c = dir.join("main.c");
        let bin = dir.join("rt_promise_race");
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

            static int g_winner = -1;
            static int g_mixed = -1;
            static int g_rejected = 0;

            static void *on_winner(void *data, void *value) {
                (void)data;
                g_winner = (int)(intptr_t)value;
                return value;
            }

            static void *on_mixed(void *data, void *value) {
                (void)data;
                g_mixed = (int)(intptr_t)value;
                return value;
            }

            static void *on_reject_ok(void *data, void *value) {
                (void)data; (void)value;
                g_rejected = -1;
                return value;
            }

            static void *on_reject_err(void *data, void *reason) {
                (void)data;
                g_rejected = (int)(intptr_t)reason;
                return reason;
            }

            int main(void) {
                DraconicValue *a = draconic_rt_array_new(2);
                DraconicValue *p0 = draconic_rt_promise_new();
                DraconicValue *p1 = draconic_rt_promise_new();
                draconic_rt_promise_resolve(p0, (void *)(intptr_t)10);
                draconic_rt_promise_resolve(p1, (void *)(intptr_t)20);
                draconic_rt_array_set(a, 0, p0);
                draconic_rt_array_set(a, 1, p1);
                DraconicValue *p_race = draconic_rt_promise_race(a);
                (void)draconic_rt_promise_then(p_race, on_winner, NULL, NULL, NULL);

                DraconicValue *m = draconic_rt_array_new(2);
                draconic_rt_array_set(m, 0, (void *)(intptr_t)1);
                DraconicValue *pm = draconic_rt_promise_new();
                draconic_rt_promise_resolve(pm, (void *)(intptr_t)2);
                draconic_rt_array_set(m, 1, pm);
                DraconicValue *p_mixed = draconic_rt_promise_race(m);
                (void)draconic_rt_promise_then(p_mixed, on_mixed, NULL, NULL, NULL);

                DraconicValue *r = draconic_rt_array_new(2);
                DraconicValue *bad = draconic_rt_promise_new();
                DraconicValue *ok = draconic_rt_promise_new();
                draconic_rt_promise_reject(bad, (void *)(intptr_t)7);
                draconic_rt_promise_resolve(ok, (void *)(intptr_t)1);
                draconic_rt_array_set(r, 0, bad);
                draconic_rt_array_set(r, 1, ok);
                DraconicValue *p_rej = draconic_rt_promise_race(r);
                (void)draconic_rt_promise_then(p_rej, on_reject_ok, NULL, on_reject_err, NULL);

                draconic_rt_job_drain();

                if (g_winner != 10) {
                    fprintf(stderr, "winner want 10 got %d\n", g_winner);
                    return 1;
                }
                if (g_mixed != 1) {
                    fprintf(stderr, "mixed want 1 got %d\n", g_mixed);
                    return 2;
                }
                if (g_rejected != 7) {
                    fprintf(stderr, "rejected want 7 got %d\n", g_rejected);
                    return 3;
                }

                draconic_rt_print_str("promise-race-ok");
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
        assert!(status.success(), "clang failed to link promise race test");

        let output = Command::new(&bin).output().expect("run rt_promise_race");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "promise race binary failed: {:?}\nstderr={stderr}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "promise-race-ok\n", "stdout={stdout:?}");
    }

    #[test]
    fn promise_all_settled_via_job_queue() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
        let archive = build_runtime_static_lib(&dir).expect("build static lib");
        let main_c = dir.join("main.c");
        let bin = dir.join("rt_promise_all_settled");
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

            static int g_empty_len = -1;
            static int g_settled_len = -1;
            static const char *g_s0 = NULL;
            static int g_v0 = -1;
            static const char *g_s1 = NULL;
            static int g_r1 = -1;
            static const char *g_m0 = NULL;
            static int g_mv0 = -1;
            static const char *g_m1 = NULL;
            static int g_mv1 = -1;

            static void *on_empty(void *data, void *value) {
                (void)data;
                g_empty_len = (int)draconic_rt_array_len((DraconicValue *)value);
                return value;
            }

            static void *on_settled(void *data, void *value) {
                (void)data;
                DraconicValue *arr = (DraconicValue *)value;
                g_settled_len = (int)draconic_rt_array_len(arr);
                DraconicValue *e0 = (DraconicValue *)draconic_rt_array_get(arr, 0);
                DraconicValue *e1 = (DraconicValue *)draconic_rt_array_get(arr, 1);
                g_s0 = (const char *)draconic_rt_object_get(e0, "status");
                g_v0 = (int)(intptr_t)draconic_rt_object_get(e0, "value");
                g_s1 = (const char *)draconic_rt_object_get(e1, "status");
                g_r1 = (int)(intptr_t)draconic_rt_object_get(e1, "reason");
                return value;
            }

            static void *on_mixed(void *data, void *value) {
                (void)data;
                DraconicValue *arr = (DraconicValue *)value;
                DraconicValue *e0 = (DraconicValue *)draconic_rt_array_get(arr, 0);
                DraconicValue *e1 = (DraconicValue *)draconic_rt_array_get(arr, 1);
                g_m0 = (const char *)draconic_rt_object_get(e0, "status");
                g_mv0 = (int)(intptr_t)draconic_rt_object_get(e0, "value");
                g_m1 = (const char *)draconic_rt_object_get(e1, "status");
                g_mv1 = (int)(intptr_t)draconic_rt_object_get(e1, "value");
                return value;
            }

            int main(void) {
                DraconicValue *empty = draconic_rt_array_new(0);
                DraconicValue *p_empty = draconic_rt_promise_all_settled(empty);
                (void)draconic_rt_promise_then(p_empty, on_empty, NULL, NULL, NULL);

                DraconicValue *a = draconic_rt_array_new(2);
                DraconicValue *p0 = draconic_rt_promise_new();
                DraconicValue *p1 = draconic_rt_promise_new();
                draconic_rt_promise_resolve(p0, (void *)(intptr_t)10);
                draconic_rt_promise_reject(p1, (void *)(intptr_t)7);
                draconic_rt_array_set(a, 0, p0);
                draconic_rt_array_set(a, 1, p1);
                DraconicValue *p_set = draconic_rt_promise_all_settled(a);
                (void)draconic_rt_promise_then(p_set, on_settled, NULL, NULL, NULL);

                DraconicValue *m = draconic_rt_array_new(2);
                draconic_rt_array_set(m, 0, (void *)(intptr_t)1);
                DraconicValue *pm = draconic_rt_promise_new();
                draconic_rt_promise_resolve(pm, (void *)(intptr_t)2);
                draconic_rt_array_set(m, 1, pm);
                DraconicValue *p_mixed = draconic_rt_promise_all_settled(m);
                (void)draconic_rt_promise_then(p_mixed, on_mixed, NULL, NULL, NULL);

                draconic_rt_job_drain();

                if (g_empty_len != 0) {
                    fprintf(stderr, "emptyLen want 0 got %d\n", g_empty_len);
                    return 1;
                }
                if (g_settled_len != 2) {
                    fprintf(stderr, "settledLen want 2 got %d\n", g_settled_len);
                    return 2;
                }
                if (!g_s0 || strcmp(g_s0, "fulfilled") != 0 || g_v0 != 10) {
                    fprintf(stderr, "s0/v0 bad: %s %d\n", g_s0 ? g_s0 : "(null)", g_v0);
                    return 3;
                }
                if (!g_s1 || strcmp(g_s1, "rejected") != 0 || g_r1 != 7) {
                    fprintf(stderr, "s1/r1 bad: %s %d\n", g_s1 ? g_s1 : "(null)", g_r1);
                    return 4;
                }
                if (!g_m0 || strcmp(g_m0, "fulfilled") != 0 || g_mv0 != 1) {
                    fprintf(stderr, "mixed0 bad: %s %d\n", g_m0 ? g_m0 : "(null)", g_mv0);
                    return 5;
                }
                if (!g_m1 || strcmp(g_m1, "fulfilled") != 0 || g_mv1 != 2) {
                    fprintf(stderr, "mixed1 bad: %s %d\n", g_m1 ? g_m1 : "(null)", g_mv1);
                    return 6;
                }

                draconic_rt_print_str("promise-all-settled-ok");
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
        assert!(status.success(), "clang failed to link promise allSettled test");

        let output = Command::new(&bin).output().expect("run rt_promise_all_settled");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "promise allSettled binary failed: {:?}\nstderr={stderr}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "promise-all-settled-ok\n", "stdout={stdout:?}");
    }

    #[test]
    fn promise_any_via_job_queue() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
        let archive = build_runtime_static_lib(&dir).expect("build static lib");
        let main_c = dir.join("main.c");
        let bin = dir.join("rt_promise_any");
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

            static int g_winner = -1;
            static int g_mixed = -1;
            static int g_all_rej = 0;
            static const char *g_err_name = NULL;
            static int g_err_len = -1;
            static int g_empty_rej = 0;
            static const char *g_empty_name = NULL;
            static int g_empty_len = -1;

            static void *on_winner(void *data, void *value) {
                (void)data;
                g_winner = (int)(intptr_t)value;
                return value;
            }

            static void *on_mixed(void *data, void *value) {
                (void)data;
                g_mixed = (int)(intptr_t)value;
                return value;
            }

            static void *on_all_rej_ok(void *data, void *value) {
                (void)data; (void)value;
                g_all_rej = -1;
                return value;
            }

            static void *on_all_rej_err(void *data, void *reason) {
                (void)data;
                g_all_rej = 1;
                DraconicValue *e = (DraconicValue *)reason;
                g_err_name = (const char *)draconic_rt_object_get(e, "name");
                DraconicValue *errs = (DraconicValue *)draconic_rt_object_get(e, "errors");
                g_err_len = (int)draconic_rt_array_len(errs);
                return reason;
            }

            static void *on_empty_ok(void *data, void *value) {
                (void)data; (void)value;
                g_empty_rej = -1;
                return value;
            }

            static void *on_empty_err(void *data, void *reason) {
                (void)data;
                g_empty_rej = 1;
                DraconicValue *e = (DraconicValue *)reason;
                g_empty_name = (const char *)draconic_rt_object_get(e, "name");
                DraconicValue *errs = (DraconicValue *)draconic_rt_object_get(e, "errors");
                g_empty_len = (int)draconic_rt_array_len(errs);
                return reason;
            }

            int main(void) {
                DraconicValue *a = draconic_rt_array_new(2);
                DraconicValue *p0 = draconic_rt_promise_new();
                DraconicValue *p1 = draconic_rt_promise_new();
                draconic_rt_promise_resolve(p0, (void *)(intptr_t)10);
                draconic_rt_promise_resolve(p1, (void *)(intptr_t)20);
                draconic_rt_array_set(a, 0, p0);
                draconic_rt_array_set(a, 1, p1);
                DraconicValue *p_win = draconic_rt_promise_any(a);
                (void)draconic_rt_promise_then(p_win, on_winner, NULL, NULL, NULL);

                DraconicValue *m = draconic_rt_array_new(2);
                draconic_rt_array_set(m, 0, (void *)(intptr_t)1);
                DraconicValue *pm = draconic_rt_promise_new();
                draconic_rt_promise_resolve(pm, (void *)(intptr_t)2);
                draconic_rt_array_set(m, 1, pm);
                DraconicValue *p_mixed = draconic_rt_promise_any(m);
                (void)draconic_rt_promise_then(p_mixed, on_mixed, NULL, NULL, NULL);

                DraconicValue *r = draconic_rt_array_new(2);
                DraconicValue *r0 = draconic_rt_promise_new();
                DraconicValue *r1 = draconic_rt_promise_new();
                draconic_rt_promise_reject(r0, (void *)(intptr_t)7);
                draconic_rt_promise_reject(r1, (void *)(intptr_t)9);
                draconic_rt_array_set(r, 0, r0);
                draconic_rt_array_set(r, 1, r1);
                DraconicValue *p_rej = draconic_rt_promise_any(r);
                (void)draconic_rt_promise_then(p_rej, on_all_rej_ok, NULL, on_all_rej_err, NULL);

                DraconicValue *empty = draconic_rt_array_new(0);
                DraconicValue *p_empty = draconic_rt_promise_any(empty);
                (void)draconic_rt_promise_then(p_empty, on_empty_ok, NULL, on_empty_err, NULL);

                draconic_rt_job_drain();

                if (g_winner != 10) {
                    fprintf(stderr, "winner want 10 got %d\n", g_winner);
                    return 1;
                }
                if (g_mixed != 1) {
                    fprintf(stderr, "mixed want 1 got %d\n", g_mixed);
                    return 2;
                }
                if (g_all_rej != 1) {
                    fprintf(stderr, "allRejected want 1 got %d\n", g_all_rej);
                    return 3;
                }
                if (!g_err_name || strcmp(g_err_name, "AggregateError") != 0 || g_err_len != 2) {
                    fprintf(stderr, "err bad: %s %d\n", g_err_name ? g_err_name : "(null)", g_err_len);
                    return 4;
                }
                if (g_empty_rej != 1) {
                    fprintf(stderr, "emptyRejected want 1 got %d\n", g_empty_rej);
                    return 5;
                }
                if (!g_empty_name || strcmp(g_empty_name, "AggregateError") != 0 || g_empty_len != 0) {
                    fprintf(stderr, "empty err bad: %s %d\n",
                        g_empty_name ? g_empty_name : "(null)", g_empty_len);
                    return 6;
                }

                draconic_rt_print_str("promise-any-ok");
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
        assert!(status.success(), "clang failed to link promise any test");

        let output = Command::new(&bin).output().expect("run rt_promise_any");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "promise any binary failed: {:?}\nstderr={stderr}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "promise-any-ok\n", "stdout={stdout:?}");
    }

    #[test]
    fn promise_finally_pass_through_via_job_queue() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
        let archive = build_runtime_static_lib(&dir).expect("build static lib");
        let main_c = dir.join("main.c");
        let bin = dir.join("rt_promise_finally");
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

            static int g_fulfilled_side;
            static int g_rejected_side;
            static int g_resolved;
            static int g_caught;

            static void *on_fulfilled_side(void *data, void *value) {
                (void)data; (void)value;
                g_fulfilled_side = 1;
                return (void *)(intptr_t)999;
            }

            static void *on_rejected_side(void *data, void *reason) {
                (void)data; (void)reason;
                g_rejected_side = 1;
                return (void *)(intptr_t)888;
            }

            static void *on_resolve(void *data, void *value) {
                (void)data;
                g_resolved = (int)(intptr_t)value;
                return value;
            }

            static void *on_catch(void *data, void *reason) {
                (void)data;
                g_caught = (int)(intptr_t)reason;
                return reason;
            }

            int main(void) {
                DraconicValue *p = draconic_rt_promise_new();
                DraconicValue *pf = draconic_rt_promise_finally(p, on_fulfilled_side, NULL);
                (void)draconic_rt_promise_then(pf, on_resolve, NULL, NULL, NULL);
                draconic_rt_promise_resolve(p, (void *)(intptr_t)42);
                draconic_rt_job_drain();
                if (g_fulfilled_side != 1) {
                    fprintf(stderr, "fulfilled side want 1 got %d\n", g_fulfilled_side);
                    return 1;
                }
                if (g_resolved != 42) {
                    fprintf(stderr, "resolved want 42 got %d (callback return must not replace)\n", g_resolved);
                    return 2;
                }

                DraconicValue *q = draconic_rt_promise_new();
                DraconicValue *qf = draconic_rt_promise_finally(q, on_rejected_side, NULL);
                (void)draconic_rt_promise_then(qf, NULL, NULL, on_catch, NULL);
                draconic_rt_promise_reject(q, (void *)(intptr_t)7);
                draconic_rt_job_drain();
                if (g_rejected_side != 1) {
                    fprintf(stderr, "rejected side want 1 got %d\n", g_rejected_side);
                    return 3;
                }
                if (g_caught != 7) {
                    fprintf(stderr, "caught want 7 got %d\n", g_caught);
                    return 4;
                }

                /* already settled */
                g_fulfilled_side = 0;
                g_resolved = 0;
                DraconicValue *r = draconic_rt_promise_new();
                draconic_rt_promise_resolve(r, (void *)(intptr_t)11);
                DraconicValue *rf = draconic_rt_promise_finally(r, on_fulfilled_side, NULL);
                (void)draconic_rt_promise_then(rf, on_resolve, NULL, NULL, NULL);
                draconic_rt_job_drain();
                if (g_fulfilled_side != 1 || g_resolved != 11) {
                    fprintf(stderr, "settled finally failed side=%d resolved=%d\n",
                        g_fulfilled_side, g_resolved);
                    return 5;
                }

                draconic_rt_print_str("finally-ok");
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
        assert!(status.success(), "clang failed to link promise finally test");

        let output = Command::new(&bin).output().expect("run rt_promise_finally");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "finally binary failed: {:?}\nstderr={stderr}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "finally-ok\n", "stdout={stdout:?}");
    }

    #[test]
    fn job_queue_fifo_drain_and_nested_enqueue() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
        let archive = build_runtime_static_lib(&dir).expect("build static lib");
        let main_c = dir.join("main.c");
        let bin = dir.join("rt_job_queue");
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

            static int g_order[8];
            static size_t g_n;

            static void push_order(int v) {
                if (g_n < 8) {
                    g_order[g_n++] = v;
                }
            }

            static void job_a(void *data) {
                (void)data;
                push_order(1);
            }

            static void job_b(void *data) {
                (void)data;
                push_order(2);
                /* Nested enqueue during drain: must run after the current job,
                   after already-queued siblings (FIFO of the whole queue). */
                draconic_rt_job_enqueue(job_a, NULL); /* will be 1 again as job 4 */
            }

            static void job_c(void *data) {
                (void)data;
                push_order(3);
            }

            static void job_print_i64(void *data) {
                int64_t v = (int64_t)(intptr_t)data;
                draconic_rt_print_i64(v);
            }

            int main(void) {
                if (draconic_rt_job_pending() != 0) {
                    fprintf(stderr, "pending want 0 got %zu\n",
                            draconic_rt_job_pending());
                    return 1;
                }

                draconic_rt_job_enqueue(job_a, NULL);
                draconic_rt_job_enqueue(job_b, NULL);
                draconic_rt_job_enqueue(job_c, NULL);

                if (draconic_rt_job_pending() != 3) {
                    fprintf(stderr, "pending want 3 got %zu\n",
                            draconic_rt_job_pending());
                    return 2;
                }

                draconic_rt_job_drain();

                if (draconic_rt_job_pending() != 0) {
                    fprintf(stderr, "after drain pending want 0 got %zu\n",
                            draconic_rt_job_pending());
                    return 3;
                }

                /* Expected order: A, B (enqueues another A), C, nested A → 1,2,3,1 */
                if (g_n != 4
                    || g_order[0] != 1
                    || g_order[1] != 2
                    || g_order[2] != 3
                    || g_order[3] != 1) {
                    fprintf(stderr, "order want 1,2,3,1 got");
                    for (size_t i = 0; i < g_n; i++) {
                        fprintf(stderr, " %d", g_order[i]);
                    }
                    fprintf(stderr, "\n");
                    return 4;
                }

                /* Second drain is a no-op; print path observes jobs ran. */
                draconic_rt_job_enqueue(job_print_i64, (void *)(intptr_t)42);
                draconic_rt_job_drain();
                draconic_rt_job_drain();

                puts("job-queue-ok");
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
        assert!(status.success(), "clang failed to link job queue test");

        let output = Command::new(&bin).output().expect("run rt_job_queue");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "job queue binary failed: {:?}\nstderr={stderr}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "42\njob-queue-ok\n", "stdout={stdout:?}");
    }

    fn which_clang() -> Option<PathBuf> {
        find_clang()
    }

    fn tempfile_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "draconic-runtime-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
