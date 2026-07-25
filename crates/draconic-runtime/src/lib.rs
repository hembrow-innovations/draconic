//! Native Runtime: GC, async job queue, std hooks (ROADMAP B08+).

use std::path::{Path, PathBuf};

/// Path to the minimal Runtime C translation unit (`draconic_rt.c`).
pub fn c_runtime_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/draconic_rt.c")
}

/// C source for the Runtime hello entry (embedded for tests and tooling).
pub fn c_runtime_source() -> &'static str {
    include_str!("draconic_rt.c")
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

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

    fn which_clang() -> Option<PathBuf> {
        if let Ok(p) = std::env::var("CLANG") {
            let p = PathBuf::from(p);
            if p.is_file() {
                return Some(p);
            }
        }
        for candidate in ["clang", "/usr/bin/clang", "/opt/homebrew/opt/llvm@22/bin/clang"] {
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
