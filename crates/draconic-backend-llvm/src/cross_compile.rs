//! ROADMAP D04: available linux/darwin/windows × amd64/arm64 compile surface.

use std::path::Path;
use std::process::{Command, Stdio};

use draconic_diagnostics::{Diagnostic, Span};

/// One OS/arch pair from the D04 distribution matrix, with its LLVM triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossCompilePair {
    pub pair: &'static str,
    pub triple: &'static str,
}

const MATRIX: &[CrossCompilePair] = &[
    CrossCompilePair {
        pair: "linux/amd64",
        triple: "x86_64-unknown-linux-gnu",
    },
    CrossCompilePair {
        pair: "linux/arm64",
        triple: "aarch64-unknown-linux-gnu",
    },
    CrossCompilePair {
        pair: "darwin/amd64",
        triple: "x86_64-apple-darwin",
    },
    CrossCompilePair {
        pair: "darwin/arm64",
        triple: "aarch64-apple-darwin",
    },
    CrossCompilePair {
        pair: "windows/amd64",
        triple: "x86_64-pc-windows-msvc",
    },
    CrossCompilePair {
        pair: "windows/arm64",
        triple: "aarch64-pc-windows-msvc",
    },
];

/// Available OS/arch pairs the LLVM backend can emit for, as available.
pub fn cross_compile_matrix() -> &'static [CrossCompilePair] {
    MATRIX
}

/// Host pair when this sitting is one of the ROADMAP D04 OS/arch cells.
pub fn host_cross_compile_pair() -> Option<CrossCompilePair> {
    let pair = if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
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
        return None;
    };
    MATRIX.iter().copied().find(|p| p.pair == pair)
}

/// Compile LLVM IR to an object for at least one non-host D04 matrix triple.
/// Returns the pair that succeeded. D04.01 smoke.
pub fn compile_object_for_non_host(
    llvm_ir: &str,
    out_obj: &Path,
) -> Result<CrossCompilePair, Diagnostic> {
    let host = host_cross_compile_pair();
    let mut last_err = None;
    for pair in MATRIX.iter().copied() {
        if Some(pair) == host {
            continue;
        }
        match compile_object_for_triple(llvm_ir, pair.triple, out_obj) {
            Ok(()) => return Ok(pair),
            Err(err) => last_err = Some(err),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        Diagnostic::new(
            "no non-host D04 matrix triple available for LLVM emit",
            Span::dummy(),
        )
    }))
}

/// Compile LLVM IR to an object for `triple`. Pairs clang cannot target stay
/// unavailable (error); D04 does not require a non-host success.
pub fn compile_object_for_triple(
    llvm_ir: &str,
    triple: &str,
    out_obj: &Path,
) -> Result<(), Diagnostic> {
    let clang = crate::find_clang().ok_or_else(|| {
        Diagnostic::new(
            "clang not found (set CLANG or install a C toolchain)",
            Span::dummy(),
        )
    })?;

    if let Some(parent) = out_obj.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            Diagnostic::new(format!("create output dir failed: {e}"), Span::dummy())
        })?;
    }

    let ll_path = match out_obj.file_stem() {
        Some(stem) => {
            let mut ll = out_obj.to_path_buf();
            ll.set_file_name(format!("{}.ll", stem.to_string_lossy()));
            ll
        }
        None => out_obj.with_extension("ll"),
    };
    std::fs::write(&ll_path, llvm_ir)
        .map_err(|e| Diagnostic::new(format!("write LLVM IR failed: {e}"), Span::dummy()))?;

    let output = Command::new(&clang)
        .arg("-c")
        .arg(&ll_path)
        .arg("-o")
        .arg(out_obj)
        .arg("-Wno-override-module")
        .arg("-target")
        .arg(triple)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Diagnostic::new(format!("spawn clang -c failed: {e}"), Span::dummy()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Diagnostic::new(
            format!("clang -c -target {triple} not available: {stderr}"),
            Span::dummy(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_is_the_six_roadmap_pairs() {
        let pairs: Vec<_> = cross_compile_matrix().iter().map(|p| p.pair).collect();
        assert_eq!(
            pairs,
            [
                "linux/amd64",
                "linux/arm64",
                "darwin/amd64",
                "darwin/arm64",
                "windows/amd64",
                "windows/arm64",
            ]
        );
    }

    #[test]
    fn at_least_one_non_host_triple_emits_an_object() {
        let host = host_cross_compile_pair().expect("host is a D04 pair");
        let ir = "define i32 @main() {\n  ret i32 0\n}\n";
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "draconic-d04-01-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("smoke.o");

        let pair = compile_object_for_non_host(ir, &out)
            .expect("D04.01 requires LLVM emit for at least one non-host matrix triple");

        assert_ne!(pair.pair, host.pair, "smoke triple must not be the host");
        let bytes = std::fs::read(&out).unwrap();
        assert!(!bytes.is_empty(), "non-host object should be non-empty");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn host_pair_matches_this_os_arch() {
        let host = host_cross_compile_pair().expect("host is a D04 pair");
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            assert_eq!(host.pair, "darwin/arm64");
            assert_eq!(host.triple, "aarch64-apple-darwin");
        } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
            assert_eq!(host.pair, "darwin/amd64");
            assert_eq!(host.triple, "x86_64-apple-darwin");
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            assert_eq!(host.pair, "linux/amd64");
            assert_eq!(host.triple, "x86_64-unknown-linux-gnu");
        } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
            assert_eq!(host.pair, "linux/arm64");
            assert_eq!(host.triple, "aarch64-unknown-linux-gnu");
        } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            assert_eq!(host.pair, "windows/amd64");
            assert_eq!(host.triple, "x86_64-pc-windows-msvc");
        } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
            assert_eq!(host.pair, "windows/arm64");
            assert_eq!(host.triple, "aarch64-pc-windows-msvc");
        }
    }
}
