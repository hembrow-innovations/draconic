//! Resolve module-path import specifiers to cached package files (Roadmap K06.01).
//!
//! Specifiers like `github.com/org/pkg` or `github.com/org/pkg/util` map to a
//! locked package's checkout root (plus optional subpath) under the module cache.
//!
//! K06.02: resolved paths must stay inside the package checkout root (no escape
//! via `..`, symlinks, or relative imports from package modules).

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cache::{CachePathError, ModuleCache};
use crate::lock::LockFile;

/// Result of resolving a module-path import to a file under a package checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedImport {
    /// Locked module path that matched (longest prefix).
    pub module_path: String,
    /// Package checkout root (`mod/{path…}/{oid}/`).
    pub package_root: PathBuf,
    /// Subpath inside the package (empty = package root entry).
    pub subpath: String,
    /// Canonical (when possible) path to the resolved source file.
    pub file: PathBuf,
}

/// Error while resolving a module-path import specifier (K06.01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportResolveError {
    /// Specifier is not a Go-like module path (relative or other form).
    NotModulePath { spec: String },
    /// No locked package path is a prefix of the specifier.
    NotInLock { spec: String },
    /// Lock pin exists but checkout is missing under the cache root.
    NotInCache {
        module_path: String,
        commit_oid: String,
        expected: PathBuf,
    },
    /// Cache path validation failed.
    CachePath(CachePathError),
    /// Subpath would escape or is otherwise invalid inside the package.
    InvalidSubpath { spec: String, reason: &'static str },
    /// Resolved path leaves the package checkout root (K06.02).
    PackageBoundary {
        spec: String,
        package_root: PathBuf,
        resolved: PathBuf,
    },
    /// Package root + subpath did not resolve to a readable module file.
    FileNotFound {
        spec: String,
        package_root: PathBuf,
        subpath: String,
    },
    /// Filesystem error while probing the package tree.
    Io(String),
}

impl fmt::Display for ImportResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportResolveError::NotModulePath { spec } => {
                write!(f, "import resolve: `{spec}` is not a module-path specifier")
            }
            ImportResolveError::NotInLock { spec } => {
                write!(
                    f,
                    "import resolve: `{spec}` does not match any package in draconic.lock"
                )
            }
            ImportResolveError::NotInCache {
                module_path,
                commit_oid,
                expected,
            } => {
                write!(
                    f,
                    "import resolve: package `{module_path}`@{commit_oid} is not in the module cache (expected `{}`)",
                    expected.display()
                )
            }
            ImportResolveError::CachePath(e) => write!(f, "import resolve: {e}"),
            ImportResolveError::InvalidSubpath { spec, reason } => {
                write!(f, "import resolve: invalid subpath in `{spec}`: {reason}")
            }
            ImportResolveError::PackageBoundary {
                spec,
                package_root,
                resolved,
            } => {
                write!(
                    f,
                    "import resolve: `{spec}` resolves to `{}` outside package root `{}` (package boundary)",
                    resolved.display(),
                    package_root.display()
                )
            }
            ImportResolveError::FileNotFound {
                spec,
                package_root,
                subpath,
            } => {
                if subpath.is_empty() {
                    write!(
                        f,
                        "import resolve: cannot find module entry for `{spec}` under `{}`",
                        package_root.display()
                    )
                } else {
                    write!(
                        f,
                        "import resolve: cannot find `{subpath}` for `{spec}` under `{}`",
                        package_root.display()
                    )
                }
            }
            ImportResolveError::Io(msg) => write!(f, "import resolve: I/O error: {msg}"),
        }
    }
}

impl std::error::Error for ImportResolveError {}

impl From<CachePathError> for ImportResolveError {
    fn from(e: CachePathError) -> Self {
        ImportResolveError::CachePath(e)
    }
}

/// True when `spec` looks like a Go-like module path import (not relative).
///
/// Relative `./` / `../` and bare filenames are not module paths. A valid path
/// has a domain-like first segment and at least two `/`-separated parts; longer
/// subpaths (past the locked package) are allowed.
pub fn looks_like_module_path_import(spec: &str) -> bool {
    if spec.is_empty() || spec.starts_with("./") || spec.starts_with("../") || spec.starts_with('/')
    {
        return false;
    }
    if spec.contains('\\') || spec.contains('\0') || spec.chars().any(|c| c.is_whitespace()) {
        return false;
    }
    if spec.starts_with('.') {
        return false;
    }
    let segments: Vec<&str> = spec.split('/').collect();
    if segments.len() < 2 {
        return false;
    }
    let host = segments[0];
    if !host.contains('.') {
        return false;
    }
    for seg in &segments {
        if seg.is_empty() || *seg == "." || *seg == ".." {
            return false;
        }
        if !seg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
        {
            return false;
        }
    }
    true
}

