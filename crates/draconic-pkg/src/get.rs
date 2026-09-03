//! `draconic get <module_path>@<ver>`: fetch, update manifest + lock + cache (K05.01).

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cache::ModuleCache;
use crate::content_hash_tree;
use crate::lock::{parse_lock, write_lock, LockEntry, LockFile};
use crate::resolve::{resolve_highest_matching_tag, ResolveError};
use crate::{
    parse_manifest, resolve_git_url, sanitize_stored_git_url, validate_git_url,
    validate_module_path, validate_version_req, write_manifest, GitAuth, ManifestError,
};

/// Default relative cache dir under the workspace when no override is given.
pub const DEFAULT_CACHE_DIR_NAME: &str = ".draconic/mod-cache";

/// Manifest file name at the workspace root.
pub const MANIFEST_FILE: &str = "draconic.toml";

/// Lockfile name at the workspace root.
pub const LOCK_FILE: &str = "draconic.lock";

/// Result of a successful `get` (K05.01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetResult {
    /// Module path that was added/updated.
    pub path: String,
    /// Version requirement written into the manifest.
    pub version_req: String,
    /// Concrete resolved version (no operator).
    pub resolved_version: String,
    /// Commit OID pinned in the lock.
    pub commit_oid: String,
    /// Checkout directory under the module cache.
    pub checkout_dir: PathBuf,
}

/// Error while running package get.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GetError {
    /// Spec is not `module_path@ver`.
    InvalidSpec { spec: String, reason: &'static str },
    /// Module path schema failed.
    InvalidPath { path: String, reason: &'static str },
    /// Version requirement schema failed.
    InvalidVersionReq { req: String, reason: &'static str },
    /// Optional `--url` failed validation.
    InvalidUrl { url: String, reason: &'static str },
    /// Workspace has no readable `draconic.toml`.
    MissingManifest { path: String },
    /// Manifest parse/validate failed.
    Manifest(String),
    /// Cannot depend on own module path.
    SelfDependency { path: String },
    /// Existing lockfile is malformed.
    Lock(String),
    /// Clone/fetch/checkout failed.
    Cache { path: String, message: String },
    /// Version/tag resolve failed.
    Resolve { path: String, source: ResolveError },
    /// Content hash failed.
    ContentHash { path: String, message: String },
    /// Built lock entry invalid.
    LockEntry { path: String, message: String },
    /// Writing manifest/lock failed.
    Io(String),
}

impl fmt::Display for GetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GetError::InvalidSpec { spec, reason } => {
                write!(f, "get: invalid spec `{spec}`: {reason}")
            }
            GetError::InvalidPath { path, reason } => {
                write!(f, "get: invalid module path `{path}`: {reason}")
            }
            GetError::InvalidVersionReq { req, reason } => {
                write!(f, "get: invalid version requirement `{req}`: {reason}")
            }
            GetError::InvalidUrl { url, reason } => {
                write!(f, "get: invalid git URL `{url}`: {reason}")
            }
            GetError::MissingManifest { path } => {
                write!(f, "get: missing `{path}` (run from a package root)")
            }
            GetError::Manifest(msg) => write!(f, "get: {msg}"),
            GetError::SelfDependency { path } => {
                write!(f, "get: package cannot depend on itself (`{path}`)")
            }
            GetError::Lock(msg) => write!(f, "get: {msg}"),
            GetError::Cache { path, message } => {
                write!(f, "get: `{path}` cache: {message}")
            }
            GetError::Resolve { path, source } => {
                write!(f, "get: `{path}`: {source}")
            }
            GetError::ContentHash { path, message } => {
                write!(f, "get: `{path}` content hash: {message}")
            }
            GetError::LockEntry { path, message } => {
                write!(f, "get: `{path}` lock entry: {message}")
            }
            GetError::Io(msg) => write!(f, "get: {msg}"),
        }
    }
}

impl std::error::Error for GetError {}

impl From<ManifestError> for GetError {
    fn from(e: ManifestError) -> Self {
        GetError::Manifest(e.to_string())
    }
}

