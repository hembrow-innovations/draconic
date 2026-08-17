//! Embed host target triple and git commit for verbose `draconic -V` (U13).

use std::path::Path;
use std::process::Command;

fn main() {
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=DRACONIC_TARGET={target}");

    let commit = git_commit().unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=DRACONIC_GIT_COMMIT={commit}");

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let git_head = Path::new(&manifest_dir).join("../../.git/HEAD");
        if git_head.exists() {
            println!("cargo:rerun-if-changed={}", git_head.display());
            if let Ok(contents) = std::fs::read_to_string(&git_head) {
                if let Some(rest) = contents.trim().strip_prefix("ref: ") {
                    let ref_path = Path::new(&manifest_dir).join("../../.git").join(rest);
                    if ref_path.exists() {
                        println!("cargo:rerun-if-changed={}", ref_path.display());
                    }
                }
            }
        }
    }
}

fn git_commit() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8(output.stdout).ok()?;
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}
