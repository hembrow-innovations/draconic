//! `draconic mod tidy`: lock matches manifest; fetch missing; prune unused (K05.02).

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cache::ModuleCache;
use crate::content_hash_tree;
use crate::get::{default_cache_root, LOCK_FILE, MANIFEST_FILE};
use crate::lock::{parse_lock, write_lock, LockEntry, LockFile};
use crate::resolve::{resolve_highest_matching_tag, version_satisfies_req, ResolveError};
use crate::{parse_manifest, resolve_git_url, Manifest, ManifestError};

/// Summary of a successful tidy (K05.02).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TidyResult {
    /// Paths whose existing lock pins were kept (still satisfy req + cache).
    pub kept: Vec<String>,
    /// Paths newly resolved/fetched into lock + cache.
    pub fetched: Vec<String>,
    /// Paths removed from lock (not in manifest deps).
    pub pruned: Vec<String>,
    /// Written lock path.
    pub lock_path: PathBuf,
}

/// Error while running `mod tidy`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TidyError {
    /// Workspace has no readable `draconic.toml`.
    MissingManifest { path: String },
    /// Manifest parse/validate failed.
    Manifest(String),
    /// Existing lockfile is malformed.
    Lock(String),
    /// Clone/fetch/checkout failed for a dependency.
    Cache { path: String, message: String },
    /// Version/tag resolve failed.
    Resolve { path: String, source: ResolveError },
    /// Content hash failed.
    ContentHash { path: String, message: String },
    /// Built lock entry invalid.
    LockEntry { path: String, message: String },
    /// Writing lock failed.
    Io(String),
}

impl fmt::Display for TidyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TidyError::MissingManifest { path } => {
                write!(f, "mod tidy: missing `{path}` (run from a package root)")
            }
            TidyError::Manifest(msg) => write!(f, "mod tidy: {msg}"),
            TidyError::Lock(msg) => write!(f, "mod tidy: {msg}"),
            TidyError::Cache { path, message } => {
                write!(f, "mod tidy: `{path}` cache: {message}")
            }
            TidyError::Resolve { path, source } => {
                write!(f, "mod tidy: `{path}`: {source}")
            }
            TidyError::ContentHash { path, message } => {
                write!(f, "mod tidy: `{path}` content hash: {message}")
            }
            TidyError::LockEntry { path, message } => {
                write!(f, "mod tidy: `{path}` lock entry: {message}")
            }
            TidyError::Io(msg) => write!(f, "mod tidy: {msg}"),
        }
    }
}

impl std::error::Error for TidyError {}

impl From<ManifestError> for TidyError {
    fn from(e: ManifestError) -> Self {
        TidyError::Manifest(e.to_string())
    }
}

/// Align `draconic.lock` with `draconic.toml` direct deps (K05.02).
///
/// 1. Load workspace manifest
/// 2. Load existing lock (or empty)
/// 3. For each manifest dep (sorted): keep valid lock pin if it still satisfies
///    the version req, git URL matches, and checkout is available; else resolve,
///    fetch, checkout, content-hash, and pin
/// 4. Drop lock packages not listed in the manifest
/// 5. Write lock (manifest unchanged)
pub fn mod_tidy(workspace: &Path, cache: &ModuleCache) -> Result<TidyResult, TidyError> {
    let manifest_path = workspace.join(MANIFEST_FILE);
    let lock_path = workspace.join(LOCK_FILE);

    let src = fs::read_to_string(&manifest_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            TidyError::MissingManifest {
                path: manifest_path.display().to_string(),
            }
        } else {
            TidyError::Io(format!("read {}: {e}", manifest_path.display()))
        }
    })?;
    let manifest = parse_manifest(&src)?;

    let old_lock = load_or_empty_lock(&lock_path)?;
    let (new_lock, kept, fetched) = rebuild_lock(&manifest, &old_lock, cache)?;

    let mut pruned: Vec<String> = old_lock
        .packages
        .keys()
        .filter(|p| !new_lock.packages.contains_key(p.as_str()))
        .cloned()
        .collect();
    pruned.sort();

    fs::write(&lock_path, write_lock(&new_lock))
        .map_err(|e| TidyError::Io(format!("write {}: {e}", lock_path.display())))?;

    Ok(TidyResult {
        kept,
        fetched,
        pruned,
        lock_path,
    })
}

/// Convenience: tidy using [`default_cache_root`] for `workspace`.
pub fn mod_tidy_default_cache(workspace: &Path) -> Result<TidyResult, TidyError> {
    let cache = ModuleCache::new(default_cache_root(workspace));
    mod_tidy(workspace, &cache)
}

