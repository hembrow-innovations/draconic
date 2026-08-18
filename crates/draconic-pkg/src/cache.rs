//! Module cache layout, git fetch, and OID checkout (Roadmap K03.01–K03.03).
//!
//! On-disk roots are keyed by module path + full commit OID. Layout (under a
//! cache root):
//!
//! ```text
//! {cache_root}/mod/{module_path_segments…}/{commit_oid}/   # tree at OID (K03.03)
//! {cache_root}/vcs/{module_path_segments…}/               # bare git clone (K03.02)
//! ```
//!
//! Example: module `github.com/org/lib` at OID `0123…ef` →
//! `{cache_root}/mod/github.com/org/lib/0123…ef/`.
//!
//! K03.01: path computation + validation.
//! K03.02: `git clone --bare` / `git fetch` into the VCS store (HTTPS + fixture repos).
//! K03.03: checkout pinned OID into `mod/…`; cache hit skips network.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Error while cloning or fetching a package into the module cache (K03.02).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheFetchError {
    /// Module path / layout validation failed.
    Path(CachePathError),
    /// Git URL is empty or not an allowed clone URL.
    InvalidUrl { url: String, reason: &'static str },
    /// Filesystem error (create dirs, etc.).
    Io(String),
    /// `git` subprocess failed or is unavailable.
    Git(String),
}

impl fmt::Display for CacheFetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheFetchError::Path(e) => write!(f, "{e}"),
            CacheFetchError::InvalidUrl { url, reason } => {
                write!(f, "module cache: invalid git URL `{url}`: {reason}")
            }
            CacheFetchError::Io(msg) => write!(f, "module cache: I/O error: {msg}"),
            CacheFetchError::Git(msg) => write!(f, "module cache: git error: {msg}"),
        }
    }
}

impl std::error::Error for CacheFetchError {}

impl From<CachePathError> for CacheFetchError {
    fn from(e: CachePathError) -> Self {
        CacheFetchError::Path(e)
    }
}

/// Accept clone URLs for K03.02: https/http (production), plus `file://` and
/// absolute local paths for fixture repos in tests.
fn validate_clone_url(url: &str) -> Result<(), &'static str> {
    if url.is_empty() {
        return Err("must not be empty");
    }
    if url != url.trim() {
        return Err("must not have leading or trailing whitespace");
    }
    if url.chars().any(|c| c.is_whitespace()) {
        return Err("must not contain whitespace");
    }

    if let Some(rest) = url.strip_prefix("https://") {
        if rest.is_empty() || !rest.contains('.') {
            return Err("https URL must include a host");
        }
        return Ok(());
    }
    if let Some(rest) = url.strip_prefix("http://") {
        if rest.is_empty() || !rest.contains('.') {
            return Err("http URL must include a host");
        }
        return Ok(());
    }
    if let Some(rest) = url.strip_prefix("file://") {
        if rest.is_empty() {
            return Err("file URL must include a path");
        }
        return Ok(());
    }
    // Absolute local path (fixture repos): Unix `/…` or Windows drive `C:\…` / `C:/…`.
    let path = Path::new(url);
    if path.is_absolute() {
        return Ok(());
    }

    Err("must be https://, http://, file://, or an absolute local path")
}

/// True if `dir` looks like an existing bare git repository.
fn is_bare_git_repo(dir: &Path) -> bool {
    dir.join("HEAD").is_file() && dir.join("objects").is_dir() && dir.join("refs").is_dir()
}

