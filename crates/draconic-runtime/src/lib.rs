//! Native Runtime: GC + minimal std (N05) + job queue (N06.01) + Promise ABI (N06.02–N06.10)
//! + host I/O substrate (H00.02–H00.03, H01.01 process args); embed later (N07).

pub mod abi;
pub use abi::*;
pub use crypto::sha256_js_polyfill;
pub use url::{parse_url, parse_url_js_polyfill, ParsedUrl};

#[cfg(test)]
mod host_abi_tests;
#[cfg(test)]
mod host_bytes_tests;


/// L03.01: SHA-256 digest over `Uint8Array` bytes (NIST FIPS 180-2).
pub mod crypto {
    pub fn sha256_js_polyfill() -> &'static str {
        r#"function sha256(bytes) {
  if (bytes instanceof ArrayBuffer) bytes = new Uint8Array(bytes);
  if (!(bytes instanceof Uint8Array)) throw new TypeError("sha256 expects Uint8Array");
  var c = null;
  try { c = require("crypto"); } catch (e) {}
  if (c && typeof c.createHash === "function") {
    var h = c.createHash("sha256");
    h.update(Buffer.from(bytes));
    return new Uint8Array(h.digest());
  }
  throw new TypeError("sha256 unavailable");
}
if (typeof globalThis !== "undefined") globalThis.sha256 = sha256;
"#
    }
}

/// L08.01: portable URL parse — scheme / host / path / query / hash.
pub mod url {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ParsedUrl {
        pub scheme: String,
        pub host: String,
        pub path: String,
        pub query: String,
        pub hash: String,
    }

    pub fn parse_url(input: &str) -> Result<ParsedUrl, ()> {
        let bytes = input.as_bytes();
        let mut i = 0;
        if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() {
            return Err(());
        }
        while i < bytes.len() {
            let b = bytes[i];
            if b == b':' { break; }
            if !(b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.')) {
                return Err(());
            }
            i += 1;
        }
        if i == 0 || i >= bytes.len() || bytes[i] != b':' { return Err(()); }
        let scheme = input[..i].to_ascii_lowercase();
        i += 1;
        if i + 1 >= bytes.len() || bytes[i] != b'/' || bytes[i + 1] != b'/' { return Err(()); }
        i += 2;
        let auth_start = i;
        while i < bytes.len() && !matches!(bytes[i], b'/' | b'?' | b'#') { i += 1; }
        if i == auth_start { return Err(()); }
        let host = input[auth_start..i].to_string();
        let mut path = String::new();
        if i < bytes.len() && bytes[i] == b'/' {
            let path_start = i;
            i += 1;
            while i < bytes.len() && !matches!(bytes[i], b'?' | b'#') { i += 1; }
            path = input[path_start..i].to_string();
        }
        let mut query = String::new();
        if i < bytes.len() && bytes[i] == b'?' {
            i += 1;
            let q_start = i;
            while i < bytes.len() && bytes[i] != b'#' { i += 1; }
            query = input[q_start..i].to_string();
        }
        let mut hash = String::new();
        if i < bytes.len() && bytes[i] == b'#' {
            i += 1;
            hash = input[i..].to_string();
        }
        Ok(ParsedUrl { scheme, host, path, query, hash })
    }