/// Longest locked package path that is a prefix of `spec`.
///
/// Returns `(module_path, subpath)` where `subpath` is empty when `spec` equals
/// the package path, or the remainder after `package/` otherwise.
pub fn match_locked_package(spec: &str, lock: &LockFile) -> Option<(String, String)> {
    let mut best: Option<(String, String)> = None;
    for path in lock.packages.keys() {
        if spec == path.as_str() {
            // Exact match always wins over any shorter prefix.
            return Some((path.clone(), String::new()));
        }
        if let Some(rest) = spec.strip_prefix(path.as_str()) {
            if let Some(sub) = rest.strip_prefix('/') {
                let better = match &best {
                    None => true,
                    Some((prev, _)) => path.len() > prev.len(),
                };
                if better {
                    best = Some((path.clone(), sub.to_string()));
                }
            }
        }
    }
    best
}

/// Resolve a module-path import specifier to a file in the module cache (K06.01).
///
/// Requires a lock pin and a completed checkout. Does not fetch (see K07).
pub fn resolve_module_import(
    spec: &str,
    lock: &LockFile,
    cache: &ModuleCache,
) -> Result<ResolvedImport, ImportResolveError> {
    if !looks_like_module_path_import(spec) {
        return Err(ImportResolveError::NotModulePath {
            spec: spec.to_string(),
        });
    }

    let (module_path, subpath) = match_locked_package(spec, lock).ok_or_else(|| {
        ImportResolveError::NotInLock {
            spec: spec.to_string(),
        }
    })?;

    if !subpath.is_empty()
        && subpath
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
    {
        return Err(ImportResolveError::InvalidSubpath {
            spec: spec.to_string(),
            reason: "must not contain empty, '.', or '..' segments",
        });
    }

    let entry = lock
        .packages
        .get(&module_path)
        .expect("match_locked_package key exists");
    let package_root = cache.entry_dir(&module_path, &entry.commit_oid)?;
    if !cache.has_entry(&module_path, &entry.commit_oid)? {
        return Err(ImportResolveError::NotInCache {
            module_path: module_path.clone(),
            commit_oid: entry.commit_oid.clone(),
            expected: package_root,
        });
    }

    let file = resolve_package_file(&package_root, &subpath).ok_or_else(|| {
        ImportResolveError::FileNotFound {
            spec: spec.to_string(),
            package_root: package_root.clone(),
            subpath: subpath.clone(),
        }
    })?;

    let package_root = fs::canonicalize(&package_root).unwrap_or(package_root);
    let file = fs::canonicalize(&file).unwrap_or(file);

    // K06.02: reject symlink / join results that leave the checkout root.
    if !path_is_within_root(&file, &package_root) {
        return Err(ImportResolveError::PackageBoundary {
            spec: spec.to_string(),
            package_root,
            resolved: file,
        });
    }

    Ok(ResolvedImport {
        module_path,
        package_root,
        subpath,
        file,
    })
}

/// True when `path` is `root` or a descendant of `root` (component-wise).
///
/// Both paths should be canonical when possible so symlink escapes are caught.
pub fn path_is_within_root(path: &Path, root: &Path) -> bool {
    let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    path == root || path.starts_with(&root)
}

/// Walk ancestors of `path` for a package checkout marker (`.draconic-checkout-oid`).
///
/// Returns the directory containing the marker (the package root), if any.
pub fn find_package_checkout_root(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() {
        path.parent()?.to_path_buf()
    } else {
        path.to_path_buf()
    };
    let mut dir = start;
    loop {
        if dir.join(".draconic-checkout-oid").is_file() {
            return Some(fs::canonicalize(&dir).unwrap_or(dir));
        }
        dir = dir.parent()?.to_path_buf();
    }
}

/// Ensure `resolved` stays inside `package_root` (K06.02).
pub fn ensure_within_package(
    resolved: &Path,
    package_root: &Path,
    spec: &str,
) -> Result<(), ImportResolveError> {
    if path_is_within_root(resolved, package_root) {
        return Ok(());
    }
    Err(ImportResolveError::PackageBoundary {
        spec: spec.to_string(),
        package_root: fs::canonicalize(package_root).unwrap_or_else(|_| package_root.to_path_buf()),
        resolved: fs::canonicalize(resolved).unwrap_or_else(|_| resolved.to_path_buf()),
    })
}

