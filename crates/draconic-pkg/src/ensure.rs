//! Ensure locked package checkouts exist in the module cache (Roadmap K07.01–K07.03 / K08).
//!
//! Used by `draconic build` to auto-fetch missing locked cache entries before link.
//! With `offline`, only the cache is consulted; a miss is a hard error with a fixit.
//! When a lock is present, pins are authoritative: checkout uses lock `commit_oid` only
//! (never re-resolve tags / float to a newer matching version — K07.03).
//! After each pin is present, verifies checkout OID marker + path against lock
//! `commit_oid` and recomputes the package tree SHA-256 against lock `content_hash`
//! (K08.01 / K08.02). Mismatched OID or hash → hard-fail; no silent wrong tree.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cache::{CacheFetchError, ModuleCache};
use crate::get::{default_cache_root, LOCK_FILE};
use crate::hash::{verify_package_integrity, PackageIntegrityError};
use crate::lock::{parse_lock, LockFile};

/// Summary of ensuring locked packages are present in the cache (K07.01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnsureLockedResult {
    /// Module paths already present (cache hit; no network).
    pub kept: Vec<String>,
    /// Module paths fetched/checked out this call.
    pub fetched: Vec<String>,
}

/// Error while ensuring locked cache entries (K07.01 / K07.02 / K08).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnsureLockedError {
    /// Existing lockfile is malformed.
    Lock(String),
    /// Clone/fetch/checkout failed for a lock pin.
    Cache { path: String, message: String },
    /// Offline build: locked pin missing from cache (K07.02).
    OfflineMiss { path: String },
    /// Checkout marker/path OID does not match lock `commit_oid` (K08.02).
    OidMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    /// Recomputed tree hash does not match lock `content_hash` (K08.01).
    ContentHashMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    /// Failed to hash package tree or read checkout marker while verifying (K08).
    ContentHash { path: String, message: String },
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
            EnsureLockedError::OfflineMiss { path } => write!(
                f,
                "build packages: `{path}` not in cache (offline); run `draconic get` or build without --offline"
            ),
            EnsureLockedError::OidMismatch {
                path,
                expected,
                actual,
            } => write!(
                f,
                "build packages: `{path}` OID mismatch (lock={expected}, marker={actual}); refuse wrong tree"
            ),
            EnsureLockedError::ContentHashMismatch {
                path,
                expected,
                actual,
            } => write!(
                f,
                "build packages: `{path}` content hash mismatch (lock={expected}, actual={actual}); refuse tampered or wrong tree"
            ),
            EnsureLockedError::ContentHash { path, message } => {
                write!(f, "build packages: `{path}` content hash: {message}")
            }
            EnsureLockedError::Io(msg) => write!(f, "build packages: {msg}"),
        }
    }
}

impl std::error::Error for EnsureLockedError {}

/// Ensure every pin in `lock` has a completed checkout under `cache` (K07.01–K07.03 / K08).
///
/// Cache hits skip network. When `offline` is false, missing entries are materialised via
/// [`ModuleCache::checkout`] using the lock's git URL + commit OID only (K07.03: never
/// re-resolve tags or float to a newer matching version while the lock is present).
/// When `offline` is true, a missing pin is [`EnsureLockedError::OfflineMiss`] (no network).
///
/// After a pin is present (hit or fetch), verifies checkout marker/path OID against lock
/// `commit_oid` and recomputes the package tree SHA-256 against lock `content_hash`
/// (K08.01 / K08.02). A directory whose marker OID disagrees with the lock pin is refused
/// (not treated as a quiet miss that could mask a wrong tree).
pub fn ensure_locked_entries(
    lock: &LockFile,
    cache: &ModuleCache,
    offline: bool,
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
            verify_locked_entry_integrity(cache, path, entry)?;
            kept.push(path.clone());
            continue;
        }
        // K08.02: dir exists with a *different* marker OID → refuse, do not silently
        // treat as miss / overwrite without diagnosing the wrong pin.
        if let Some(actual) = conflicting_checkout_oid(cache, path, entry)? {
            return Err(EnsureLockedError::OidMismatch {
                path: path.clone(),
                expected: entry.commit_oid.clone(),
                actual,
            });
        }
        if offline {
            return Err(EnsureLockedError::OfflineMiss {
                path: path.clone(),
            });
        }
        cache
            .checkout(path, &entry.commit_oid, &entry.git_url)
            .map_err(|e: CacheFetchError| EnsureLockedError::Cache {
                path: path.clone(),
                message: e.to_string(),
            })?;
        verify_locked_entry_integrity(cache, path, entry)?;
        fetched.push(path.clone());
    }

    kept.sort();
    fetched.sort();
    Ok(EnsureLockedResult { kept, fetched })
}