    pub fn parse_url_js_polyfill() -> &'static str {
        r#"function parseUrl(input) {
  if (typeof input !== "string") input = String(input);
  var s = input;
  var i = 0;
  var n = s.length;
  if (n === 0) throw new TypeError("Invalid URL");
  var c0 = s.charCodeAt(0);
  if (!((c0 >= 65 && c0 <= 90) || (c0 >= 97 && c0 <= 122))) throw new TypeError("Invalid URL");
  while (i < n) {
    var b = s.charCodeAt(i);
    if (b === 58) break;
    var ok = (b >= 65 && b <= 90) || (b >= 97 && b <= 122) || (b >= 48 && b <= 57)
      || b === 43 || b === 45 || b === 46;
    if (!ok) throw new TypeError("Invalid URL");
    i++;
  }
  if (i === 0 || i >= n || s.charCodeAt(i) !== 58) throw new TypeError("Invalid URL");
  var scheme = s.slice(0, i).toLowerCase();
  i++;
  if (i + 1 >= n || s.charCodeAt(i) !== 47 || s.charCodeAt(i + 1) !== 47) throw new TypeError("Invalid URL");
  i += 2;
  var authStart = i;
  while (i < n) {
    var ch = s.charCodeAt(i);
    if (ch === 47 || ch === 63 || ch === 35) break;
    i++;
  }
  if (i === authStart) throw new TypeError("Invalid URL");
  var host = s.slice(authStart, i);
  var path = "";
  if (i < n && s.charCodeAt(i) === 47) {
    var pathStart = i;
    i++;
    while (i < n) {
      var ch2 = s.charCodeAt(i);
      if (ch2 === 63 || ch2 === 35) break;
      i++;
    }
    path = s.slice(pathStart, i);
  }
  var query = "";
  if (i < n && s.charCodeAt(i) === 63) {
    i++;
    var qStart = i;
    while (i < n && s.charCodeAt(i) !== 35) i++;
    query = s.slice(qStart, i);
  }
  var hash = "";
  if (i < n && s.charCodeAt(i) === 35) {
    i++;
    hash = s.slice(i);
  }
  return { scheme: scheme, host: host, path: path, query: query, hash: hash };
}
if (typeof globalThis !== "undefined") globalThis.parseUrl = parseUrl;
"#
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn parses_full_url() {
            let u = parse_url("https://example.com/path?q=1#frag").unwrap();
            assert_eq!(u.scheme, "https");
            assert_eq!(u.host, "example.com");
            assert_eq!(u.path, "/path");
            assert_eq!(u.query, "q=1");
            assert_eq!(u.hash, "frag");
        }
        #[test]
        fn parses_port_and_root_path() {
            let u = parse_url("http://localhost:8080/").unwrap();
            assert_eq!(u.host, "localhost:8080");
            assert_eq!(u.path, "/");
        }
        #[test]
        fn empty_path_when_absent() {
            assert_eq!(parse_url("https://example.com").unwrap().path, "");
        }
        #[test]
        fn authority_with_userinfo() {
            let u = parse_url("https://user:pass@example.com:443/a/b?x=1&y=2#top").unwrap();
            assert_eq!(u.host, "user:pass@example.com:443");
            assert_eq!(u.path, "/a/b");
            assert_eq!(u.query, "x=1&y=2");
            assert_eq!(u.hash, "top");
        }
        #[test]
        fn rejects_relative() {
            assert!(parse_url("/path").is_err());
            assert!(parse_url("").is_err());
        }
        #[test]
        fn lowercases_scheme() {
            let u = parse_url("HTTPS://Example.COM/x").unwrap();
            assert_eq!(u.scheme, "https");
            assert_eq!(u.host, "Example.COM");
        }
    }
}

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Path to the Runtime C translation unit (`draconic_rt.c`).
pub fn c_runtime_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/draconic_rt.c")
}

/// Path to the Host I/O substrate C translation unit (`draconic_rt_host.c`, H00.02).
pub fn c_host_runtime_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/draconic_rt_host.c")
}

/// Path to the Host I/O substrate header (`draconic_rt_host.h`, H00.02).
pub fn c_host_runtime_header_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/draconic_rt_host.h")
}

/// Path to the public Runtime C header (`draconic_rt.h`).
pub fn c_runtime_header_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/draconic_rt.h")
}

/// All Runtime C translation units linked into `libdraconic_rt.a`.
pub fn c_runtime_source_paths() -> Vec<PathBuf> {
    vec![c_runtime_path(), c_host_runtime_path()]
}

/// C source for the Runtime (embedded for tests and tooling).
pub fn c_runtime_source() -> &'static str {
    include_str!("draconic_rt.c")
}

/// C source for the Host I/O substrate (embedded for tests and tooling).
pub fn c_host_runtime_source() -> &'static str {
    include_str!("draconic_rt_host.c")
}

/// C header for the Host I/O substrate (embedded for tests and tooling).
pub fn c_host_runtime_header_source() -> &'static str {
    include_str!("draconic_rt_host.h")
}

/// C header for the Runtime ABI (embedded for tests and tooling).
pub fn c_runtime_header_source() -> &'static str {
    include_str!("draconic_rt.h")
}