fn run_git(args: &[&str]) -> Result<String, CacheFetchError> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| CacheFetchError::Git(format!("failed to spawn git: {e}")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("git {:?} failed with status {}", args, output.status)
        };
        Err(CacheFetchError::Git(detail))
    }
}

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

    /// Bare git store for a module: `{root}/vcs/{path segments…}/` (K03.02).
    pub fn vcs_dir(&self, module_path: &str) -> Result<PathBuf, CachePathError> {
        Ok(self.root.join(vcs_rel_path(module_path)?))
    }

    /// Relative path from cache root to the VCS bare repo (`vcs/…`).
    pub fn vcs_rel(&self, module_path: &str) -> Result<PathBuf, CachePathError> {
        vcs_rel_path(module_path)
    }

    /// Clone `git_url` into the module VCS store, or `git fetch` if already present.
    ///
    /// Returns the absolute path to the bare repository under the cache root.
    /// HTTPS/HTTP URLs are accepted for production remotes; `file://` and absolute
    /// local paths support fixture repos in tests (no network).
    pub fn clone_or_fetch(
        &self,
        module_path: &str,
        git_url: &str,
    ) -> Result<PathBuf, CacheFetchError> {
        if let Err(reason) = validate_clone_url(git_url) {
            return Err(CacheFetchError::InvalidUrl {
                url: git_url.to_string(),
                reason,
            });
        }
        let dest = self.vcs_dir(module_path)?;
        if is_bare_git_repo(&dest) {
            // Update refs from origin (clone sets origin). Fail closed if fetch fails.
            run_git(&[
                "-C",
                dest.to_str().ok_or_else(|| {
                    CacheFetchError::Io("VCS path is not valid UTF-8".into())
                })?,
                "fetch",
                "--force",
                "origin",
                "+refs/*:refs/*",
            ])?;
            return Ok(dest);
        }
        if dest.exists() {
            return Err(CacheFetchError::Io(format!(
                "VCS path `{}` exists but is not a bare git repository",
                dest.display()
            )));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                CacheFetchError::Io(format!(
                    "create VCS parent `{}`: {e}",
                    parent.display()
                ))
            })?;
        }
        let dest_str = dest
            .to_str()
            .ok_or_else(|| CacheFetchError::Io("VCS path is not valid UTF-8".into()))?;
        run_git(&["clone", "--bare", git_url, dest_str])?;
        if !is_bare_git_repo(&dest) {
            return Err(CacheFetchError::Git(format!(
                "clone succeeded but `{}` is not a bare repository",
                dest.display()
            )));
        }
        Ok(dest)
    }

    /// True when the module already has a bare VCS store in this cache.
    pub fn has_vcs(&self, module_path: &str) -> Result<bool, CachePathError> {
        Ok(is_bare_git_repo(&self.vcs_dir(module_path)?))
    }

    /// True when `mod/{path…}/{oid}/` already holds a completed checkout (K03.03).
    pub fn has_entry(
        &self,
        module_path: &str,
        commit_oid: &str,
    ) -> Result<bool, CachePathError> {
        let dir = self.entry_dir(module_path, commit_oid)?;
        Ok(is_complete_checkout(&dir, commit_oid))
    }

    /// Materialize the package tree at `commit_oid` under `mod/{path…}/{oid}/`.
    ///
    /// Ensures the bare VCS store via [`Self::clone_or_fetch`], then extracts the
    /// tree with `git archive` (no `.git` in the package dir). When
    /// [`Self::has_entry`] is already true, returns the existing directory and
    /// performs **no** network/`git fetch` (cache hit).
    pub fn checkout(
        &self,
        module_path: &str,
        commit_oid: &str,
        git_url: &str,
    ) -> Result<PathBuf, CacheFetchError> {
        if let Err(reason) = validate_commit_oid(commit_oid) {
            return Err(CachePathError::InvalidCommitOid {
                oid: commit_oid.to_string(),
                reason,
            }
            .into());
        }
        let dest = self.entry_dir(module_path, commit_oid)?;
        if is_complete_checkout(&dest, commit_oid) {
            return Ok(dest);
        }

        let vcs = self.clone_or_fetch(module_path, git_url)?;
        let vcs_str = vcs
            .to_str()
            .ok_or_else(|| CacheFetchError::Io("VCS path is not valid UTF-8".into()))?;

        // Confirm OID exists in the bare store before writing the entry dir.
        run_git(&["-C", vcs_str, "cat-file", "-e", &format!("{commit_oid}^{{commit}}")])?;

        if dest.exists() {
            fs::remove_dir_all(&dest).map_err(|e| {
                CacheFetchError::Io(format!("remove incomplete checkout `{}`: {e}", dest.display()))
            })?;
        }
        fs::create_dir_all(&dest).map_err(|e| {
            CacheFetchError::Io(format!("create checkout dir `{}`: {e}", dest.display()))
        })?;

        let dest_str = dest
            .to_str()
            .ok_or_else(|| CacheFetchError::Io("checkout path is not valid UTF-8".into()))?;

        // Extract tree only (no .git) so K03.04 content hash is over package files.
        let archive = Command::new("git")
            .args(["-C", vcs_str, "archive", "--format=tar", commit_oid])
            .output()
            .map_err(|e| CacheFetchError::Git(format!("failed to spawn git archive: {e}")))?;
        if !archive.status.success() {
            let stderr = String::from_utf8_lossy(&archive.stderr).trim().to_string();
            let _ = fs::remove_dir_all(&dest);
            return Err(CacheFetchError::Git(if stderr.is_empty() {
                format!("git archive {commit_oid} failed with status {}", archive.status)
            } else {
                stderr
            }));
        }

        let mut tar = Command::new("tar")
            .args(["-x", "-C", dest_str])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| CacheFetchError::Io(format!("failed to spawn tar: {e}")))?;
        {
            use std::io::Write;
            let mut stdin = tar
                .stdin
                .take()
                .ok_or_else(|| CacheFetchError::Io("tar stdin missing".into()))?;
            stdin
                .write_all(&archive.stdout)
                .map_err(|e| CacheFetchError::Io(format!("write tar stdin: {e}")))?;
        }
        let tar_out = tar
            .wait_with_output()
            .map_err(|e| CacheFetchError::Io(format!("tar wait: {e}")))?;
        if !tar_out.status.success() {
            let stderr = String::from_utf8_lossy(&tar_out.stderr).trim().to_string();
            let _ = fs::remove_dir_all(&dest);
            return Err(CacheFetchError::Io(if stderr.is_empty() {
                format!("tar extract failed with status {}", tar_out.status)
            } else {
                stderr
            }));
        }

        write_checkout_marker(&dest, commit_oid).map_err(|e| {
            let _ = fs::remove_dir_all(&dest);
            CacheFetchError::Io(e)
        })?;

        Ok(dest)
    }
}

