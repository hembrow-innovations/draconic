//! Clang archive build, Runtime C paths, and link flags.

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
    cmd.arg("-pthread");
}

/// Build `libdraconic_rt.a` in `out_dir` (clang `-c` + `ar`).
///
/// Compiles every path from [`c_runtime_source_paths`] (core + host substrate)
/// into the archive. Callers link with the archive path (or `-L`/`-ldraconic_rt`)
/// instead of recompiling C sources each time.
pub fn build_runtime_static_lib(out_dir: &Path) -> Result<PathBuf, String> {
    build_runtime_static_lib_with_lto(out_dir, false)
}

/// Prefer a tree file; if the compile-time `CARGO_MANIFEST_DIR` path is gone
/// (deleted pi-worktree), write `embedded` into `out_dir` so clang can still run.
fn resolve_runtime_c_file(
    disk: PathBuf,
    embedded: &str,
    out_dir: &Path,
    name: &str,
) -> Result<PathBuf, String> {
    if disk.is_file() {
        return Ok(disk);
    }
    std::fs::create_dir_all(out_dir).map_err(|e| format!("create out_dir failed: {e}"))?;
    let dest = out_dir.join(name);
    std::fs::write(&dest, embedded).map_err(|e| format!("write embedded {name} failed: {e}"))?;
    Ok(dest)
}

fn embedded_c_source_for_name(name: &str) -> Option<&'static str> {
    match name {
        "draconic_rt.c" => Some(c_runtime_source()),
        "draconic_rt_host.c" => Some(c_host_runtime_source()),
        _ => None,
    }
}

fn resolve_runtime_header_dir(disk_header: PathBuf, out_dir: &Path) -> Result<PathBuf, String> {
    if disk_header.is_file() {
        return Ok(disk_header
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf());
    }
    std::fs::create_dir_all(out_dir).map_err(|e| format!("create out_dir failed: {e}"))?;
    std::fs::write(out_dir.join("draconic_rt.h"), c_runtime_header_source())
        .map_err(|e| format!("write embedded draconic_rt.h failed: {e}"))?;
    std::fs::write(
        out_dir.join("draconic_rt_host.h"),
        c_host_runtime_header_source(),
    )
    .map_err(|e| format!("write embedded draconic_rt_host.h failed: {e}"))?;
    Ok(out_dir.to_path_buf())
}

/// D05.02: same as [`build_runtime_static_lib`], compiling with `-flto -Os` when `lto`.
pub fn build_runtime_static_lib_with_lto(out_dir: &Path, lto: bool) -> Result<PathBuf, String> {
    build_runtime_static_lib_with_inputs(
        out_dir,
        lto,
        &c_runtime_source_paths(),
        c_runtime_header_path(),
    )
}

fn build_runtime_static_lib_with_inputs(
    out_dir: &Path,
    lto: bool,
    disk_sources: &[PathBuf],
    disk_header: PathBuf,
) -> Result<PathBuf, String> {
    let clang = find_clang()
        .ok_or_else(|| "clang not found (set CLANG or install a C toolchain)".to_string())?;
    let ar = find_ar().ok_or_else(|| "ar not found (set AR or install binutils)".to_string())?;

    std::fs::create_dir_all(out_dir).map_err(|e| format!("create out_dir failed: {e}"))?;

    let mut sources: Vec<PathBuf> = Vec::with_capacity(disk_sources.len());
    for disk in disk_sources {
        let name = disk
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("runtime C source name missing: {}", disk.display()))?;
        let embedded = embedded_c_source_for_name(name)
            .ok_or_else(|| format!("runtime C source missing: {}", disk.display()))?;
        sources.push(resolve_runtime_c_file(
            disk.clone(),
            embedded,
            out_dir,
            name,
        )?);
    }

    let header_dir = resolve_runtime_header_dir(disk_header, out_dir)?;
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
        if lto {
            compile_cmd.arg("-flto").arg("-Os");
        }
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
        return Err(format!(
            "static lib missing after ar: {}",
            archive.display()
        ));
    }
    Ok(archive)
}