/// If the entry dir exists with a checkout marker that is not the lock OID, return it.
fn conflicting_checkout_oid(
    cache: &ModuleCache,
    path: &str,
    entry: &crate::lock::LockEntry,
) -> Result<Option<String>, EnsureLockedError> {
    let dir = cache
        .entry_dir(path, &entry.commit_oid)
        .map_err(|e| EnsureLockedError::Cache {
            path: path.to_string(),
            message: e.to_string(),
        })?;
    if !dir.is_dir() {
        return Ok(None);
    }
    match crate::hash::read_checkout_oid(&dir) {
        Ok(Some(actual)) if actual != entry.commit_oid => Ok(Some(actual)),
        Ok(_) => Ok(None),
        Err(e) => Err(EnsureLockedError::ContentHash {
            path: path.to_string(),
            message: e.to_string(),
        }),
    }
}

/// Verify checkout OID pin + tree hash against lock (K08.01 / K08.02).
fn verify_locked_entry_integrity(
    cache: &ModuleCache,
    path: &str,
    entry: &crate::lock::LockEntry,
) -> Result<(), EnsureLockedError> {
    let dir = cache
        .entry_dir(path, &entry.commit_oid)
        .map_err(|e| EnsureLockedError::Cache {
            path: path.to_string(),
            message: e.to_string(),
        })?;
    match verify_package_integrity(&dir, &entry.commit_oid, &entry.content_hash) {
        Ok(()) => Ok(()),
        Err(PackageIntegrityError::OidMismatch {
            expected, actual, ..
        })
        | Err(PackageIntegrityError::PathOidMismatch {
            expected, actual, ..
        }) => Err(EnsureLockedError::OidMismatch {
            path: path.to_string(),
            expected,
            actual,
        }),
        Err(PackageIntegrityError::MissingMarker { .. }) => Err(EnsureLockedError::OidMismatch {
            path: path.to_string(),
            expected: entry.commit_oid.clone(),
            actual: String::new(),
        }),
        Err(PackageIntegrityError::ContentHash(
            crate::hash::ContentHashVerifyError::Mismatch {
                expected, actual, ..
            },
        )) => Err(EnsureLockedError::ContentHashMismatch {
            path: path.to_string(),
            expected,
            actual,
        }),
        Err(PackageIntegrityError::ContentHash(e)) => Err(EnsureLockedError::ContentHash {
            path: path.to_string(),
            message: e.to_string(),
        }),
    }
}