/// Parse `module_path@ver` into (path, version_req).
///
/// The last `@` separates path from version so paths never contain `@`.
pub fn parse_get_spec(spec: &str) -> Result<(String, String), GetError> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err(GetError::InvalidSpec {
            spec: spec.to_string(),
            reason: "must not be empty",
        });
    }
    let Some((path, ver)) = spec.rsplit_once('@') else {
        return Err(GetError::InvalidSpec {
            spec: spec.to_string(),
            reason: "expected module_path@version (missing '@')",
        });
    };
    if path.is_empty() {
        return Err(GetError::InvalidSpec {
            spec: spec.to_string(),
            reason: "module path before '@' is empty",
        });
    }
    if ver.is_empty() {
        return Err(GetError::InvalidSpec {
            spec: spec.to_string(),
            reason: "version after '@' is empty",
        });
    }
    if let Err(reason) = validate_module_path(path) {
        return Err(GetError::InvalidPath {
            path: path.to_string(),
            reason,
        });
    }
    if let Err(reason) = validate_version_req(ver) {
        return Err(GetError::InvalidVersionReq {
            req: ver.to_string(),
            reason,
        });
    }
    Ok((path.to_string(), ver.to_string()))
}

/// Default module cache root for a workspace directory.
pub fn default_cache_root(workspace: &Path) -> PathBuf {
    if let Ok(p) = std::env::var("DRACONIC_MOD_CACHE") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    workspace.join(DEFAULT_CACHE_DIR_NAME)
}

/// Add or update a dependency: fetch into `cache`, write `draconic.toml` + `draconic.lock`.
///
/// Steps (K05.01):
/// 1. Load workspace `draconic.toml`
/// 2. Insert/update `dependencies[path] = version_req` (and optional `urls` override)
/// 3. Clone/fetch, resolve highest matching tag, checkout, content-hash
/// 4. Merge pin into existing lock (or create empty lock)
/// 5. Write manifest + lock
pub fn get_package(
    workspace: &Path,
    module_path: &str,
    version_req: &str,
    git_url_override: Option<&str>,
    cache: &ModuleCache,
) -> Result<GetResult, GetError> {
    get_package_with_auth(
        workspace,
        module_path,
        version_req,
        git_url_override,
        cache,
        &GitAuth::from_env(),
    )
}

