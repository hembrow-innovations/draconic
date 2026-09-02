//! ROADMAP D04: cross-compile matrix linux/darwin/windows × amd64/arm64 (as available).
//!
//! Combined surface for the parent row: docs and CI name the available OS/arch
//! pairs, and the LLVM backend emits for pairs this sitting can compile.
//! Child D04.01 is a dedicated non-host triple smoke; this row does not require
//! one. Child D04.02 locks docs+CI alone.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_llvm::{
    build_native_binary, compile_object_for_triple, cross_compile_matrix, emit_llvm_ir,
    host_cross_compile_pair,
};
use draconic_frontend::compile_source;

/// Spec pairs from ROADMAP D04: linux/darwin/windows × amd64/arm64.
const SPEC_PAIRS: &[(&str, &str)] = &[
    ("linux/amd64", "x86_64-unknown-linux-gnu"),
    ("linux/arm64", "aarch64-unknown-linux-gnu"),
    ("darwin/amd64", "x86_64-apple-darwin"),
    ("darwin/arm64", "aarch64-apple-darwin"),
    ("windows/amd64", "x86_64-pc-windows-msvc"),
    ("windows/arm64", "aarch64-pc-windows-msvc"),
];

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-cross-compile-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn read(path: &str) -> String {
    let full = repo_root().join(path);
    assert!(full.is_file(), "missing {} (D04)", full.display());
    fs::read_to_string(&full).unwrap_or_else(|e| panic!("read {}: {e}", full.display()))
}

fn expected_host_pair() -> &'static str {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "linux/amd64"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "linux/arm64"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "darwin/amd64"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "darwin/arm64"
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "windows/amd64"
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        "windows/arm64"
    } else {
        panic!("D04 host OS/arch is not a ROADMAP matrix pair")
    }
}

#[test]
fn matrix_covers_roadmap_os_arch_pairs() {
    let matrix = cross_compile_matrix();
    assert_eq!(
        matrix.len(),
        SPEC_PAIRS.len(),
        "D04 matrix should be the six ROADMAP OS/arch pairs"
    );
    for (pair, triple) in SPEC_PAIRS {
        let found = matrix.iter().find(|p| p.pair == *pair);
        assert!(found.is_some(), "matrix missing OS/arch pair {pair}");
        assert_eq!(
            found.unwrap().triple,
            *triple,
            "LLVM triple for {pair}"
        );
    }
}

#[test]
fn host_pair_is_in_the_matrix() {
    let host = host_cross_compile_pair().expect("D04 host should be a matrix pair");
    assert_eq!(host.pair, expected_host_pair());
    assert!(
        cross_compile_matrix().iter().any(|p| p.pair == host.pair && p.triple == host.triple),
        "host pair {} / {} must appear in the matrix",
        host.pair,
        host.triple
    );
}

/// Combined sitting: docs + CI name the matrix, and LLVM emits a host binary.
#[test]
fn docs_ci_and_host_llvm_emit_form_one_available_matrix() {
    let install = read("website/install.md");
    let workflow = read(".github/workflows/release-artifact.yml");
    for (pair, _) in SPEC_PAIRS {
        assert!(
            install.contains(pair),
            "install docs should name available OS/arch pair {pair}:\n{install}"
        );
        assert!(
            workflow.contains(pair),
            "workflow should have a CI job for available OS/arch pair {pair}:\n{workflow}"
        );
    }

    let host = host_cross_compile_pair().expect("D04 host should be a matrix pair");
    let dir = temp_dir();
    let out = dir.join("prog");
    let module = compile_source("let x: i32 = 42;").expect("compile");
    let ll = emit_llvm_ir(&module).expect("emit_llvm_ir");
    build_native_binary(&ll, Path::new(&out)).expect("host LLVM emit");
    let output = std::process::Command::new(&out).output().expect("run");
    assert!(
        output.status.success(),
        "host binary for {} failed: {}",
        host.pair,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "42\n");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn llvm_emits_objects_for_available_matrix_pairs() {
    let host = host_cross_compile_pair().expect("D04 host should be a matrix pair");
    let module = compile_source("let x: i32 = 1;").expect("compile");
    let ll = emit_llvm_ir(&module).expect("emit_llvm_ir");
    let dir = temp_dir();

    for pair in cross_compile_matrix() {
        let out = dir.join(format!("{}.o", pair.pair.replace('/', "-")));
        match compile_object_for_triple(&ll, pair.triple, &out) {
            Ok(()) => {
                let bytes = fs::read(&out).unwrap_or_else(|e| {
                    panic!("read object for {} ({}): {e}", pair.pair, pair.triple)
                });
                assert!(
                    !bytes.is_empty(),
                    "object for {} ({}) should be non-empty",
                    pair.pair,
                    pair.triple
                );
            }
            Err(err) => {
                assert_ne!(
                    pair.pair, host.pair,
                    "host pair {} must be available for LLVM emit: {err}",
                    host.pair
                );
            }
        }
    }

    let _ = fs::remove_dir_all(&dir);
}