#[cfg(test)]
pub(crate) fn test_which_clang() -> Option<PathBuf> {
    find_clang()
}

#[cfg(test)]
pub(crate) fn test_tempfile_dir() -> PathBuf {
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
    for candidate in [
        "ar",
        "/usr/bin/ar",
        "llvm-ar",
        "/opt/homebrew/opt/llvm/bin/llvm-ar",
    ] {
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
    use crate::{
        ALLOC_OBJECT_SYMBOL, ALLOC_STRING_SYMBOL, GC_ALLOC_THRESHOLD_SYMBOL, GC_COLLECT_SYMBOL,
        GC_INIT_SYMBOL, GC_LIVE_COUNT_SYMBOL, GC_ROOT_POP_SYMBOL, GC_ROOT_PUSH_SYMBOL,
        GC_SET_ALLOC_THRESHOLD_SYMBOL, GC_SHUTDOWN_SYMBOL, HELLO_SYMBOL,
        MINIMAL_STD_AND_GC_SYMBOLS, PRINT_BOOL_SYMBOL, PRINT_F64_SYMBOL, PRINT_I64_SYMBOL,
        PRINT_U64_SYMBOL,
    };
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
        assert!(
            c_runtime_path().is_file(),
            "draconic_rt.c must exist on disk"
        );
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
        let dir = test_tempfile_dir();
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
    fn resolve_runtime_c_file_writes_embedded_when_disk_missing() {
        let dir = test_tempfile_dir();
        let missing = dir.join("gone").join("draconic_rt.c");
        assert!(!missing.is_file());
        let got = resolve_runtime_c_file(missing, "/* embedded rt */\n", &dir, "draconic_rt.c")
            .expect("materialize");
        assert_eq!(got, dir.join("draconic_rt.c"));
        assert_eq!(
            std::fs::read_to_string(&got).unwrap(),
            "/* embedded rt */\n"
        );
    }

    #[test]
    fn resolve_runtime_c_file_keeps_disk_path_when_present() {
        let dir = test_tempfile_dir();
        let disk = dir.join("tree").join("draconic_rt.c");
        std::fs::create_dir_all(disk.parent().unwrap()).unwrap();
        std::fs::write(&disk, "/* disk rt */\n").unwrap();
        let got =
            resolve_runtime_c_file(disk.clone(), "/* embedded rt */\n", &dir, "draconic_rt.c")
                .expect("disk path");
        assert_eq!(got, disk);
        assert_eq!(std::fs::read_to_string(&got).unwrap(), "/* disk rt */\n");
    }

    #[test]
    fn build_runtime_static_lib_from_missing_tree_paths() {
        let dir = test_tempfile_dir();
        let archive = build_runtime_static_lib_with_inputs(
            &dir,
            false,
            &[
                PathBuf::from("/nonexistent-draconic-tree/draconic_rt.c"),
                PathBuf::from("/nonexistent-draconic-tree/draconic_rt_host.c"),
            ],
            PathBuf::from("/nonexistent-draconic-tree/draconic_rt.h"),
        )
        .expect("build from embedded when CARGO_MANIFEST_DIR is gone");
        assert!(
            archive.is_file(),
            "expected archive at {}",
            archive.display()
        );
    }

    #[test]
    fn links_static_lib_gc_and_minimal_std() {
        let clang = test_which_clang().expect("clang required for runtime native tests");
        let dir = test_tempfile_dir();
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
        let clang = test_which_clang().expect("clang required for runtime native tests");
        let dir = test_tempfile_dir();
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
        assert!(
            output.status.success(),
            "binary failed: {:?}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "hello\n", "stdout={stdout:?}");
    }

    #[test]
    fn print_hello_smoke() {
        print_hello();
    }
}
