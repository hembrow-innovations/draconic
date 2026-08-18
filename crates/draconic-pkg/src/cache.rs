//! Module cache layout (Roadmap K03.01).
//!
//! On-disk roots are keyed by module path + full commit OID. Layout (under a
//! cache root):
//!
//! ```text
//! {cache_root}/mod/{module_path_segments…}/{commit_oid}/
//! ```
//!
//! Example: module `github.com/org/lib` at OID `0123…ef` →
//! `{cache_root}/mod/github.com/org/lib/0123…ef/`.
//!
//! K03.01 is path computation + validation only (no git clone — K03.02).

use std::fmt;
use std::path::{Path, PathBuf};

use crate::validate_module_path;

/// On-disk module cache root and entry path helpers (K03.01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleCache {
    /// Absolute or relative root directory for cached packages.
    pub root: PathBuf,
}

/// Error while computing or validating a module cache path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CachePathError {
    /// Module path fails Go-like schema.
    InvalidPath { path: String, reason: &'static str },
    /// Commit OID is not a full 40-char lowercase hex SHA-1.
    InvalidCommitOid { oid: String, reason: &'static str },
    /// Module path segment would be unsafe as a single filesystem component.
    UnsafePathSegment { path: String, segment: String },
}

impl fmt::Display for CachePathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CachePathError::InvalidPath { path, reason } => {
                write!(f, "module cache: invalid module path `{path}`: {reason}")
            }
            CachePathError::InvalidCommitOid { oid, reason } => {
                write!(f, "module cache: invalid commit OID `{oid}`: {reason}")
            }
            CachePathError::UnsafePathSegment { path, segment } => {
                write!(
                    f,
                    "module cache: module path `{path}` has unsafe path segment `{segment}`"
                )
            }
        }
    }
}

impl std::error::Error for CachePathError {}

/// Full git commit SHA-1: exactly 40 lowercase hex digits.
fn validate_commit_oid(oid: &str) -> Result<(), &'static str> {
    if oid.len() != 40 {
        return Err("must be exactly 40 hexadecimal characters");
    }
    if !oid.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')) {
        return Err("must be lowercase hexadecimal");
    }
    Ok(())
}

/// Reject path segments that could escape or collide on disk (`.` / `..` already
/// rejected by [`validate_module_path`]; also ban OS separators and empties).
fn assert_safe_segments(module_path: &str) -> Result<(), CachePathError> {
    for segment in module_path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(CachePathError::UnsafePathSegment {
                path: module_path.to_string(),
                segment: segment.to_string(),
            });
        }
        if segment.contains('\\') || segment.contains('\0') {
            return Err(CachePathError::UnsafePathSegment {
                path: module_path.to_string(),
                segment: segment.to_string(),
            });
        }
        // Defensive: no nested separators after split.
        if segment.contains('/') {
            return Err(CachePathError::UnsafePathSegment {
                path: module_path.to_string(),
                segment: segment.to_string(),
            });
        }
    }
    Ok(())
}

impl ModuleCache {
    /// Create a cache handle rooted at `root` (not required to exist yet).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Directory for one pinned module checkout: `{root}/mod/{path…}/{oid}/`.
    pub fn entry_dir(
        &self,
        module_path: &str,
        commit_oid: &str,
    ) -> Result<PathBuf, CachePathError> {
        Ok(self.root.join(entry_rel_path(module_path, commit_oid)?))
    }

    /// Relative path from cache root to the entry directory (`mod/…/oid`).
    pub fn entry_rel(
        &self,
        module_path: &str,
        commit_oid: &str,
    ) -> Result<PathBuf, CachePathError> {
        entry_rel_path(module_path, commit_oid)
    }
}

/// Relative entry path: `mod/{module_path_segments…}/{commit_oid}`.
pub fn entry_rel_path(module_path: &str, commit_oid: &str) -> Result<PathBuf, CachePathError> {
    if let Err(reason) = validate_module_path(module_path) {
        return Err(CachePathError::InvalidPath {
            path: module_path.to_string(),
            reason,
        });
    }
    if let Err(reason) = validate_commit_oid(commit_oid) {
        return Err(CachePathError::InvalidCommitOid {
            oid: commit_oid.to_string(),
            reason,
        });
    }
    assert_safe_segments(module_path)?;

    let mut path = PathBuf::from("mod");
    for segment in module_path.split('/') {
        path.push(segment);
    }
    path.push(commit_oid);
    Ok(path)
}