/// [`get_package`] with explicit [`GitAuth`] (K11.01). Credentials are never written
/// to `draconic.toml` or `draconic.lock`.
pub fn get_package_with_auth(
    workspace: &Path,
    module_path: &str,
    version_req: &str,
    git_url_override: Option<&str>,
    cache: &ModuleCache,
    auth: &GitAuth,
) -> Result<GetResult, GetError> {
    if let Err(reason) = validate_module_path(module_path) {
        return Err(GetError::InvalidPath {
            path: module_path.to_string(),
            reason,
        });
    }
    if let Err(reason) = validate_version_req(version_req) {
        return Err(GetError::InvalidVersionReq {
            req: version_req.to_string(),
            reason,
        });
    }
    if let Some(url) = git_url_override {
        if let Err(reason) = validate_git_url(url) {
            return Err(GetError::InvalidUrl {
                url: url.to_string(),
                reason,
            });
        }
    }

    let manifest_path = workspace.join(MANIFEST_FILE);
    let lock_path = workspace.join(LOCK_FILE);

    let src = fs::read_to_string(&manifest_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            GetError::MissingManifest {
                path: manifest_path.display().to_string(),
            }
        } else {
            GetError::Io(format!("read {}: {e}", manifest_path.display()))
        }
    })?;
    let mut manifest = parse_manifest(&src)?;

    if module_path == manifest.module {
        return Err(GetError::SelfDependency {
            path: module_path.to_string(),
        });
    }

    manifest
        .dependencies
        .insert(module_path.to_string(), version_req.to_string());

    // Clone may use userinfo in the override; stored urls/lock never persist secrets (K11.01).
    let clone_url = git_url_override
        .map(str::to_string)
        .unwrap_or_else(|| resolve_git_url(&manifest, module_path));
    if let Some(url) = git_url_override {
        manifest
            .urls
            .insert(module_path.to_string(), sanitize_stored_git_url(url));
    }

    // Re-validate after mutation (self-dep already checked; paths/reqs validated above).
    crate::validate_manifest(&manifest)?;

    let stored_url = sanitize_stored_git_url(&clone_url);

    let vcs = cache
        .clone_or_fetch_with_auth(module_path, &clone_url, auth)
        .map_err(|e| GetError::Cache {
            path: module_path.to_string(),
            message: e.to_string(),
        })?;

    let resolved =
        resolve_highest_matching_tag(&vcs, version_req).map_err(|source| GetError::Resolve {
            path: module_path.to_string(),
            source,
        })?;

    let checkout = cache
        .checkout(module_path, &resolved.commit_oid, &clone_url)
        .map_err(|e| GetError::Cache {
            path: module_path.to_string(),
            message: e.to_string(),
        })?;

    let content_hash = content_hash_tree(&checkout).map_err(|e| GetError::ContentHash {
        path: module_path.to_string(),
        message: e.to_string(),
    })?;

    let entry = LockEntry::new(
        module_path.to_string(),
        resolved.version.clone(),
        stored_url,
        resolved.commit_oid.clone(),
        content_hash,
    )
    .map_err(|e| GetError::LockEntry {
        path: module_path.to_string(),
        message: e.to_string(),
    })?;

    let mut lock = load_or_empty_lock(&lock_path)?;
    lock.packages.insert(module_path.to_string(), entry);

    fs::write(&manifest_path, write_manifest(&manifest))
        .map_err(|e| GetError::Io(format!("write {}: {e}", manifest_path.display())))?;
    fs::write(&lock_path, write_lock(&lock))
        .map_err(|e| GetError::Io(format!("write {}: {e}", lock_path.display())))?;

    Ok(GetResult {
        path: module_path.to_string(),
        version_req: version_req.to_string(),
        resolved_version: resolved.version,
        commit_oid: resolved.commit_oid,
        checkout_dir: checkout,
    })
}