/// Marker file written after a successful K03.03 checkout (OID pin).
const CHECKOUT_MARKER: &str = ".draconic-checkout-oid";

fn checkout_marker_path(entry_dir: &Path) -> PathBuf {
    entry_dir.join(CHECKOUT_MARKER)
}

fn write_checkout_marker(entry_dir: &Path, commit_oid: &str) -> Result<(), String> {
    fs::write(checkout_marker_path(entry_dir), format!("{commit_oid}\n"))
        .map_err(|e| format!("write checkout marker: {e}"))
}

fn is_complete_checkout(entry_dir: &Path, commit_oid: &str) -> bool {
    let marker = checkout_marker_path(entry_dir);
    let Ok(contents) = fs::read_to_string(&marker) else {
        return false;
    };
    contents.trim() == commit_oid
}

/// Relative VCS path: `vcs/{module_path_segments…}`.
pub fn vcs_rel_path(module_path: &str) -> Result<PathBuf, CachePathError> {
    if let Err(reason) = validate_module_path(module_path) {
        return Err(CachePathError::InvalidPath {
            path: module_path.to_string(),
            reason,
        });
    }
    assert_safe_segments(module_path)?;

    let mut path = PathBuf::from("vcs");
    for segment in module_path.split('/') {
        path.push(segment);
    }
    Ok(path)
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

    // --- K03.02: git clone/fetch into cache ---

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "draconic-pkg-k0302-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn git_ok(args: &[&str], cwd: &Path) {
        let out = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("GIT_AUTHOR_NAME", "Draconic Test")
            .env("GIT_AUTHOR_EMAIL", "test@draconic.local")
            .env("GIT_COMMITTER_NAME", "Draconic Test")
            .env("GIT_COMMITTER_EMAIL", "test@draconic.local")
            .output()
            .expect("spawn git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Create a non-bare fixture repo with one commit; return its path.
    fn fixture_repo(root: &Path) -> PathBuf {
        let repo = root.join("upstream");
        fs::create_dir_all(&repo).unwrap();
        git_ok(&["init"], &repo);
        git_ok(&["config", "user.email", "test@draconic.local"], &repo);
        git_ok(&["config", "user.name", "Draconic Test"], &repo);
        // Default branch name stable across git versions.
        git_ok(&["checkout", "-B", "main"], &repo);
        fs::write(repo.join("hello.txt"), "hello from fixture\n").unwrap();
        git_ok(&["add", "hello.txt"], &repo);
        git_ok(&["commit", "-m", "initial"], &repo);
        repo
    }

    fn head_oid(repo: &Path) -> String {
        let out = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo)
            .output()
            .expect("rev-parse");
        assert!(out.status.success());
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[test]
    fn vcs_rel_path_layout() {
        let rel = vcs_rel_path(PATH).expect("rel");
        assert_eq!(
            rel,
            PathBuf::from("vcs").join("github.com").join("org").join("lib")
        );
    }

    #[test]
    fn clone_or_fetch_clones_fixture_into_vcs_store() {
        let root = temp_dir("clone");
        let upstream = fixture_repo(&root);
        let oid = head_oid(&upstream);
        let cache = ModuleCache::new(root.join("cache"));

        assert!(!cache.has_vcs(PATH).unwrap());
        let vcs = cache
            .clone_or_fetch(PATH, upstream.to_str().unwrap())
            .expect("clone");

        assert!(vcs.starts_with(root.join("cache")));
        assert!(is_bare_git_repo(&vcs));
        assert!(cache.has_vcs(PATH).unwrap());
        assert_eq!(vcs, cache.vcs_dir(PATH).unwrap());

        // Bare clone retains the commit object.
        let out = Command::new("git")
            .args(["cat-file", "-t", &oid])
            .current_dir(&vcs)
            .output()
            .expect("cat-file");
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "commit");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn clone_or_fetch_file_url_fixture() {
        let root = temp_dir("file-url");
        let upstream = fixture_repo(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let url = format!("file://{}", upstream.display());
        let vcs = cache.clone_or_fetch(PATH, &url).expect("clone file url");
        assert!(is_bare_git_repo(&vcs));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn clone_or_fetch_second_call_fetches() {
        let root = temp_dir("fetch");
        let upstream = fixture_repo(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();

        let vcs1 = cache.clone_or_fetch(PATH, url).expect("first clone");
        // Add a second commit on upstream.
        fs::write(upstream.join("hello.txt"), "second\n").unwrap();
        git_ok(&["add", "hello.txt"], &upstream);
        git_ok(&["commit", "-m", "second"], &upstream);
        let oid2 = head_oid(&upstream);

        let vcs2 = cache.clone_or_fetch(PATH, url).expect("fetch");
        assert_eq!(vcs1, vcs2);
        let out = Command::new("git")
            .args(["cat-file", "-t", &oid2])
            .current_dir(&vcs2)
            .output()
            .expect("cat-file oid2");
        assert!(
            out.status.success(),
            "fetch should bring new commit: {}",
            String::from_utf8_lossy(&out.stderr)
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn clone_or_fetch_rejects_invalid_module_path() {
        let cache = ModuleCache::new("/tmp/cache");
        let err = cache
            .clone_or_fetch("not-a-path", "https://example.com/x.git")
            .expect_err("bad path");
        assert!(matches!(err, CacheFetchError::Path(_)), "{err:?}");
    }

    #[test]
    fn clone_or_fetch_rejects_empty_url() {
        let cache = ModuleCache::new("/tmp/cache");
        let err = cache.clone_or_fetch(PATH, "").expect_err("empty url");
        match err {
            CacheFetchError::InvalidUrl { url, reason } => {
                assert_eq!(url, "");
                assert!(!reason.is_empty());
            }
            other => panic!("expected InvalidUrl, got {other:?}"),
        }
    }

    #[test]
    fn clone_or_fetch_rejects_ftp_url() {
        let cache = ModuleCache::new("/tmp/cache");
        let err = cache
            .clone_or_fetch(PATH, "ftp://example.com/x.git")
            .expect_err("ftp");
        assert!(matches!(err, CacheFetchError::InvalidUrl { .. }), "{err:?}");
    }

    #[test]
    fn validate_clone_url_accepts_https() {
        assert!(validate_clone_url("https://github.com/org/pkg.git").is_ok());
        assert!(validate_clone_url("http://git.example.com/org/pkg.git").is_ok());
    }

    #[test]
    fn clone_or_fetch_missing_remote_is_git_error() {
        let root = temp_dir("missing");
        let cache = ModuleCache::new(root.join("cache"));
        let missing = root.join("no-such-repo");
        let err = cache
            .clone_or_fetch(PATH, missing.to_str().unwrap())
            .expect_err("missing remote");
        assert!(matches!(err, CacheFetchError::Git(_)), "{err:?}");
        assert!(!cache.has_vcs(PATH).unwrap());
        let _ = fs::remove_dir_all(&root);
    }

    // --- K03.03: checkout pinned OID; cache hit skips network ---

    fn temp_dir_k0303(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "draconic-pkg-k0303-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn checkout_materializes_tree_at_oid() {
        let root = temp_dir_k0303("checkout");
        let upstream = fixture_repo(&root);
        let oid = head_oid(&upstream);
        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();

        assert!(!cache.has_entry(PATH, &oid).unwrap());
        let entry = cache.checkout(PATH, &oid, url).expect("checkout");

        assert_eq!(entry, cache.entry_dir(PATH, &oid).unwrap());
        assert!(cache.has_entry(PATH, &oid).unwrap());
        let hello = fs::read_to_string(entry.join("hello.txt")).expect("hello.txt");
        assert_eq!(hello, "hello from fixture\n");
        // Package tree has no .git (archive extract).
        assert!(!entry.join(".git").exists());
        // Marker pins the OID.
        assert_eq!(
            fs::read_to_string(entry.join(CHECKOUT_MARKER))
                .unwrap()
                .trim(),
            oid
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn checkout_cache_hit_skips_network() {
        let root = temp_dir_k0303("hit");
        let upstream = fixture_repo(&root);
        let oid = head_oid(&upstream);
        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();

        let entry1 = cache.checkout(PATH, &oid, url).expect("first checkout");
        // Remove upstream so a second checkout cannot clone/fetch.
        fs::remove_dir_all(&upstream).expect("remove upstream");
        let entry2 = cache
            .checkout(PATH, &oid, url)
            .expect("cache hit must not need remote");
        assert_eq!(entry1, entry2);
        assert!(cache.has_entry(PATH, &oid).unwrap());
        assert_eq!(
            fs::read_to_string(entry2.join("hello.txt")).unwrap(),
            "hello from fixture\n"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn checkout_two_oids_are_isolated() {
        let root = temp_dir_k0303("two-oid");
        let upstream = fixture_repo(&root);
        let oid1 = head_oid(&upstream);
        fs::write(upstream.join("hello.txt"), "second revision\n").unwrap();
        git_ok(&["add", "hello.txt"], &upstream);
        git_ok(&["commit", "-m", "second"], &upstream);
        let oid2 = head_oid(&upstream);
        assert_ne!(oid1, oid2);

        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();
        let e1 = cache.checkout(PATH, &oid1, url).expect("oid1");
        let e2 = cache.checkout(PATH, &oid2, url).expect("oid2");
        assert_ne!(e1, e2);
        assert_eq!(
            fs::read_to_string(e1.join("hello.txt")).unwrap(),
            "hello from fixture\n"
        );
        assert_eq!(
            fs::read_to_string(e2.join("hello.txt")).unwrap(),
            "second revision\n"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn checkout_rejects_unknown_oid() {
        let root = temp_dir_k0303("bad-oid");
        let upstream = fixture_repo(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();
        let missing = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let err = cache
            .checkout(PATH, missing, url)
            .expect_err("unknown oid");
        assert!(matches!(err, CacheFetchError::Git(_)), "{err:?}");
        assert!(!cache.has_entry(PATH, missing).unwrap());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn checkout_rejects_invalid_oid_shape() {
        let cache = ModuleCache::new("/tmp/cache");
        let err = cache
            .checkout(PATH, "not-an-oid", "https://example.com/x.git")
            .expect_err("bad oid");
        assert!(
            matches!(err, CacheFetchError::Path(CachePathError::InvalidCommitOid { .. })),
            "{err:?}"
        );
    }

    #[test]
    fn has_entry_false_when_marker_missing() {
        let root = temp_dir_k0303("no-marker");
        let cache = ModuleCache::new(root.join("cache"));
        let entry = cache.entry_dir(PATH, OID).unwrap();
        fs::create_dir_all(&entry).unwrap();
        fs::write(entry.join("hello.txt"), "orphan\n").unwrap();
        assert!(!cache.has_entry(PATH, OID).unwrap());
        let _ = fs::remove_dir_all(&root);
    }
}
