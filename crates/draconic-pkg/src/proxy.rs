//! K11.04: module proxy/mirror (Athens/GOPROXY-shaped); git stays canonical.
//!
//! Fetch may go through a configured proxy or mirror. Module path identity,
//! lock `git_url`, commit OID, and tree hash still name the git tree — never
//! the proxy URL. A missing or failing proxy falls through to the next list
//! entry (`direct` = canonical git) and does not rewrite identity.

use std::fmt;

use crate::auth::{sanitize_stored_git_url, GitAuth};
use crate::cache::{CacheFetchError, ModuleCache};

/// Environment variable for a GOPROXY-shaped module proxy list.
pub const PROXY_ENV: &str = "DRACONIC_PROXY";

/// One entry in a GOPROXY-shaped proxy list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyEntry {
    /// HTTP(S), `file://`, or absolute-path mirror base. Fetch may go here.
    Mirror(String),
    /// Canonical git clone of the lock/manifest git URL.
    Direct,
    /// Stop the list; do not fetch further (including git).
    Off,
}

/// Ordered GOPROXY-shaped proxy list (K11.04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleProxy {
    /// Entries tried in order. `off` stops the list.
    pub entries: Vec<ProxyEntry>,
}

/// One fetch attempt: clone URL vs the git identity stored in lock/origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyFetch {
    /// URL passed to git clone/fetch (mirror or canonical).
    pub fetch_url: String,
    /// Canonical git URL stored in lock / `origin` (never the proxy).
    pub canonical_git_url: String,
}

/// Error while parsing or applying a module proxy list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyError {
    /// A list token is not `direct`, `off`, or an allowed proxy base.
    InvalidBase { base: String, reason: &'static str },
    /// Proxy list is `off` (or `off` before any fetchable entry).
    Off,
}

impl fmt::Display for ProxyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyError::InvalidBase { base, reason } => {
                write!(f, "module proxy: invalid base `{base}`: {reason}")
            }
            ProxyError::Off => write!(f, "module proxy is off; fetch disabled"),
        }
    }
}

impl std::error::Error for ProxyError {}

impl ModuleProxy {
    /// No proxy: fetch only from canonical git.
    pub fn direct() -> Self {
        Self {
            entries: vec![ProxyEntry::Direct],
        }
    }

    /// Parse a GOPROXY-shaped comma list (`url,direct`, `off`, …). Empty → direct.
    pub fn parse(spec: &str) -> Result<Self, ProxyError> {
        let spec = spec.trim();
        if spec.is_empty() {
            return Ok(Self::direct());
        }
        let mut entries = Vec::new();
        for token in spec.split(',') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            if token.eq_ignore_ascii_case("direct") {
                entries.push(ProxyEntry::Direct);
                continue;
            }
            if token.eq_ignore_ascii_case("off") {
                entries.push(ProxyEntry::Off);
                continue;
            }
            validate_proxy_base(token)?;
            entries.push(ProxyEntry::Mirror(token.trim_end_matches('/').to_string()));
        }
        if entries.is_empty() {
            return Ok(Self::direct());
        }
        Ok(Self { entries })
    }

    /// Resolve from process env (`DRACONIC_PROXY`). Unset/empty → [`Self::direct`].
    pub fn from_env() -> Result<Self, ProxyError> {
        module_proxy_from_vars(|k| std::env::var(k).ok())
    }

    /// True when fetch is only canonical git (no mirror, not `off`).
    pub fn is_direct_only(&self) -> bool {
        self.entries.iter().all(|e| matches!(e, ProxyEntry::Direct))
            && !self.entries.is_empty()
            && !self.entries.iter().any(|e| matches!(e, ProxyEntry::Off))
    }

    /// Ordered clone attempts. Every attempt keeps `canonical_git_url` unchanged.
    pub fn fetch_plan(&self, module_path: &str, canonical_git_url: &str) -> Vec<ProxyFetch> {
        let mut out = Vec::new();
        for entry in &self.entries {
            match entry {
                ProxyEntry::Off => break,
                ProxyEntry::Direct => out.push(ProxyFetch {
                    fetch_url: canonical_git_url.to_string(),
                    canonical_git_url: canonical_git_url.to_string(),
                }),
                ProxyEntry::Mirror(base) => out.push(ProxyFetch {
                    fetch_url: mirror_fetch_url(base, module_path),
                    canonical_git_url: canonical_git_url.to_string(),
                }),
            }
        }
        out
    }
}