/// Map package root + subpath to a concrete module file.
fn resolve_package_file(package_root: &Path, subpath: &str) -> Option<PathBuf> {
    if subpath.is_empty() {
        return resolve_dir_entry(package_root).or_else(|| {
            // last path segment of the package root dir name as `name.drac`
            let name = package_root.file_name()?.to_str()?;
            // package_root ends with oid; parent is last module segment
            let pkg_name = package_root.parent()?.file_name()?.to_str()?;
            let candidate = package_root.join(format!("{pkg_name}.drac"));
            if candidate.is_file() {
                return Some(candidate);
            }
            let _ = name;
            None
        });
    }

    let base = package_root.join(subpath);
    if base.is_file() {
        return Some(base);
    }
    let with_drac = if subpath.ends_with(".drac") {
        base.clone()
    } else {
        PathBuf::from(format!("{}.drac", base.display()))
    };
    if with_drac.is_file() {
        return Some(with_drac);
    }
    if base.is_dir() {
        return resolve_dir_entry(&base);
    }
    None
}

fn resolve_dir_entry(dir: &Path) -> Option<PathBuf> {
    for name in ["index.drac", "mod.drac", "main.drac"] {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::{LockEntry, LockFile};
    use std::collections::BTreeMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    const OID: &str = "0123456789abcdef0123456789abcdef01234567";
    const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "draconic-import-resolve-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn lock_with(path: &str, oid: &str) -> LockFile {
        let entry = LockEntry::new(
            path,
            "1.0.0",
            "https://github.com/org/pkg.git",
            oid,
            HASH,
        )
        .expect("lock entry");
        let mut packages = BTreeMap::new();
        packages.insert(path.to_string(), entry);
        LockFile {
            version: 1,
            packages,
        }
    }

    fn materialize_checkout(cache: &ModuleCache, module_path: &str, oid: &str, files: &[(&str, &str)]) {
        let dir = cache.entry_dir(module_path, oid).unwrap();
        fs::create_dir_all(&dir).unwrap();
        for (rel, body) in files {
            let path = dir.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, body).unwrap();
        }
        fs::write(dir.join(".draconic-checkout-oid"), format!("{oid}\n")).unwrap();
    }

    #[test]
    fn looks_like_module_path_accepts_go_paths() {
        assert!(looks_like_module_path_import("github.com/org/pkg"));
        assert!(looks_like_module_path_import("github.com/org/pkg/util"));
        assert!(!looks_like_module_path_import("./lib.drac"));
        assert!(!looks_like_module_path_import("../x"));
        assert!(!looks_like_module_path_import("lib.drac"));
        assert!(!looks_like_module_path_import("github.com/org/../pkg"));
    }

    #[test]
    fn match_locked_package_exact_and_subpath() {
        let lock = lock_with("github.com/org/pkg", OID);
        assert_eq!(
            match_locked_package("github.com/org/pkg", &lock),
            Some(("github.com/org/pkg".into(), String::new()))
        );
        assert_eq!(
            match_locked_package("github.com/org/pkg/util", &lock),
            Some(("github.com/org/pkg".into(), "util".into()))
        );
        assert_eq!(match_locked_package("github.com/other/x", &lock), None);
    }

    #[test]
    fn match_locked_package_longest_prefix() {
        let mut lock = lock_with("github.com/org/pkg", OID);
        let nested = LockEntry::new(
            "github.com/org/pkg/util",
            "2.0.0",
            "https://github.com/org/pkg-util.git",
            "abcdef0123456789abcdef0123456789abcdef01",
            HASH,
        )
        .unwrap();
        lock.packages
            .insert("github.com/org/pkg/util".into(), nested);
        assert_eq!(
            match_locked_package("github.com/org/pkg/util/x", &lock),
            Some(("github.com/org/pkg/util".into(), "x".into()))
        );
        assert_eq!(
            match_locked_package("github.com/org/pkg/other", &lock),
            Some(("github.com/org/pkg".into(), "other".into()))
        );
    }

    #[test]
    fn resolve_package_root_index() {
        let root = temp_root("root-index");
        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/pkg";
        materialize_checkout(
            &cache,
            path,
            OID,
            &[("index.drac", "export let value = 41;\n")],
        );
        let lock = lock_with(path, OID);
        let got = resolve_module_import(path, &lock, &cache).expect("resolve");
        assert_eq!(got.module_path, path);
        assert_eq!(got.subpath, "");
        assert!(got.file.ends_with("index.drac"), "{:?}", got.file);
        let body = fs::read_to_string(&got.file).unwrap();
        assert!(body.contains("value"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_package_subpath_file() {
        let root = temp_root("subpath");
        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/pkg";
        materialize_checkout(
            &cache,
            path,
            OID,
            &[
                ("index.drac", "export let root = 1;\n"),
                ("util.drac", "export let helper = 2;\n"),
                ("nested/mod.drac", "export let deep = 3;\n"),
            ],
        );
        let lock = lock_with(path, OID);

        let util = resolve_module_import("github.com/org/pkg/util", &lock, &cache).expect("util");
        assert_eq!(util.subpath, "util");
        assert!(util.file.ends_with("util.drac"), "{:?}", util.file);

        let deep =
            resolve_module_import("github.com/org/pkg/nested", &lock, &cache).expect("nested");
        assert_eq!(deep.subpath, "nested");
        assert!(deep.file.ends_with("mod.drac"), "{:?}", deep.file);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_missing_lock_and_cache() {
        let root = temp_root("miss");
        let cache = ModuleCache::new(root.join("cache"));
        let lock = lock_with("github.com/org/pkg", OID);

        let err = resolve_module_import("github.com/other/x", &lock, &cache).unwrap_err();
        assert!(matches!(err, ImportResolveError::NotInLock { .. }), "{err}");

        let err = resolve_module_import("github.com/org/pkg", &lock, &cache).unwrap_err();
        assert!(matches!(err, ImportResolveError::NotInCache { .. }), "{err}");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_file_not_found() {
        let root = temp_root("nofile");
        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/pkg";
        materialize_checkout(&cache, path, OID, &[("other.drac", "export let x = 1;\n")]);
        let lock = lock_with(path, OID);
        let err = resolve_module_import(path, &lock, &cache).unwrap_err();
        assert!(
            matches!(err, ImportResolveError::FileNotFound { .. }),
            "{err}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// K06.02: symlink (or other resolve) that leaves the package root is rejected.
    #[test]
    fn resolve_rejects_escape_outside_package_root() {
        let root = temp_root("escape");
        let outside = root.join("secret.drac");
        fs::write(&outside, "export let leak = 1;\n").unwrap();

        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/pkg";
        materialize_checkout(
            &cache,
            path,
            OID,
            &[("index.drac", "export let value = 1;\n")],
        );
        let pkg_dir = cache.entry_dir(path, OID).unwrap();
        let link = pkg_dir.join("escape.drac");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, &link).unwrap();
        }
        #[cfg(not(unix))]
        {
            let _ = (outside, link);
            let _ = fs::remove_dir_all(&root);
            return;
        }

        let lock = lock_with(path, OID);
        let err = resolve_module_import("github.com/org/pkg/escape", &lock, &cache).unwrap_err();
        assert!(
            matches!(err, ImportResolveError::PackageBoundary { .. }),
            "{err}"
        );
        assert!(
            err.to_string().contains("package boundary")
                || err.to_string().contains("outside package"),
            "{err}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn path_within_root_rejects_sibling_prefix() {
        let root = temp_root("prefix");
        let pkg = root.join("pkg");
        let sibling = root.join("pkg-evil");
        fs::create_dir_all(&pkg).unwrap();
        fs::create_dir_all(&sibling).unwrap();
        let inside = pkg.join("a.drac");
        let outside = sibling.join("a.drac");
        fs::write(&inside, "x").unwrap();
        fs::write(&outside, "y").unwrap();
        assert!(path_is_within_root(&inside, &pkg));
        assert!(!path_is_within_root(&outside, &pkg));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn find_package_checkout_root_walks_to_marker() {
        let root = temp_root("marker");
        let nested = root.join("nested").join("deep");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join(".draconic-checkout-oid"), "abc\n").unwrap();
        let file = nested.join("m.drac");
        fs::write(&file, "export let x = 1;\n").unwrap();
        let found = find_package_checkout_root(&file).expect("marker");
        assert_eq!(
            fs::canonicalize(&found).unwrap(),
            fs::canonicalize(&root).unwrap()
        );
        assert!(find_package_checkout_root(&temp_root("nomarker")).is_none());
        let _ = fs::remove_dir_all(&root);
    }
}
