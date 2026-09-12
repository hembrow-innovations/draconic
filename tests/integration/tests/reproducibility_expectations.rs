//! ROADMAP D03.01: document reproducibility expectations (timestamps, paths).
//!
//! Operators need words before emit identity: same source plus a matching
//! toolchain pin does not always mean byte-identical files. JS artifacts are
//! byte-identical and carry no timestamps or source paths. LLVM IR is identical
//! only for the same source, pin, and source path (DWARF embeds that path).
//! Linked Mach-O/ELF binaries may differ in timestamps, UUIDs, and linker noise.
//! Byte-identical emit is D03.02; this row locks the public policy text.

use std::fs;
use std::path::PathBuf;

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

fn read(path: &str) -> String {
    let full = public_site_root().join(path);
    assert!(full.is_file(), "missing {} (D03.01)", full.display());
    fs::read_to_string(&full).unwrap_or_else(|e| panic!("read {}: {e}", full.display()))
}

#[test]
fn install_docs_have_a_reproducibility_section() {
    let text = read("content/install.md");
    let lower = text.to_ascii_lowercase();
    assert!(
        text.contains("## Reproducibility") || lower.contains("reproducibility"),
        "install docs should have a reproducibility section so operators can find timestamp and path policy:\n{text}"
    );
}

#[test]
fn install_docs_name_timestamp_and_path_reproducibility_expectations() {
    let text = read("content/install.md");
    let lower = text.to_ascii_lowercase();
    assert!(
        lower.contains("timestamp"),
        "install docs should name timestamp reproducibility expectations:\n{text}"
    );
    assert!(
        lower.contains("path"),
        "install docs should name path reproducibility expectations:\n{text}"
    );
    assert!(
        lower.contains("byte-identical") && (lower.contains("js") || lower.contains("javascript")),
        "install docs should say JS artifacts are byte-identical for the same source + pin:\n{text}"
    );
    assert!(
        lower.contains("llvm ir") && lower.contains("source path"),
        "install docs should say LLVM IR identity depends on the source path:\n{text}"
    );
    assert!(
        (lower.contains("mach-o") || lower.contains("elf")) && lower.contains("timestamp"),
        "install docs should say Mach-O/ELF timestamps may differ:\n{text}"
    );
    assert!(
        lower.contains("dwarf") || lower.contains("embed"),
        "install docs should say native debug records embed the source path:\n{text}"
    );
}