/// Resolve [`ModuleProxy`] from a key/value lookup (env in production; map in tests).
pub fn module_proxy_from_vars<F>(mut get: F) -> Result<ModuleProxy, ProxyError>
where
    F: FnMut(&str) -> Option<String>,
{
    match get(PROXY_ENV) {
        Some(spec) if !spec.trim().is_empty() => ModuleProxy::parse(&spec),
        _ => Ok(ModuleProxy::direct()),
    }
}

/// Git clone URL a mirror would serve for `module_path`. Does not replace identity.
pub fn mirror_fetch_url(base: &str, module_path: &str) -> String {
    let base = base.trim_end_matches('/');
    if let Some(rest) = base.strip_prefix("file://") {
        let rest = rest.trim_end_matches('/');
        format!("file://{rest}/{module_path}")
    } else {
        format!("{base}/{module_path}")
    }
}

fn validate_proxy_base(base: &str) -> Result<(), ProxyError> {
    if base.is_empty() {
        return Err(ProxyError::InvalidBase {
            base: base.to_string(),
            reason: "must not be empty",
        });
    }
    if base.chars().any(|c| c.is_whitespace()) {
        return Err(ProxyError::InvalidBase {
            base: base.to_string(),
            reason: "must not contain whitespace",
        });
    }
    if let Some(rest) = base.strip_prefix("https://") {
        if rest.is_empty() || !rest.contains('.') {
            return Err(ProxyError::InvalidBase {
                base: base.to_string(),
                reason: "https URL must include a host",
            });
        }
        return Ok(());
    }
    if let Some(rest) = base.strip_prefix("http://") {
        if rest.is_empty() || !rest.contains('.') {
            return Err(ProxyError::InvalidBase {
                base: base.to_string(),
                reason: "http URL must include a host",
            });
        }
        return Ok(());
    }
    if let Some(rest) = base.strip_prefix("file://") {
        if rest.is_empty() {
            return Err(ProxyError::InvalidBase {
                base: base.to_string(),
                reason: "file URL must include a path",
            });
        }
        return Ok(());
    }
    let path = std::path::Path::new(base);
    if path.is_absolute() {
        return Ok(());
    }
    Err(ProxyError::InvalidBase {
        base: base.to_string(),
        reason: "must be https://, http://, file://, or an absolute path",
    })
}

