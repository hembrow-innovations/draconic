//! ROADMAP D04.02: matrix docs for available OS/arch pairs.
//!
//! D01.01 already ships a host-triple artifact from the runner that built it.
//! This row makes the distribution matrix honest: docs name the available
//! linux/darwin/windows × amd64/arm64 pairs.
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

fn public_site_root() -> PathBuf {
    let from_env = std::env::var_os("DRACONIC_WEB").map(PathBuf::from);
    let candidate = from_env.unwrap_or_else(|| repo_root().join("../draconic-web"));
    candidate.canonicalize().unwrap_or_else(|e| {
        panic!(
            "public site lives in draconic-web (set DRACONIC_WEB); {}: {e}",
            candidate.display()
        )
    })
}

fn read_site(path: &str) -> String {
    let full = public_site_root().join(path);
    assert!(full.is_file(), "missing {} (D04.02)", full.display());
    fs::read_to_string(&full).unwrap_or_else(|e| panic!("read {}: {e}", full.display()))
}

#[test]
fn install_docs_list_available_os_arch_pairs() {
    let text = read_site("content/install.md");
    for pair in AVAILABLE_PAIRS {
        assert!(
            text.contains(pair),
            "install docs should name available OS/arch pair {pair}:\n{text}"
        );
    }
}