/// Print the Runtime hello line (`hello` + newline) to stdout.
pub fn print_hello() {
    println!("hello");
}

/// Extra clang link flags required when linking the Runtime static lib.
/// H11.01: Secure Transport on macOS (`Security` + `CoreFoundation`).
pub fn apply_runtime_link_flags(cmd: &mut Command) {
    if cfg!(target_os = "macos") {
        cmd.arg("-framework").arg("Security");
        cmd.arg("-framework").arg("CoreFoundation");
    }
}

/// Build `libdraconic_rt.a` in `out_dir` (clang `-c` + `ar`).
///
/// Compiles every path from [`c_runtime_source_paths`] (core + host substrate)
/// into the archive. Callers link with the archive path (or `-L`/`-ldraconic_rt`)
/// instead of recompiling C sources each time.
pub fn build_runtime_static_lib(out_dir: &Path) -> Result<PathBuf, String> {
    let clang = find_clang().ok_or_else(|| {
        "clang not found (set CLANG or install a C toolchain)".to_string()
    })?;
    let ar = find_ar().ok_or_else(|| "ar not found (set AR or install binutils)".to_string())?;

    let sources = c_runtime_source_paths();
    for src in &sources {
        if !src.is_file() {
            return Err(format!("runtime C source missing: {}", src.display()));
        }
    }

    std::fs::create_dir_all(out_dir).map_err(|e| format!("create out_dir failed: {e}"))?;

    let header_dir = c_runtime_header_path()
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let archive = out_dir.join("libdraconic_rt.a");
    let mut objs: Vec<PathBuf> = Vec::with_capacity(sources.len());

    for src in &sources {
        let stem = src
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("draconic_rt");
        let obj = out_dir.join(format!("{stem}.o"));
        let mut compile_cmd = Command::new(&clang);
        compile_cmd
            .arg("-c")
            .arg(src)
            .arg("-o")
            .arg(&obj)
            .arg("-I")
            .arg(&header_dir)
            // H11.01 Secure Transport APIs are deprecated in favor of Network.framework.
            .arg("-Wno-deprecated-declarations");
        let compile = compile_cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("spawn clang failed: {e}"))?;
        if !compile.status.success() {
            let stderr = String::from_utf8_lossy(&compile.stderr);
            return Err(format!("clang -c {} failed: {stderr}", src.display()));
        }
        objs.push(obj);
    }

    let mut ar_cmd = Command::new(&ar);
    ar_cmd.arg("rcs").arg(&archive);
    for obj in &objs {
        ar_cmd.arg(obj);
    }
    let archive_out = ar_cmd
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

#[cfg(test)]
fn test_which_clang() -> Option<PathBuf> {
    find_clang()
}