impl ModuleCache {
    /// Clone/fetch using a GOPROXY-shaped proxy list (K11.04).
    ///
    /// Tries each [`ModuleProxy::fetch_plan`] URL. On success, stored `origin`
    /// is the canonical git URL — never the mirror. A missing or failing mirror
    /// tries the next entry; `direct` uses `canonical_git_url`. `off` (empty
    /// plan) fails closed without rewriting identity.
    pub fn clone_or_fetch_with_proxy(
        &self,
        module_path: &str,
        canonical_git_url: &str,
        proxy: &ModuleProxy,
        auth: &GitAuth,
    ) -> Result<std::path::PathBuf, CacheFetchError> {
        let plan = proxy.fetch_plan(module_path, canonical_git_url);
        if plan.is_empty() {
            return Err(CacheFetchError::Proxy(ProxyError::Off));
        }
        let stored = sanitize_stored_git_url(canonical_git_url);
        let mut last_err: Option<CacheFetchError> = None;
        for attempt in plan {
            match self.clone_or_fetch_split(module_path, &attempt.fetch_url, &stored, auth) {
                Ok(dest) => return Ok(dest),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or(CacheFetchError::Proxy(ProxyError::Off)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::content_hash_tree;
    use crate::lock::LockEntry;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    const PATH: &str = "github.com/org/lib";
    const VERSION: &str = "1.0.0";

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "draconic-pkg-k1104-{tag}-{}-{}",
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

    fn fixture_repo(root: &Path) -> PathBuf {
        let repo = root.join("canonical");
        fs::create_dir_all(&repo).unwrap();
        git_ok(&["init"], &repo);
        git_ok(&["config", "user.email", "test@draconic.local"], &repo);
        git_ok(&["config", "user.name", "Draconic Test"], &repo);
        git_ok(&["checkout", "-B", "main"], &repo);
        fs::write(repo.join("hello.txt"), "hello from canonical\n").unwrap();
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

    fn origin_url(vcs: &Path) -> String {
        let out = Command::new("git")
            .args([
                "-C",
                vcs.to_str().unwrap(),
                "config",
                "--get",
                "remote.origin.url",
            ])
            .output()
            .expect("origin url");
        assert!(out.status.success(), "missing origin");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn populate_mirror(mirror_root: &Path, canonical: &Path) -> PathBuf {
        let dest = mirror_root.join(PATH);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        let out = Command::new("git")
            .args([
                "clone",
                "--bare",
                canonical.to_str().unwrap(),
                dest.to_str().unwrap(),
            ])
            .output()
            .expect("mirror clone");
        assert!(
            out.status.success(),
            "mirror clone: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        dest
    }

    #[test]
    fn k11_04_parse_empty_and_direct_are_direct_only() {
        let empty = ModuleProxy::parse("").unwrap();
        assert!(empty.is_direct_only());
        assert_eq!(empty, ModuleProxy::direct());
        let direct = ModuleProxy::parse("direct").unwrap();
        assert!(direct.is_direct_only());
        assert_eq!(direct.entries, vec![ProxyEntry::Direct]);
    }

    #[test]
    fn k11_04_parse_goproxy_shaped_list() {
        let p = ModuleProxy::parse("https://proxy.example.com,direct").unwrap();
        assert_eq!(
            p.entries,
            vec![
                ProxyEntry::Mirror("https://proxy.example.com".into()),
                ProxyEntry::Direct,
            ]
        );
        assert!(!p.is_direct_only());
    }

    #[test]
    fn k11_04_parse_off_stops_list() {
        let p = ModuleProxy::parse("https://proxy.example.com,off,direct").unwrap();
        assert_eq!(
            p.entries,
            vec![
                ProxyEntry::Mirror("https://proxy.example.com".into()),
                ProxyEntry::Off,
                ProxyEntry::Direct,
            ]
        );
        let plan = p.fetch_plan(PATH, "https://github.com/org/lib.git");
        assert_eq!(plan.len(), 1);
        assert_eq!(
            plan[0].fetch_url,
            "https://proxy.example.com/github.com/org/lib"
        );
        assert_eq!(plan[0].canonical_git_url, "https://github.com/org/lib.git");
    }

    #[test]
    fn k11_04_parse_rejects_invalid_base() {
        let err = ModuleProxy::parse("ftp://mirror.example/x").expect_err("ftp");
        match err {
            ProxyError::InvalidBase { base, reason } => {
                assert_eq!(base, "ftp://mirror.example/x");
                assert!(!reason.is_empty());
            }
            other => panic!("expected InvalidBase, got {other:?}"),
        }
        let err = ModuleProxy::parse("relative/mirror").expect_err("relative");
        assert!(matches!(err, ProxyError::InvalidBase { .. }));
        let msg = ModuleProxy::parse("not a url").unwrap_err().to_string();
        assert!(msg.contains("module proxy"), "{msg}");
    }

    #[test]
    fn k11_04_from_vars_unset_is_direct() {
        let p = module_proxy_from_vars(|_| None).unwrap();
        assert!(p.is_direct_only());
        let p = module_proxy_from_vars(|k| {
            if k == PROXY_ENV {
                Some(String::new())
            } else {
                None
            }
        })
        .unwrap();
        assert!(p.is_direct_only());
    }

    #[test]
    fn k11_04_from_vars_reads_draconic_proxy() {
        let p = module_proxy_from_vars(|k| {
            if k == PROXY_ENV {
                Some("https://athens.example,direct".into())
            } else {
                None
            }
        })
        .unwrap();
        assert_eq!(
            p.entries,
            vec![
                ProxyEntry::Mirror("https://athens.example".into()),
                ProxyEntry::Direct,
            ]
        );
    }

    #[test]
    fn k11_04_mirror_fetch_url_joins_base_and_module_path() {
        assert_eq!(
            mirror_fetch_url("https://proxy.example.com", PATH),
            "https://proxy.example.com/github.com/org/lib"
        );
        assert_eq!(
            mirror_fetch_url("https://proxy.example.com/", PATH),
            "https://proxy.example.com/github.com/org/lib"
        );
        assert_eq!(
            mirror_fetch_url("file:///tmp/mirrors", PATH),
            "file:///tmp/mirrors/github.com/org/lib"
        );
        assert_eq!(
            mirror_fetch_url("/var/mirrors", PATH),
            "/var/mirrors/github.com/org/lib"
        );
    }

    #[test]
    fn k11_04_fetch_plan_keeps_canonical_git_url() {
        let canonical = "https://github.com/org/lib.git";
        let p = ModuleProxy::parse("https://mirror.example/mod,direct").unwrap();
        let plan = p.fetch_plan(PATH, canonical);
        assert_eq!(plan.len(), 2);
        assert_eq!(
            plan[0].fetch_url,
            "https://mirror.example/mod/github.com/org/lib"
        );
        assert_eq!(plan[0].canonical_git_url, canonical);
        assert_ne!(plan[0].fetch_url, plan[0].canonical_git_url);
        assert_eq!(plan[1].fetch_url, canonical);
        assert_eq!(plan[1].canonical_git_url, canonical);
    }

    #[test]
    fn k11_04_clone_via_mirror_stores_canonical_origin() {
        let root = temp_dir("mirror-origin");
        let canonical = fixture_repo(&root);
        let canonical_url = canonical.to_str().unwrap().to_string();
        let mirror_root = root.join("mirror");
        populate_mirror(&mirror_root, &canonical);
        let cache = ModuleCache::new(root.join("cache"));
        let proxy = ModuleProxy::parse(mirror_root.to_str().unwrap()).unwrap();
        let mirror_url = mirror_fetch_url(mirror_root.to_str().unwrap(), PATH);

        let vcs = cache
            .clone_or_fetch_with_proxy(PATH, &canonical_url, &proxy, &GitAuth::None)
            .expect("clone via mirror");
        let origin = origin_url(&vcs);
        assert_eq!(origin, canonical_url);
        assert_ne!(origin, mirror_url);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k11_04_clone_via_mirror_lock_identity_is_git_canonical() {
        let root = temp_dir("lock-id");
        let canonical = fixture_repo(&root);
        let canonical_url = canonical.to_str().unwrap().to_string();
        let oid = head_oid(&canonical);
        let mirror_root = root.join("mirror");
        populate_mirror(&mirror_root, &canonical);
        let cache = ModuleCache::new(root.join("cache"));
        let proxy = ModuleProxy::parse(mirror_root.to_str().unwrap()).unwrap();
        let mirror_url = mirror_fetch_url(mirror_root.to_str().unwrap(), PATH);

        cache
            .clone_or_fetch_with_proxy(PATH, &canonical_url, &proxy, &GitAuth::None)
            .expect("clone via mirror");
        let checkout = cache
            .checkout(PATH, &oid, &canonical_url)
            .expect("checkout");
        let hash = content_hash_tree(&checkout).expect("hash");

        let entry = LockEntry::new(PATH, VERSION, &canonical_url, &oid, &hash).expect("lock");
        assert_eq!(entry.path, PATH);
        assert_eq!(entry.git_url, canonical_url);
        assert_eq!(entry.commit_oid, oid);
        assert_eq!(entry.content_hash, hash);
        assert_ne!(entry.git_url, mirror_url);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k11_04_failing_proxy_falls_back_to_direct_without_rewriting_identity() {
        let root = temp_dir("fallback");
        let canonical = fixture_repo(&root);
        let canonical_url = canonical.to_str().unwrap().to_string();
        let missing = root.join("missing-mirror");
        let spec = format!("{},direct", missing.display());
        let proxy = ModuleProxy::parse(&spec).unwrap();
        let cache = ModuleCache::new(root.join("cache"));
        let mirror_url = mirror_fetch_url(missing.to_str().unwrap(), PATH);

        let vcs = cache
            .clone_or_fetch_with_proxy(PATH, &canonical_url, &proxy, &GitAuth::None)
            .expect("direct fallback");
        let origin = origin_url(&vcs);
        assert_eq!(origin, canonical_url);
        assert_ne!(origin, mirror_url);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k11_04_missing_proxy_without_direct_does_not_rewrite_identity() {
        let root = temp_dir("missing");
        let canonical = fixture_repo(&root);
        let canonical_url = canonical.to_str().unwrap().to_string();
        let missing = root.join("no-mirror");
        let proxy = ModuleProxy::parse(missing.to_str().unwrap()).unwrap();
        let cache = ModuleCache::new(root.join("cache"));

        let err = cache
            .clone_or_fetch_with_proxy(PATH, &canonical_url, &proxy, &GitAuth::None)
            .expect_err("missing mirror");
        assert!(
            matches!(err, CacheFetchError::Git(_) | CacheFetchError::Proxy(_)),
            "{err:?}"
        );
        assert!(!cache.has_vcs(PATH).unwrap());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k11_04_off_does_not_fetch() {
        let root = temp_dir("off");
        let canonical = fixture_repo(&root);
        let canonical_url = canonical.to_str().unwrap().to_string();
        let proxy = ModuleProxy::parse("off").unwrap();
        let cache = ModuleCache::new(root.join("cache"));

        let err = cache
            .clone_or_fetch_with_proxy(PATH, &canonical_url, &proxy, &GitAuth::None)
            .expect_err("off");
        match err {
            CacheFetchError::Proxy(e) => {
                assert!(matches!(e, ProxyError::Off), "{e:?}");
                assert!(e.to_string().contains("off"), "{e}");
            }
            other => panic!("expected Proxy off, got {other:?}"),
        }
        assert!(!cache.has_vcs(PATH).unwrap());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k11_04_file_url_mirror_keeps_canonical_identity() {
        let root = temp_dir("file-url");
        let canonical = fixture_repo(&root);
        let canonical_url = format!("file://{}", canonical.display());
        let mirror_root = root.join("mirror");
        populate_mirror(&mirror_root, &canonical);
        let proxy_base = format!("file://{}", mirror_root.display());
        let proxy = ModuleProxy::parse(&proxy_base).unwrap();
        let cache = ModuleCache::new(root.join("cache"));
        let mirror_url = mirror_fetch_url(&proxy_base, PATH);

        let vcs = cache
            .clone_or_fetch_with_proxy(PATH, &canonical_url, &proxy, &GitAuth::None)
            .expect("file:// mirror");
        let origin = origin_url(&vcs);
        assert_eq!(origin, canonical_url);
        assert_ne!(origin, mirror_url);

        let _ = fs::remove_dir_all(&root);
    }
}
