//! Ensure locked package checkouts exist in the module cache (Roadmap K07.01).
//!
//! Used by `draconic build` to auto-fetch missing locked cache entries before link.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cache::{CacheFetchError, ModuleCache};
use crate::get::{default_cache_root, LOCK_FILE};
use crate::lock::{parse_lock, LockFile};

/// Summary of ensuring locked packages are present in the cache (K07.01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnsureLockedResult {
    /// Module paths already present (cache hit; no network).
    pub kept: Vec<String>,
    /// Module paths fetched/checked out this call.
    pub fetched: Vec<String>,
}

/// Error while ensuring locked cache entries (K07.01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnsureLockedError {
    /// Existing lockfile is malformed.
    Lock(String),
    /// Clone/fetch/checkout failed for a lock pin.
    Cache { path: String, message: String },
    /// Filesystem error reading the lock.
    Io(String),
}

impl fmt::Display for EnsureLockedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnsureLockedError::Lock(msg) => write!(f, "build packages: {msg}"),
            EnsureLockedError::Cache { path, message } => {
                write!(f, "build packages: `{path}` cache: {message}")
            }
            EnsureLockedError::Io(msg) => write!(f, "build packages: {msg}"),
        }
    }
}

impl std::error::Error for EnsureLockedError {}

/// Ensure every pin in `lock` has a completed checkout under `cache` (K07.01).
///
/// Cache hits skip network. Missing entries are materialised via
/// [`ModuleCache::checkout`] using the lock's git URL + commit OID (no version float).
pub fn ensure_locked_entries(
    lock: &LockFile,
    cache: &ModuleCache,
) -> Result<EnsureLockedResult, EnsureLockedError> {
    let mut kept = Vec::new();
    let mut fetched = Vec::new();

    for (path, entry) in &lock.packages {
        let present = cache
            .has_entry(path, &entry.commit_oid)
            .map_err(|e| EnsureLockedError::Cache {
                path: path.clone(),
                message: e.to_string(),
            })?;
        if present {
            kept.push(path.clone());
            continue;
        }
        cache
            .checkout(path, &entry.commit_oid, &entry.git_url)
            .map_err(|e: CacheFetchError| EnsureLockedError::Cache {
                path: path.clone(),
                message: e.to_string(),
            })?;
        fetched.push(path.clone());
    }

    kept.sort();
    fetched.sort();
    Ok(EnsureLockedResult { kept, fetched })
}

/// Discover `draconic.lock` walking ancestors of `entry`, then ensure checkouts (K07.01).
///
/// Returns `Ok(None)` when no lockfile is found (plain programs need no package fetch).
pub fn ensure_locked_for_entry(
    entry: &Path,
) -> Result<Option<EnsureLockedResult>, EnsureLockedError> {
    let Some((workspace, lock)) = discover_lock(entry)? else {
        return Ok(None);
    };
    let cache = ModuleCache::new(default_cache_root(&workspace));
    ensure_locked_entries(&lock, &cache).map(Some)
}