fn load_or_empty_lock(lock_path: &Path) -> Result<LockFile, TidyError> {
    match fs::read_to_string(lock_path) {
        Ok(src) => parse_lock(&src).map_err(|e| TidyError::Lock(e.to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(LockFile {
            version: 1,
            packages: Default::default(),
        }),
        Err(e) => Err(TidyError::Io(format!("read {}: {e}", lock_path.display()))),
    }
}

fn rebuild_lock(
    manifest: &Manifest,
    old_lock: &LockFile,
    cache: &ModuleCache,
) -> Result<(LockFile, Vec<String>, Vec<String>), TidyError> {
    let mut packages = std::collections::BTreeMap::new();
    let mut kept = Vec::new();
    let mut fetched = Vec::new();

    for (path, req) in &manifest.dependencies {
        let git_url = resolve_git_url(manifest, path);

        if let Some(entry) = old_lock.packages.get(path) {
            if entry.git_url == git_url
                && version_satisfies_req(&entry.version, req)
                && try_ensure_checkout(cache, path, entry).is_ok()
            {
                packages.insert(path.clone(), entry.clone());
                kept.push(path.clone());
                continue;
            }
        }

        let entry = resolve_and_pin(cache, path, req, &git_url)?;
        packages.insert(path.clone(), entry);
        fetched.push(path.clone());
    }

    Ok((
        LockFile {
            version: 1,
            packages,
        },
        kept,
        fetched,
    ))
}

fn try_ensure_checkout(cache: &ModuleCache, path: &str, entry: &LockEntry) -> Result<(), ()> {
    if cache.has_entry(path, &entry.commit_oid).unwrap_or(false) {
        return Ok(());
    }
    cache
        .checkout(path, &entry.commit_oid, &entry.git_url)
        .map(|_| ())
        .map_err(|_| ())
}

fn resolve_and_pin(
    cache: &ModuleCache,
    path: &str,
    req: &str,
    git_url: &str,
) -> Result<LockEntry, TidyError> {
    let vcs = cache
        .clone_or_fetch(path, git_url)
        .map_err(|e| TidyError::Cache {
            path: path.to_string(),
            message: e.to_string(),
        })?;

    let resolved =
        resolve_highest_matching_tag(&vcs, req).map_err(|source| TidyError::Resolve {
            path: path.to_string(),
            source,
        })?;

    let checkout = cache
        .checkout(path, &resolved.commit_oid, git_url)
        .map_err(|e| TidyError::Cache {
            path: path.to_string(),
            message: e.to_string(),
        })?;

    let content_hash = content_hash_tree(&checkout).map_err(|e| TidyError::ContentHash {
        path: path.to_string(),
        message: e.to_string(),
    })?;

    LockEntry::new(
        path.to_string(),
        resolved.version,
        git_url.to_string(),
        resolved.commit_oid,
        content_hash,
    )
    .map_err(|e| TidyError::LockEntry {
        path: path.to_string(),
        message: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::get::get_package;
    use crate::write_manifest;
    use crate::ModuleCache;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(tag: &str) -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "draconic-pkg-k0502-{tag}-{}-{}-{}",
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

    fn tagged_upstream(root: &Path, name: &str, tags: &[&str]) -> (PathBuf, String) {
        let repo = root.join(name);
        fs::create_dir_all(&repo).unwrap();
        git_ok(&["init"], &repo);
        git_ok(&["config", "user.email", "test@draconic.local"], &repo);
        git_ok(&["config", "user.name", "Draconic Test"], &repo);
        git_ok(&["checkout", "-B", "main"], &repo);
        fs::write(
            repo.join("lib.drac"),
            format!("export let x = \"{name}\";\n"),
        )
        .unwrap();
        git_ok(&["add", "."], &repo);
        git_ok(&["commit", "-m", "init"], &repo);
        let oid = {
            let out = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&repo)
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };
        for t in tags {
            git_ok(&["tag", t], &repo);
        }
        (repo, oid)
    }

    #[test]
    fn k05_02_tidy_fetches_missing_lock_from_manifest() {
        let root = temp_dir("fetch");
        let (upstream, oid) = tagged_upstream(&root, "up", &["v1.2.3", "v1.0.0"]);
        let ws = root.join("app");
        fs::create_dir_all(&ws).unwrap();
        let path = "github.com/org/lib";
        fs::write(
            ws.join(MANIFEST_FILE),
            format!(
                r#"module = "github.com/acme/app"

[dependencies]
"{path}" = "^1.0.0"

[urls]
"{path}" = "{url}"
"#,
                path = path,
                url = upstream.to_str().unwrap()
            ),
        )
        .unwrap();

        let cache = ModuleCache::new(root.join("cache"));
        let r = mod_tidy(&ws, &cache).expect("tidy");
        assert!(r.fetched.iter().any(|p| p == path), "{r:?}");
        assert!(r.kept.is_empty(), "{r:?}");
        assert!(r.pruned.is_empty(), "{r:?}");

        let lock = parse_lock(&fs::read_to_string(ws.join(LOCK_FILE)).unwrap()).unwrap();
        let e = lock.packages.get(path).expect("pin");
        assert_eq!(e.version, "1.2.3");
        assert_eq!(e.commit_oid, oid);
        assert!(cache.has_entry(path, &oid).unwrap());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k05_02_tidy_prunes_unused_lock_entries() {
        let root = temp_dir("prune");
        let (up_keep, oid_keep) = tagged_upstream(&root, "keep", &["v1.0.0"]);
        let (up_drop, oid_drop) = tagged_upstream(&root, "drop", &["v2.0.0"]);
        let ws = root.join("app");
        fs::create_dir_all(&ws).unwrap();
        let cache = ModuleCache::new(root.join("cache"));

        fs::write(ws.join(MANIFEST_FILE), "module = \"github.com/acme/app\"\n").unwrap();
        get_package(
            &ws,
            "github.com/a/keep",
            "1.0.0",
            Some(up_keep.to_str().unwrap()),
            &cache,
        )
        .expect("get keep");
        get_package(
            &ws,
            "github.com/b/drop",
            "2.0.0",
            Some(up_drop.to_str().unwrap()),
            &cache,
        )
        .expect("get drop");

        // Manifest keeps only `a/keep`.
        let m = parse_manifest(&fs::read_to_string(ws.join(MANIFEST_FILE)).unwrap()).unwrap();
        let mut slim = m.clone();
        slim.dependencies.remove("github.com/b/drop");
        slim.urls.remove("github.com/b/drop");
        fs::write(ws.join(MANIFEST_FILE), write_manifest(&slim)).unwrap();

        let r = mod_tidy(&ws, &cache).expect("tidy");
        assert!(r.pruned.iter().any(|p| p == "github.com/b/drop"), "{r:?}");
        assert!(
            r.kept.iter().any(|p| p == "github.com/a/keep")
                || r.fetched.iter().any(|p| p == "github.com/a/keep"),
            "{r:?}"
        );

        let lock = parse_lock(&fs::read_to_string(ws.join(LOCK_FILE)).unwrap()).unwrap();
        assert_eq!(lock.packages.len(), 1);
        assert!(lock.packages.contains_key("github.com/a/keep"));
        assert!(!lock.packages.contains_key("github.com/b/drop"));
        assert_eq!(lock.packages["github.com/a/keep"].commit_oid, oid_keep);
        let _ = oid_drop;

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k05_02_tidy_keeps_existing_pin_when_still_valid() {
        let root = temp_dir("keep");
        let (upstream, oid) = tagged_upstream(&root, "up", &["v1.0.0", "v1.5.0"]);
        let ws = root.join("app");
        fs::create_dir_all(&ws).unwrap();
        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/lib";

        fs::write(ws.join(MANIFEST_FILE), "module = \"github.com/acme/app\"\n").unwrap();
        // Pin exact 1.0.0 first.
        get_package(&ws, path, "1.0.0", Some(upstream.to_str().unwrap()), &cache).expect("get");

        // Widen req but keep lock pin 1.0.0 (do not float to 1.5.0).
        let mut m = parse_manifest(&fs::read_to_string(ws.join(MANIFEST_FILE)).unwrap()).unwrap();
        m.dependencies.insert(path.to_string(), "^1.0.0".into());
        fs::write(ws.join(MANIFEST_FILE), write_manifest(&m)).unwrap();

        let before = fs::read_to_string(ws.join(LOCK_FILE)).unwrap();
        let r = mod_tidy(&ws, &cache).expect("tidy");
        assert!(r.kept.iter().any(|p| p == path), "{r:?}");
        assert!(!r.fetched.iter().any(|p| p == path), "{r:?}");

        let lock = parse_lock(&fs::read_to_string(ws.join(LOCK_FILE)).unwrap()).unwrap();
        assert_eq!(lock.packages[path].version, "1.0.0");
        assert_eq!(lock.packages[path].commit_oid, oid);
        // Stable when nothing changed.
        let after = fs::read_to_string(ws.join(LOCK_FILE)).unwrap();
        assert_eq!(before, after);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k05_02_tidy_empty_deps_clears_lock() {
        let root = temp_dir("empty");
        let (upstream, _) = tagged_upstream(&root, "up", &["v1.0.0"]);
        let ws = root.join("app");
        fs::create_dir_all(&ws).unwrap();
        let cache = ModuleCache::new(root.join("cache"));

        fs::write(ws.join(MANIFEST_FILE), "module = \"github.com/acme/app\"\n").unwrap();
        get_package(
            &ws,
            "github.com/org/lib",
            "1.0.0",
            Some(upstream.to_str().unwrap()),
            &cache,
        )
        .expect("get");

        fs::write(ws.join(MANIFEST_FILE), "module = \"github.com/acme/app\"\n").unwrap();

        let r = mod_tidy(&ws, &cache).expect("tidy");
        assert_eq!(r.pruned, vec!["github.com/org/lib".to_string()]);
        let lock = parse_lock(&fs::read_to_string(ws.join(LOCK_FILE)).unwrap()).unwrap();
        assert!(lock.packages.is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k05_02_missing_manifest() {
        let root = temp_dir("nomf");
        let cache = ModuleCache::new(root.join("cache"));
        let err = mod_tidy(&root, &cache).expect_err("missing");
        assert!(matches!(err, TidyError::MissingManifest { .. }), "{err:?}");
        let _ = fs::remove_dir_all(&root);
    }
}