/// True if `path` is exactly under `cache_root` as a module entry directory
/// (`…/mod/<segments…>/<40-hex-oid>`), without escaping the root.
pub fn is_entry_under_root(cache_root: &Path, path: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(cache_root) else {
        return false;
    };
    let components: Vec<_> = rel
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect();
    // mod + at least host/path + oid
    if components.len() < 4 {
        return false;
    }
    if components[0] != "mod" {
        return false;
    }
    let oid = components[components.len() - 1];
    validate_commit_oid(oid).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &str = "github.com/org/lib";
    const OID: &str = "0123456789abcdef0123456789abcdef01234567";
    const OID2: &str = "fedcba9876543210fedcba9876543210fedcba98";

    #[test]
    fn entry_rel_path_layout() {
        let rel = entry_rel_path(PATH, OID).expect("rel");
        let expected = PathBuf::from("mod")
            .join("github.com")
            .join("org")
            .join("lib")
            .join(OID);
        assert_eq!(rel, expected);
    }

    #[test]
    fn module_cache_entry_dir_joins_root() {
        let cache = ModuleCache::new("/tmp/draconic-cache");
        let dir = cache.entry_dir(PATH, OID).expect("entry");
        assert_eq!(
            dir,
            PathBuf::from("/tmp/draconic-cache")
                .join("mod")
                .join("github.com")
                .join("org")
                .join("lib")
                .join(OID)
        );
    }

    #[test]
    fn different_oids_different_dirs() {
        let cache = ModuleCache::new("/cache");
        let a = cache.entry_dir(PATH, OID).unwrap();
        let b = cache.entry_dir(PATH, OID2).unwrap();
        assert_ne!(a, b);
        assert_eq!(a.parent(), b.parent());
        assert!(a.ends_with(OID));
        assert!(b.ends_with(OID2));
    }

    #[test]
    fn different_paths_different_dirs() {
        let cache = ModuleCache::new("/cache");
        let a = cache.entry_dir("github.com/a/first", OID).unwrap();
        let b = cache.entry_dir("github.com/z/last", OID).unwrap();
        assert_ne!(a, b);
        assert!(a.ends_with(OID));
        assert!(b.ends_with(OID));
    }

    #[test]
    fn nested_module_path_segments() {
        let rel = entry_rel_path("gitlab.com/group/sub/mod", OID).unwrap();
        assert_eq!(
            rel,
            PathBuf::from("mod/gitlab.com/group/sub/mod").join(OID)
        );
    }

    #[test]
    fn reject_invalid_module_path() {
        let err = entry_rel_path("not-a-path", OID).expect_err("bad path");
        match &err {
            CachePathError::InvalidPath { path, reason } => {
                assert_eq!(path, "not-a-path");
                assert!(!reason.is_empty());
            }
            other => panic!("expected InvalidPath, got {other:?}"),
        }
        assert!(err.to_string().contains("module path"));
    }

    #[test]
    fn reject_short_commit_oid() {
        let err = entry_rel_path(PATH, "abc123").expect_err("short oid");
        match err {
            CachePathError::InvalidCommitOid { oid, reason } => {
                assert_eq!(oid, "abc123");
                assert!(reason.contains("40"));
            }
            other => panic!("expected InvalidCommitOid, got {other:?}"),
        }
    }

    #[test]
    fn reject_uppercase_commit_oid() {
        let upper = "0123456789ABCDEF0123456789ABCDEF01234567";
        let err = entry_rel_path(PATH, upper).expect_err("upper oid");
        assert!(matches!(err, CachePathError::InvalidCommitOid { .. }));
    }

    #[test]
    fn reject_empty_module_path() {
        let err = entry_rel_path("", OID).expect_err("empty");
        assert!(matches!(err, CachePathError::InvalidPath { .. }));
    }

    #[test]
    fn is_entry_under_root_accepts_valid_layout() {
        let root = Path::new("/cache");
        let entry = root.join("mod/github.com/org/lib").join(OID);
        assert!(is_entry_under_root(root, &entry));
    }

    #[test]
    fn is_entry_under_root_rejects_outside() {
        let root = Path::new("/cache");
        assert!(!is_entry_under_root(root, Path::new("/other/mod/x/y").join(OID).as_path()));
    }

    #[test]
    fn is_entry_under_root_rejects_non_mod_prefix() {
        let root = Path::new("/cache");
        let entry = root.join("other/github.com/org/lib").join(OID);
        assert!(!is_entry_under_root(root, &entry));
    }

    #[test]
    fn is_entry_under_root_rejects_short_oid_tail() {
        let root = Path::new("/cache");
        let entry = root.join("mod/github.com/org/lib/notanoid");
        assert!(!is_entry_under_root(root, &entry));
    }

    #[test]
    fn same_inputs_same_path() {
        let cache = ModuleCache::new("rel-root");
        let a = cache.entry_dir(PATH, OID).unwrap();
        let b = cache.entry_dir(PATH, OID).unwrap();
        assert_eq!(a, b);
        assert_eq!(cache.entry_rel(PATH, OID).unwrap(), entry_rel_path(PATH, OID).unwrap());
    }
}