/// Walk parents of `entry` for `draconic.lock`; return workspace dir + parsed lock.
fn discover_lock(entry: &Path) -> Result<Option<(PathBuf, LockFile)>, EnsureLockedError> {
    let start = if entry.is_file() {
        match entry.parent() {
            Some(p) => p,
            None => return Ok(None),
        }
    } else {
        entry
    };
    let mut dir = start;
    loop {
        let lock_path = dir.join(LOCK_FILE);
        if lock_path.is_file() {
            let src = fs::read_to_string(&lock_path).map_err(|e| {
                EnsureLockedError::Io(format!("read {}: {e}", lock_path.display()))
            })?;
            let lock = parse_lock(&src).map_err(|e| EnsureLockedError::Lock(e.to_string()))?;
            return Ok(Some((dir.to_path_buf(), lock)));
        }
        match dir.parent() {
            Some(p) => dir = p,
            None => return Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_hash_tree;
    use crate::lock::{write_lock, LockEntry, LockFile};
    use std::collections::BTreeMap;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "draconic-pkg-ensure-{}-{}-{}",
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

    fn tagged_upstream(root: &Path) -> (PathBuf, String) {
        let repo = root.join("upstream");
        fs::create_dir_all(&repo).unwrap();
        git_ok(&["init"], &repo);
        git_ok(&["config", "user.email", "test@draconic.local"], &repo);
        git_ok(&["config", "user.name", "Draconic Test"], &repo);
        git_ok(&["checkout", "-B", "main"], &repo);
        fs::write(repo.join("index.drac"), "export let x = 42;\n").unwrap();
        git_ok(&["add", "."], &repo);
        git_ok(&["commit", "-m", "v1.0.0"], &repo);
        git_ok(&["tag", "v1.0.0"], &repo);
        let oid = String::from_utf8(
            Command::new("git")
                .args(["-C", repo.to_str().unwrap(), "rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();
        (repo, oid)
    }

    #[test]
    fn ensure_locked_entries_fetches_missing() {
        let root = temp_dir();
        let (upstream, oid) = tagged_upstream(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/lib";
        // Hash of empty checkout is unknown until checkout; pin with placeholder then re-hash.
        // First checkout to get real hash for a valid lock entry.
        let checkout = cache
            .checkout(path, &oid, upstream.to_str().unwrap())
            .unwrap();
        let hash = content_hash_tree(&checkout).unwrap();
        // Wipe cache so ensure must re-fetch.
        fs::remove_dir_all(cache.root.join("mod")).unwrap();
        fs::remove_dir_all(cache.root.join("vcs")).ok();

        let entry = LockEntry::new(
            path,
            "1.0.0",
            upstream.to_str().unwrap(),
            oid.clone(),
            hash,
        )
        .unwrap();
        let mut packages = BTreeMap::new();
        packages.insert(path.to_string(), entry);
        let lock = LockFile {
            version: 1,
            packages,
        };

        assert!(!cache.has_entry(path, &oid).unwrap());
        let result = ensure_locked_entries(&lock, &cache).expect("ensure");
        assert_eq!(result.fetched, vec![path.to_string()]);
        assert!(result.kept.is_empty());
        assert!(cache.has_entry(path, &oid).unwrap());

        // Second call is cache hit.
        let again = ensure_locked_entries(&lock, &cache).expect("ensure hit");
        assert_eq!(again.kept, vec![path.to_string()]);
        assert!(again.fetched.is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn ensure_locked_for_entry_no_lock_is_none() {
        let root = temp_dir();
        let main = root.join("main.drac");
        fs::write(&main, "let x = 1;\n").unwrap();
        assert!(ensure_locked_for_entry(&main).unwrap().is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn ensure_locked_for_entry_discovers_lock_and_fetches() {
        let root = temp_dir();
        let (upstream, oid) = tagged_upstream(&root);
        let ws = root.join("app");
        fs::create_dir_all(&ws).unwrap();
        let cache_root = ws.join(".draconic/mod-cache");
        let cache = ModuleCache::new(&cache_root);
        let path = "github.com/org/lib";
        let checkout = cache
            .checkout(path, &oid, upstream.to_str().unwrap())
            .unwrap();
        let hash = content_hash_tree(&checkout).unwrap();
        fs::remove_dir_all(&cache_root).unwrap();

        let entry = LockEntry::new(
            path,
            "1.0.0",
            upstream.to_str().unwrap(),
            oid.clone(),
            hash,
        )
        .unwrap();
        let mut packages = BTreeMap::new();
        packages.insert(path.to_string(), entry);
        let lock = LockFile {
            version: 1,
            packages,
        };
        fs::write(ws.join(LOCK_FILE), write_lock(&lock)).unwrap();
        let main = ws.join("main.drac");
        fs::write(
            &main,
            "import { x } from \"github.com/org/lib\";\nexport let y = x;\n",
        )
        .unwrap();

        let result = ensure_locked_for_entry(&main)
            .expect("ensure")
            .expect("lock present");
        assert_eq!(result.fetched, vec![path.to_string()]);
        assert!(ModuleCache::new(&cache_root).has_entry(path, &oid).unwrap());

        let _ = fs::remove_dir_all(&root);
    }
}