/// Discover `draconic.lock` walking ancestors of `entry`, then ensure checkouts (K07.01 / K07.02).
///
/// Returns `Ok(None)` when no lockfile is found (plain programs need no package fetch).
/// `offline` forbids network fetch on cache miss.
pub fn ensure_locked_for_entry(
    entry: &Path,
    offline: bool,
) -> Result<Option<EnsureLockedResult>, EnsureLockedError> {
    let Some((workspace, lock)) = discover_lock(entry)? else {
        return Ok(None);
    };
    let cache = ModuleCache::new(default_cache_root(&workspace));
    ensure_locked_entries(&lock, &cache, offline).map(Some)
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
        let result = ensure_locked_entries(&lock, &cache, false).expect("ensure");
        assert_eq!(result.fetched, vec![path.to_string()]);
        assert!(result.kept.is_empty());
        assert!(cache.has_entry(path, &oid).unwrap());

        // Second call is cache hit.
        let again = ensure_locked_entries(&lock, &cache, false).expect("ensure hit");
        assert_eq!(again.kept, vec![path.to_string()]);
        assert!(again.fetched.is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn ensure_locked_entries_offline_misses() {
        let root = temp_dir();
        let (upstream, oid) = tagged_upstream(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/lib";
        let checkout = cache
            .checkout(path, &oid, upstream.to_str().unwrap())
            .unwrap();
        let hash = content_hash_tree(&checkout).unwrap();
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

        let err = ensure_locked_entries(&lock, &cache, true).expect_err("offline miss");
        let msg = err.to_string();
        match &err {
            EnsureLockedError::OfflineMiss { path: p } => assert_eq!(p, path),
            other => panic!("expected OfflineMiss, got {other:?}"),
        }
        assert!(!cache.has_entry(path, &oid).unwrap());
        assert!(msg.contains("offline"), "{msg}");
        assert!(msg.contains("draconic get") || msg.contains("without --offline"), "{msg}");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn ensure_locked_entries_offline_hit() {
        let root = temp_dir();
        let (upstream, oid) = tagged_upstream(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/lib";
        let checkout = cache
            .checkout(path, &oid, upstream.to_str().unwrap())
            .unwrap();
        let hash = content_hash_tree(&checkout).unwrap();

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

        let result = ensure_locked_entries(&lock, &cache, true).expect("offline hit");
        assert_eq!(result.kept, vec![path.to_string()]);
        assert!(result.fetched.is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn ensure_locked_for_entry_no_lock_is_none() {
        let root = temp_dir();
        let main = root.join("main.drac");
        fs::write(&main, "let x = 1;\n").unwrap();
        assert!(ensure_locked_for_entry(&main, false).unwrap().is_none());
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

        let result = ensure_locked_for_entry(&main, false)
            .expect("ensure")
            .expect("lock present");
        assert_eq!(result.fetched, vec![path.to_string()]);
        assert!(ModuleCache::new(&cache_root).has_entry(path, &oid).unwrap());

        let _ = fs::remove_dir_all(&root);
    }

    /// K07.03: lock pin OID wins; newer tags on the remote must not float the checkout.
    #[test]
    fn ensure_locked_entries_does_not_float_past_lock_pin() {
        let root = temp_dir();
        let repo = root.join("upstream");
        fs::create_dir_all(&repo).unwrap();
        git_ok(&["init"], &repo);
        git_ok(&["config", "user.email", "test@draconic.local"], &repo);
        git_ok(&["config", "user.name", "Draconic Test"], &repo);
        git_ok(&["checkout", "-B", "main"], &repo);
        fs::write(repo.join("index.drac"), "export let x = 41;\n").unwrap();
        git_ok(&["add", "."], &repo);
        git_ok(&["commit", "-m", "v1.0.0"], &repo);
        git_ok(&["tag", "v1.0.0"], &repo);
        let oid_v1 = String::from_utf8(
            Command::new("git")
                .args(["-C", repo.to_str().unwrap(), "rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();

        // Newer tag that a floating resolve would prefer.
        fs::write(repo.join("index.drac"), "export let x = 99;\n").unwrap();
        git_ok(&["add", "."], &repo);
        git_ok(&["commit", "-m", "v2.0.0"], &repo);
        git_ok(&["tag", "v2.0.0"], &repo);
        let oid_v2 = String::from_utf8(
            Command::new("git")
                .args(["-C", repo.to_str().unwrap(), "rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();
        assert_ne!(oid_v1, oid_v2);

        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/lib";
        // Seed content hash from the locked (v1) tree only.
        let checkout_v1 = cache
            .checkout(path, &oid_v1, repo.to_str().unwrap())
            .unwrap();
        let hash = content_hash_tree(&checkout_v1).unwrap();
        fs::remove_dir_all(cache.root.join("mod")).unwrap();
        fs::remove_dir_all(cache.root.join("vcs")).ok();

        let entry = LockEntry::new(
            path,
            "1.0.0",
            repo.to_str().unwrap(),
            oid_v1.clone(),
            hash,
        )
        .unwrap();
        let mut packages = BTreeMap::new();
        packages.insert(path.to_string(), entry);
        let lock = LockFile {
            version: 1,
            packages,
        };

        let result = ensure_locked_entries(&lock, &cache, false).expect("ensure");
        assert_eq!(result.fetched, vec![path.to_string()]);
        assert!(cache.has_entry(path, &oid_v1).unwrap());
        assert!(
            !cache.has_entry(path, &oid_v2).unwrap(),
            "must not materialize newer unpinned OID"
        );
        let src = fs::read_to_string(cache.entry_dir(path, &oid_v1).unwrap().join("index.drac"))
            .unwrap();
        assert!(src.contains("41"), "locked pin content: {src}");
        assert!(!src.contains("99"), "must not float to v2 content: {src}");

        let _ = fs::remove_dir_all(&root);
    }

    /// K08.01: cache hit still verifies recomputed tree hash against lock pin.
    #[test]
    fn ensure_locked_entries_verifies_content_hash_on_hit() {
        let root = temp_dir();
        let (upstream, oid) = tagged_upstream(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/lib";
        let checkout = cache
            .checkout(path, &oid, upstream.to_str().unwrap())
            .unwrap();
        let hash = content_hash_tree(&checkout).unwrap();

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

        let result = ensure_locked_entries(&lock, &cache, true).expect("hash ok");
        assert_eq!(result.kept, vec![path.to_string()]);

        let _ = fs::remove_dir_all(&root);
    }

    /// K08.01: tampered checkout on cache hit → hard-fail (no silent wrong tree).
    #[test]
    fn ensure_locked_entries_rejects_tampered_tree() {
        let root = temp_dir();
        let (upstream, oid) = tagged_upstream(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/lib";
        let checkout = cache
            .checkout(path, &oid, upstream.to_str().unwrap())
            .unwrap();
        let hash = content_hash_tree(&checkout).unwrap();

        // Tamper after lock pin was recorded.
        fs::write(checkout.join("index.drac"), "export let x = 666;\n").unwrap();

        let entry = LockEntry::new(
            path,
            "1.0.0",
            upstream.to_str().unwrap(),
            oid.clone(),
            hash.clone(),
        )
        .unwrap();
        let mut packages = BTreeMap::new();
        packages.insert(path.to_string(), entry);
        let lock = LockFile {
            version: 1,
            packages,
        };

        let err = ensure_locked_entries(&lock, &cache, true).expect_err("tamper");
        let msg = err.to_string();
        match &err {
            EnsureLockedError::ContentHashMismatch {
                path: p,
                expected,
                actual,
            } => {
                assert_eq!(p, path);
                assert_eq!(expected, &hash);
                assert_ne!(actual, &hash);
            }
            other => panic!("expected ContentHashMismatch, got {other:?}"),
        }
        assert!(msg.contains("content hash mismatch"), "{msg}");
        assert!(msg.contains(path), "{msg}");

        let _ = fs::remove_dir_all(&root);
    }

    /// K08.01: wrong lock hash (even with correct tree) → hard-fail.
    #[test]
    fn ensure_locked_entries_rejects_wrong_lock_hash() {
        let root = temp_dir();
        let (upstream, oid) = tagged_upstream(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/lib";
        let _checkout = cache
            .checkout(path, &oid, upstream.to_str().unwrap())
            .unwrap();
        let bogus = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

        let entry = LockEntry::new(
            path,
            "1.0.0",
            upstream.to_str().unwrap(),
            oid.clone(),
            bogus,
        )
        .unwrap();
        let mut packages = BTreeMap::new();
        packages.insert(path.to_string(), entry);
        let lock = LockFile {
            version: 1,
            packages,
        };

        let err = ensure_locked_entries(&lock, &cache, true).expect_err("wrong hash");
        match err {
            EnsureLockedError::ContentHashMismatch { expected, .. } => {
                assert_eq!(expected, bogus);
            }
            other => panic!("expected ContentHashMismatch, got {other:?}"),
        }

        let _ = fs::remove_dir_all(&root);
    }

    /// K08.01: after fetch, still verify hash before reporting success.
    #[test]
    fn ensure_locked_entries_verifies_content_hash_after_fetch() {
        let root = temp_dir();
        let (upstream, oid) = tagged_upstream(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/lib";
        let checkout = cache
            .checkout(path, &oid, upstream.to_str().unwrap())
            .unwrap();
        let hash = content_hash_tree(&checkout).unwrap();
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

        let result = ensure_locked_entries(&lock, &cache, false).expect("fetch+verify");
        assert_eq!(result.fetched, vec![path.to_string()]);
        assert!(cache.has_entry(path, &oid).unwrap());

        let _ = fs::remove_dir_all(&root);
    }

    /// K08.02: wrong checkout marker OID under the pin path → refuse (no silent wrong tree).
    #[test]
    fn ensure_locked_entries_rejects_marker_oid_mismatch() {
        let root = temp_dir();
        let (upstream, oid) = tagged_upstream(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/lib";
        let checkout = cache
            .checkout(path, &oid, upstream.to_str().unwrap())
            .unwrap();
        let hash = content_hash_tree(&checkout).unwrap();
        // Corrupt marker to a different OID while leaving tree in place.
        let other = "ffffffffffffffffffffffffffffffffffffffff";
        fs::write(checkout.join(".draconic-checkout-oid"), format!("{other}\n")).unwrap();
        assert!(!cache.has_entry(path, &oid).unwrap());

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

        let err = ensure_locked_entries(&lock, &cache, true).expect_err("oid mismatch");
        let msg = err.to_string();
        match &err {
            EnsureLockedError::OidMismatch {
                path: p,
                expected,
                actual,
            } => {
                assert_eq!(p, path);
                assert_eq!(expected, &oid);
                assert_eq!(actual, other);
            }
            other => panic!("expected OidMismatch, got {other:?}"),
        }
        assert!(msg.contains("OID mismatch"), "{msg}");
        assert!(msg.contains("refuse") || msg.contains("wrong tree"), "{msg}");

        let _ = fs::remove_dir_all(&root);
    }

    /// K08.02: online ensure still refuses mismatched marker (does not silently overwrite).
    #[test]
    fn ensure_locked_entries_online_refuses_oid_mismatch() {
        let root = temp_dir();
        let (upstream, oid) = tagged_upstream(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/lib";
        let checkout = cache
            .checkout(path, &oid, upstream.to_str().unwrap())
            .unwrap();
        let hash = content_hash_tree(&checkout).unwrap();
        let other = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        fs::write(checkout.join(".draconic-checkout-oid"), format!("{other}\n")).unwrap();

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

        let err = ensure_locked_entries(&lock, &cache, false).expect_err("online oid");
        assert!(
            matches!(err, EnsureLockedError::OidMismatch { .. }),
            "{err:?}"
        );

        let _ = fs::remove_dir_all(&root);
    }
}
