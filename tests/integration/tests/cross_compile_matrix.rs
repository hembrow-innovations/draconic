//! ROADMAP D04.02: matrix docs + CI jobs for available OS/arch pairs.
//!
//! D01.01 already ships a host-triple artifact from the runner that built it.
//! This row makes the distribution matrix honest: docs name the available
//! linux/darwin/windows × amd64/arm64 pairs, and CI jobs exist for those pairs.
//! LLVM non-host emit is D04.01; unavailable pairs stay out of this sitting.

use std::fs;
use std::path::PathBuf;

/// Spec pairs from ROADMAP D04: linux/darwin/windows × amd64/arm64 as available
/// on GitHub-hosted runners.
const AVAILABLE_PAIRS: &[&str] = &[
    "linux/amd64",
    "linux/arm64",
    "darwin/amd64",
    "darwin/arm64",
    "windows/amd64",
    "windows/arm64",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn read(path: &str) -> String {
    let full = repo_root().join(path);
    assert!(full.is_file(), "missing {} (D04.02)", full.display());
    fs::read_to_string(&full).unwrap_or_else(|e| panic!("read {}: {e}", full.display()))
}

#[test]
fn install_docs_list_available_os_arch_pairs() {
    let text = read("website/install.md");
    for pair in AVAILABLE_PAIRS {
        assert!(
            text.contains(pair),
            "install docs should name available OS/arch pair {pair}:\n{text}"
        );
    }
}

#[test]
fn ci_jobs_cover_available_os_arch_pairs() {
    let text = read(".github/workflows/release-artifact.yml");
    assert!(
        text.contains("strategy:") && text.contains("matrix:"),
        "workflow should use a job matrix for available OS/arch pairs:\n{text}"
    );
    for pair in AVAILABLE_PAIRS {
        assert!(
            text.contains(pair),
            "workflow should have a CI job for available OS/arch pair {pair}:\n{text}"
        );
    }
    assert!(
        text.contains("release-artifact.sh") || text.contains("scripts/release-artifact"),
        "matrix jobs should still stage the host-triple artifact:\n{text}"
    );
    assert!(
        text.contains("upload-artifact"),
        "matrix jobs should upload a per-pair artifact:\n{text}"
    );
    assert!(
        text.contains("draconic-cli") || text.contains("cargo build"),
        "matrix jobs should build the draconic CLI:\n{text}"
    );
    assert!(
        text.contains("matrix."),
        "artifact names should be unique per matrix pair, not a single host upload:\n{text}"
    );
}