fn load_or_empty_lock(lock_path: &Path) -> Result<LockFile, GetError> {
    match fs::read_to_string(lock_path) {
        Ok(src) => parse_lock(&src).map_err(|e| GetError::Lock(e.to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(LockFile {
            version: 1,
            packages: Default::default(),
        }),
        Err(e) => Err(GetError::Io(format!("read {}: {e}", lock_path.display()))),
    }
}

/// Convenience: parse spec then [`get_package`].
pub fn get_package_spec(
    workspace: &Path,
    spec: &str,
    git_url_override: Option<&str>,
    cache: &ModuleCache,
) -> Result<GetResult, GetError> {
    let (path, ver) = parse_get_spec(spec)?;
    get_package(workspace, &path, &ver, git_url_override, cache)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ModuleCache;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(tag: &str) -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "draconic-pkg-k0501-{tag}-{}-{}-{}",
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
        fs::write(repo.join("lib.drac"), "export let x = 1;\n").unwrap();
        git_ok(&["add", "."], &repo);
        git_ok(&["commit", "-m", "v1.2.3"], &repo);
        let oid = {
            let out = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&repo)
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };
        git_ok(&["tag", "v1.2.3"], &repo);
        git_ok(&["tag", "v1.0.0"], &repo);
        (repo, oid)
    }

    #[test]
    fn parse_get_spec_ok() {
        let (p, v) = parse_get_spec("github.com/org/lib@1.2.3").unwrap();
        assert_eq!(p, "github.com/org/lib");
        assert_eq!(v, "1.2.3");
        let (p, v) = parse_get_spec("github.com/org/lib@^1.0").unwrap();
        assert_eq!(p, "github.com/org/lib");
        assert_eq!(v, "^1.0");
    }

    #[test]
    fn parse_get_spec_rejects_bad() {
        assert!(matches!(
            parse_get_spec("no-at-sign"),
            Err(GetError::InvalidSpec { .. })
        ));
        assert!(matches!(
            parse_get_spec("@1.0.0"),
            Err(GetError::InvalidSpec { .. })
        ));
        assert!(matches!(
            parse_get_spec("github.com/org/lib@"),
            Err(GetError::InvalidSpec { .. })
        ));
        assert!(matches!(
            parse_get_spec("not-a-path@1.0.0"),
            Err(GetError::InvalidPath { .. })
        ));
        assert!(matches!(
            parse_get_spec("github.com/org/lib@latest"),
            Err(GetError::InvalidVersionReq { .. })
        ));
    }

    #[test]
    fn k05_01_get_updates_manifest_lock_cache() {
        let root = temp_dir("happy");
        let (upstream, oid) = tagged_upstream(&root);
        let ws = root.join("app");
        fs::create_dir_all(&ws).unwrap();
        fs::write(ws.join(MANIFEST_FILE), "module = \"github.com/acme/app\"\n").unwrap();

        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/lib";
        let result = get_package(
            &ws,
            path,
            "^1.0.0",
            Some(upstream.to_str().unwrap()),
            &cache,
        )
        .expect("get");

        assert_eq!(result.path, path);
        assert_eq!(result.version_req, "^1.0.0");
        assert_eq!(result.resolved_version, "1.2.3");
        assert_eq!(result.commit_oid, oid);
        assert!(result.checkout_dir.is_dir());
        assert!(cache.has_entry(path, &oid).unwrap());

        let m = parse_manifest(&fs::read_to_string(ws.join(MANIFEST_FILE)).unwrap()).unwrap();
        assert_eq!(m.dependencies.get(path).map(String::as_str), Some("^1.0.0"));
        assert_eq!(
            m.urls.get(path).map(String::as_str),
            Some(upstream.to_str().unwrap())
        );

        let lock = parse_lock(&fs::read_to_string(ws.join(LOCK_FILE)).unwrap()).unwrap();
        let e = lock.packages.get(path).expect("lock pin");
        assert_eq!(e.version, "1.2.3");
        assert_eq!(e.commit_oid, oid);
        assert_eq!(e.content_hash.len(), 64);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k05_01_get_merges_existing_lock() {
        let root = temp_dir("merge");
        let (up_a, oid_a) = {
            let (r, o) = tagged_upstream(&root);
            // retag path for second dep
            (r, o)
        };
        let up_b = root.join("upstream-b");
        fs::create_dir_all(&up_b).unwrap();
        git_ok(&["init"], &up_b);
        git_ok(&["config", "user.email", "test@draconic.local"], &up_b);
        git_ok(&["config", "user.name", "Draconic Test"], &up_b);
        git_ok(&["checkout", "-B", "main"], &up_b);
        fs::write(up_b.join("b.txt"), "b\n").unwrap();
        git_ok(&["add", "."], &up_b);
        git_ok(&["commit", "-m", "v2.0.0"], &up_b);
        let oid_b = {
            let out = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&up_b)
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };
        git_ok(&["tag", "v2.0.0"], &up_b);

        let ws = root.join("app");
        fs::create_dir_all(&ws).unwrap();
        let cache = ModuleCache::new(root.join("cache"));

        // Seed first dep.
        fs::write(ws.join(MANIFEST_FILE), "module = \"github.com/acme/app\"\n").unwrap();
        get_package(
            &ws,
            "github.com/a/first",
            "1.2.3",
            Some(up_a.to_str().unwrap()),
            &cache,
        )
        .expect("get a");

        get_package(
            &ws,
            "github.com/b/second",
            "2.0.0",
            Some(up_b.to_str().unwrap()),
            &cache,
        )
        .expect("get b");

        let lock = parse_lock(&fs::read_to_string(ws.join(LOCK_FILE)).unwrap()).unwrap();
        assert_eq!(lock.packages.len(), 2);
        assert_eq!(lock.packages["github.com/a/first"].commit_oid, oid_a);
        assert_eq!(lock.packages["github.com/b/second"].commit_oid, oid_b);

        let m = parse_manifest(&fs::read_to_string(ws.join(MANIFEST_FILE)).unwrap()).unwrap();
        assert_eq!(m.dependencies.len(), 2);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k05_01_missing_manifest() {
        let root = temp_dir("nomf");
        let cache = ModuleCache::new(root.join("cache"));
        let err =
            get_package(&root, "github.com/org/lib", "1.0.0", None, &cache).expect_err("missing");
        assert!(matches!(err, GetError::MissingManifest { .. }), "{err:?}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k05_01_self_dependency() {
        let root = temp_dir("self");
        let ws = root.join("app");
        fs::create_dir_all(&ws).unwrap();
        fs::write(ws.join(MANIFEST_FILE), "module = \"github.com/org/lib\"\n").unwrap();
        let cache = ModuleCache::new(root.join("cache"));
        let err = get_package(&ws, "github.com/org/lib", "1.0.0", None, &cache).expect_err("self");
        assert!(matches!(err, GetError::SelfDependency { .. }), "{err:?}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k05_01_spec_convenience() {
        let root = temp_dir("spec");
        let (upstream, oid) = tagged_upstream(&root);
        let ws = root.join("app");
        fs::create_dir_all(&ws).unwrap();
        fs::write(ws.join(MANIFEST_FILE), "module = \"github.com/acme/app\"\n").unwrap();
        let cache = ModuleCache::new(root.join("cache"));
        let r = get_package_spec(
            &ws,
            "github.com/org/lib@1.2.3",
            Some(upstream.to_str().unwrap()),
            &cache,
        )
        .expect("spec get");
        assert_eq!(r.commit_oid, oid);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k11_01_get_does_not_write_https_token_to_manifest_or_lock() {
        let root = temp_dir("k11-01-secret");
        let (upstream, _oid) = tagged_upstream(&root);
        let ws = root.join("app");
        fs::create_dir_all(&ws).unwrap();
        fs::write(ws.join(MANIFEST_FILE), "module = \"github.com/acme/app\"\n").unwrap();
        let cache = ModuleCache::new(root.join("cache"));
        let token = "s3cret-token-k11-01";
        let auth = GitAuth::https_token("git", token).unwrap();
        get_package_with_auth(
            &ws,
            "github.com/org/lib",
            "1.2.3",
            Some(upstream.to_str().unwrap()),
            &cache,
            &auth,
        )
        .expect("get with token");

        let mf = fs::read_to_string(ws.join(MANIFEST_FILE)).unwrap();
        let lock = fs::read_to_string(ws.join(LOCK_FILE)).unwrap();
        assert!(!mf.contains(token), "manifest leaked token:\n{mf}");
        assert!(!lock.contains(token), "lock leaked token:\n{lock}");
        assert!(!mf.contains("DRACONIC_GIT_TOKEN"), "{mf}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k11_01_get_missing_ssh_identity_fails_closed() {
        let root = temp_dir("k11-01-missing-ssh");
        let ws = root.join("app");
        fs::create_dir_all(&ws).unwrap();
        fs::write(ws.join(MANIFEST_FILE), "module = \"github.com/acme/app\"\n").unwrap();
        let cache = ModuleCache::new(root.join("cache"));
        let auth = GitAuth::Ssh {
            identity_file: Some(PathBuf::from("/no/such/k11-01-ssh-key")),
        };
        let err = get_package_with_auth(
            &ws,
            "github.com/org/lib",
            "1.0.0",
            Some("git@github.com:org/lib.git"),
            &cache,
            &auth,
        )
        .expect_err("missing ssh identity");
        let msg = err.to_string();
        assert!(msg.contains("missing"), "{msg}");
        assert!(msg.contains("SSH") || msg.contains("ssh"), "{msg}");
        assert!(
            !ws.join(LOCK_FILE).exists(),
            "must not write lock on auth failure"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
