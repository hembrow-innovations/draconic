//! D05.01: strip symbols from a native build artifact.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use draconic_diagnostics::{Diagnostic, Span};

/// Drop the symbol table (and companion `.dSYM` on macOS) from `path`.
pub(crate) fn strip_native_binary(path: &Path) -> Result<(), Diagnostic> {
    let strip = find_strip().ok_or_else(|| {
        Diagnostic::new(
            "strip not found (install binutils or Xcode command line tools)",
            Span::dummy(),
        )
    })?;
    let output = Command::new(&strip)
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Diagnostic::new(format!("spawn strip failed: {e}"), Span::dummy()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Diagnostic::new(
            format!("strip failed: {stderr}"),
            Span::dummy(),
        ));
    }
    let dsym = dsym_companion(path);
    if dsym.is_dir() {
        let _ = fs::remove_dir_all(&dsym);
    }
    Ok(())
}

fn dsym_companion(bin: &Path) -> PathBuf {
    let mut p = bin.as_os_str().to_os_string();
    p.push(".dSYM");
    PathBuf::from(p)
}

fn find_strip() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("STRIP") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    for candidate in ["strip", "/usr/bin/strip"] {
        let path = PathBuf::from(candidate);
        if path.is_absolute() {
            if path.is_file() {
                return Some(path);
            }
            continue;
        }
        let ok = Command::new(candidate)
            .arg("-h")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok();
        if ok {
            return Some(path);
        }
    }
    None
}