#[cfg(test)]
fn test_tempfile_dir() -> PathBuf {
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
            GC_SET_ALLOC_THRESHOLD_SYMBOL,
            GC_ALLOC_THRESHOLD_SYMBOL,
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

    /// N09.01: allocate many heap values, root a subset (≤64), collect, assert live_count.
    #[test]
    fn gc_stress_allocate_retain_drop_many_values() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
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
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
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

    /// N09.03: mark-sweep must reclaim unrooted cycles and keep rooted cycles live.
    ///
    /// Graphs: object↔object props, array↔array elems, object.[[Prototype]] cycle,
    /// and a 3-node ring. Unroot + collect → live_count 0; root one member → whole
    /// cycle stays live and readable.
    #[test]
    fn gc_cycles_mutual_refs_collect_and_retain() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
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
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
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
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
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
        assert!(status.success(), "clang failed to link gc auto-collect test");

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

    #[test]
    fn timer_set_clear_via_job_drain() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
        let archive = build_runtime_static_lib(&dir).expect("build static lib");
        let main_c = dir.join("main_timer.c");
        let bin = dir.join("rt_timer");
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

            static int g_fired;
            static int g_cancelled;
            static int g_nested;

            static void on_nested(void *data) {
                (void)data;
                g_nested = 1;
            }

            static void on_outer(void *data) {
                (void)data;
                draconic_rt_timer_set(on_nested, NULL, 0.0);
            }

            static void on_fire(void *data) {
                (void)data;
                g_fired = 1;
            }

            static void on_cancel(void *data) {
                (void)data;
                g_cancelled = 1;
            }

            int main(void) {
                int64_t id_fire = draconic_rt_timer_set(on_fire, NULL, 0.0);
                int64_t id_cancel = draconic_rt_timer_set(on_cancel, NULL, 0.0);
                if (id_fire <= 0 || id_cancel <= 0) {
                    fprintf(stderr, "timer ids invalid\n");
                    return 1;
                }
                draconic_rt_timer_clear(id_cancel);
                draconic_rt_timer_set(on_outer, NULL, 0.0);
                draconic_rt_job_drain();
                if (g_fired != 1) {
                    fprintf(stderr, "fired want 1 got %d\n", g_fired);
                    return 2;
                }
                if (g_cancelled != 0) {
                    fprintf(stderr, "cancelled want 0 got %d\n", g_cancelled);
                    return 3;
                }
                if (g_nested != 1) {
                    fprintf(stderr, "nested want 1 got %d\n", g_nested);
                    return 4;
                }
                puts("timer-ok");
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
        assert!(status.success(), "clang failed to link timer test");

        let output = Command::new(&bin).output().expect("run rt_timer");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "timer binary failed: {:?}\nstderr={stderr}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "timer-ok\n", "stdout={stdout:?}");
    }

    #[test]
    fn timer_set_interval_clear_via_job_drain() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
        let archive = build_runtime_static_lib(&dir).expect("build static lib");
        let main_c = dir.join("main_interval.c");
        let bin = dir.join("rt_interval");
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

            static int g_ticks;
            static int g_cancelled;
            static int64_t g_id;

            static void on_tick(void *data) {
                (void)data;
                g_ticks++;
                if (g_ticks >= 3) {
                    draconic_rt_timer_clear(g_id);
                }
            }

            static void on_cancel(void *data) {
                (void)data;
                g_cancelled = 1;
            }

            int main(void) {
                g_id = draconic_rt_timer_set_interval(on_tick, NULL, 0.0);
                int64_t cid = draconic_rt_timer_set_interval(on_cancel, NULL, 0.0);
                if (g_id <= 0 || cid <= 0) {
                    fprintf(stderr, "interval ids invalid\n");
                    return 1;
                }
                draconic_rt_timer_clear(cid);
                draconic_rt_job_drain();
                if (g_ticks != 3) {
                    fprintf(stderr, "ticks want 3 got %d\n", g_ticks);
                    return 2;
                }
                if (g_cancelled != 0) {
                    fprintf(stderr, "cancelled want 0 got %d\n", g_cancelled);
                    return 3;
                }
                puts("interval-ok");
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
        assert!(status.success(), "clang failed to link interval test");

        let output = Command::new(&bin).output().expect("run rt_interval");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "interval binary failed: {:?}\nstderr={stderr}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "interval-ok\n", "stdout={stdout:?}");
    }

    /// H05.05: job_drain waits for a future timer (OS sleep), fires it, and
    /// does not return early while the timer is still pending.
    #[test]
    fn timer_drain_waits_for_future_due() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
        let archive = build_runtime_static_lib(&dir).expect("build static lib");
        let main_c = dir.join("main_timer_wait.c");
        let bin = dir.join("rt_timer_wait");
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
#if defined(_WIN32)
            #include <windows.h>
            static double wall_ms(void) {
                FILETIME ft;
                ULARGE_INTEGER u;
                const uint64_t epoch_diff_100ns = 116444736000000000ULL;
                GetSystemTimeAsFileTime(&ft);
                u.LowPart = ft.dwLowDateTime;
                u.HighPart = ft.dwHighDateTime;
                if (u.QuadPart < epoch_diff_100ns) return 0.0;
                return (double)((u.QuadPart - epoch_diff_100ns) / 10000ULL);
            }
#else
            #include <sys/time.h>
            static double wall_ms(void) {
                struct timeval tv;
                if (gettimeofday(&tv, NULL) != 0) return 0.0;
                return ((double)tv.tv_sec * 1000.0) + ((double)tv.tv_usec / 1000.0);
            }
#endif

            static int g_fired;

            static void on_fire(void *data) {
                (void)data;
                g_fired = 1;
            }

            int main(void) {
                const double delay_ms = 40.0;
                double t0 = wall_ms();
                int64_t id = draconic_rt_timer_set(on_fire, NULL, delay_ms);
                if (id <= 0) {
                    fprintf(stderr, "timer id invalid\n");
                    return 1;
                }
                draconic_rt_job_drain();
                double elapsed = wall_ms() - t0;
                if (g_fired != 1) {
                    fprintf(stderr, "fired want 1 got %d\n", g_fired);
                    return 2;
                }
                /* Must have waited ~delay (not return immediately). */
                if (elapsed < 25.0) {
                    fprintf(stderr, "elapsed too small: %g (busy-return?)\n", elapsed);
                    return 3;
                }
                /* Must not busy-spin for seconds. */
                if (elapsed > 2000.0) {
                    fprintf(stderr, "elapsed too large: %g (spin?)\n", elapsed);
                    return 4;
                }
                puts("timer-wait-ok");
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
        assert!(status.success(), "clang failed to link timer wait test");

        let output = Command::new(&bin).output().expect("run rt_timer_wait");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "timer wait binary failed: {:?}\nstderr={stderr}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "timer-wait-ok\n", "stdout={stdout:?}");
    }

    /// H16.04: public OS sleep / yield for timer tests — sleep blocks ~ms; yield returns;
    /// non-positive / NaN sleep is a no-op.
    #[test]
    fn sleep_ms_and_yield_os() {
        let clang = which_clang().expect("clang required for runtime native tests");
        let dir = tempfile_dir();
        let archive = build_runtime_static_lib(&dir).expect("build static lib");
        let main_c = dir.join("main_sleep_yield.c");
        let bin = dir.join("rt_sleep_yield");
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
#if defined(_WIN32)
            #include <windows.h>
            static double wall_ms(void) {
                FILETIME ft;
                ULARGE_INTEGER u;
                const uint64_t epoch_diff_100ns = 116444736000000000ULL;
                GetSystemTimeAsFileTime(&ft);
                u.LowPart = ft.dwLowDateTime;
                u.HighPart = ft.dwHighDateTime;
                if (u.QuadPart < epoch_diff_100ns) return 0.0;
                return (double)((u.QuadPart - epoch_diff_100ns) / 10000ULL);
            }
#else
            #include <sys/time.h>
            static double wall_ms(void) {
                struct timeval tv;
                if (gettimeofday(&tv, NULL) != 0) return 0.0;
                return ((double)tv.tv_sec * 1000.0) + ((double)tv.tv_usec / 1000.0);
            }
#endif

            int main(void) {
                double t0, elapsed;

                /* yield must return promptly */
                t0 = wall_ms();
                draconic_rt_yield();
                elapsed = wall_ms() - t0;
                if (elapsed > 500.0) {
                    fprintf(stderr, "yield too slow: %g\n", elapsed);
                    return 1;
                }

                /* sleep ~40ms blocks at least ~25ms and not seconds */
                t0 = wall_ms();
                draconic_rt_sleep_ms(40.0);
                elapsed = wall_ms() - t0;
                if (elapsed < 25.0) {
                    fprintf(stderr, "sleep elapsed too small: %g\n", elapsed);
                    return 2;
                }
                if (elapsed > 2000.0) {
                    fprintf(stderr, "sleep elapsed too large: %g\n", elapsed);
                    return 3;
                }

                /* non-positive / NaN: immediate return */
                t0 = wall_ms();
                draconic_rt_sleep_ms(0.0);
                draconic_rt_sleep_ms(-1.0);
                draconic_rt_sleep_ms(0.0 / 0.0);
                elapsed = wall_ms() - t0;
                if (elapsed > 100.0) {
                    fprintf(stderr, "noop sleep too slow: %g\n", elapsed);
                    return 4;
                }

                puts("sleep-yield-ok");
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
        assert!(status.success(), "clang failed to link sleep/yield test");

        let output = Command::new(&bin).output().expect("run rt_sleep_yield");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "sleep/yield binary failed: {:?}\nstderr={stderr}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "sleep-yield-ok\n", "stdout={stdout:?}");
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
