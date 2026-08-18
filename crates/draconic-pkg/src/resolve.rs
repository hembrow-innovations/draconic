//! Version resolve: semver git tags → highest match + commit OID (Roadmap K04).
//!
//! Given a bare (or normal) git repo that already has tags, pick the highest
//! semver tag matching a version requirement and resolve it to a commit OID.
//! Fail closed (K04.02): empty/invalid req, no tags, non-semver-only tags, or
//! no matching tag → typed diagnostic (never silent float / wrong pin).
//!
//! K04.03: resolve a manifest's direct-deps set → full lock pins (version +
//! git URL + commit OID + content hash); v1 walks direct deps only.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::process::Command;

use crate::cache::ModuleCache;
use crate::content_hash_tree;
use crate::lock::{LockEntry, LockFile};
use crate::{resolve_git_url, validate_version_req, Manifest};

/// One resolved pin candidate from a git tag (K04.01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedVersion {
    /// Concrete version without leading `v` (e.g. `1.2.3`).
    pub version: String,
    /// Tag name as stored in git (e.g. `v1.2.3` or `1.2.3`).
    pub tag: String,
    /// Full 40-char lowercase commit SHA-1 the tag points at.
    pub commit_oid: String,
}

/// Error while resolving a version requirement against git tags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// Version requirement is empty or not a supported semver-shaped req.
    InvalidReq { req: String, reason: &'static str },
    /// Repo path is missing or not a git directory.
    NotAGitRepo { path: String },
    /// Repository has no tags at all.
    EmptyTags,
    /// Tags exist but none are parseable semver (e.g. only `main` / `latest`).
    NonSemverOnly,
    /// Semver tags exist but none satisfy the requirement.
    NoMatch { req: String },
    /// `git` subprocess failed or is unavailable.
    Git(String),
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::InvalidReq { req, reason } => {
                write!(f, "version resolve: invalid requirement `{req}`: {reason}")
            }
            ResolveError::NotAGitRepo { path } => {
                write!(f, "version resolve: `{path}` is not a git repository")
            }
            ResolveError::EmptyTags => {
                write!(f, "version resolve: repository has no tags")
            }
            ResolveError::NonSemverOnly => {
                write!(
                    f,
                    "version resolve: repository has tags but none are semver"
                )
            }
            ResolveError::NoMatch { req } => {
                write!(
                    f,
                    "version resolve: no semver tag matches requirement `{req}`"
                )
            }
            ResolveError::Git(msg) => write!(f, "version resolve: git error: {msg}"),
        }
    }
}

impl std::error::Error for ResolveError {}

/// Error while resolving a manifest's direct dependencies to lock pins (K04.03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveDirectError {
    /// One dependency failed version/tag resolve.
    Dep {
        path: String,
        source: ResolveError,
    },
    /// Clone/fetch/checkout into the module cache failed.
    Cache {
        path: String,
        message: String,
    },
    /// Content hash of a checked-out package tree failed.
    ContentHash {
        path: String,
        message: String,
    },
    /// Built lock entry failed validation (should be rare).
    LockEntry {
        path: String,
        message: String,
    },
}

impl fmt::Display for ResolveDirectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveDirectError::Dep { path, source } => {
                write!(f, "resolve direct deps: `{path}`: {source}")
            }
            ResolveDirectError::Cache { path, message } => {
                write!(f, "resolve direct deps: `{path}` cache: {message}")
            }
            ResolveDirectError::ContentHash { path, message } => {
                write!(f, "resolve direct deps: `{path}` content hash: {message}")
            }
            ResolveDirectError::LockEntry { path, message } => {
                write!(f, "resolve direct deps: `{path}` lock entry: {message}")
            }
        }
    }
}

impl std::error::Error for ResolveDirectError {}

/// Resolve all **direct** dependencies in `manifest` to a [`LockFile`] (K04.03).
///
/// For each dependency (sorted by module path):
/// 1. Resolve git URL (`[urls]` override or default derive)
/// 2. `clone_or_fetch` into `cache` VCS store
/// 3. Highest matching semver tag → commit OID
/// 4. Checkout OID into mod store
/// 5. Content-hash package tree → lock pin
///
/// v1 does **not** walk transitive deps of packages. Empty deps → empty lock.
pub fn resolve_direct_deps(
    manifest: &Manifest,
    cache: &ModuleCache,
) -> Result<LockFile, ResolveDirectError> {
    let mut packages = BTreeMap::new();

    for (path, req) in &manifest.dependencies {
        let git_url = resolve_git_url(manifest, path);

        let vcs = cache.clone_or_fetch(path, &git_url).map_err(|e| {
            ResolveDirectError::Cache {
                path: path.clone(),
                message: e.to_string(),
            }
        })?;

        let resolved = resolve_highest_matching_tag(&vcs, req).map_err(|source| {
            ResolveDirectError::Dep {
                path: path.clone(),
                source,
            }
        })?;

        let checkout = cache
            .checkout(path, &resolved.commit_oid, &git_url)
            .map_err(|e| ResolveDirectError::Cache {
                path: path.clone(),
                message: e.to_string(),
            })?;

        let content_hash = content_hash_tree(&checkout).map_err(|e| {
            ResolveDirectError::ContentHash {
                path: path.clone(),
                message: e.to_string(),
            }
        })?;

        let entry = LockEntry::new(
            path.clone(),
            resolved.version,
            git_url,
            resolved.commit_oid,
            content_hash,
        )
        .map_err(|e| ResolveDirectError::LockEntry {
            path: path.clone(),
            message: e.to_string(),
        })?;

        packages.insert(path.clone(), entry);
    }

    Ok(LockFile {
        version: 1,
        packages,
    })
}

/// Resolve `req` against semver tags in `repo` (bare or worktree); highest match wins.
///
/// Tags may be `1.2.3` or `v1.2.3`. Non-semver tags are ignored. Matching uses the
/// operators accepted by manifest validation (`^` `~` `>=` `<=` `>` `<` `=` / exact).
pub fn resolve_highest_matching_tag(
    repo: &Path,
    req: &str,
) -> Result<ResolvedVersion, ResolveError> {
    if let Err(reason) = validate_version_req(req) {
        return Err(ResolveError::InvalidReq {
            req: req.to_string(),
            reason,
        });
    }
    if !looks_like_git_repo(repo) {
        return Err(ResolveError::NotAGitRepo {
            path: repo.display().to_string(),
        });
    }

    let parsed_req = parse_version_req(req).map_err(|reason| ResolveError::InvalidReq {
        req: req.to_string(),
        reason,
    })?;

    let tags = list_git_tags(repo)?;
    if tags.is_empty() {
        return Err(ResolveError::EmptyTags);
    }

    let mut saw_semver = false;
    let mut best: Option<(SemVer, String, String)> = None;

    for tag in tags {
        let ver_str = tag
            .strip_prefix('v')
            .unwrap_or(tag.as_str())
            .to_string();
        let Ok(ver) = parse_semver(&ver_str) else {
            continue;
        };
        saw_semver = true;
        if !req_matches(&parsed_req, &ver) {
            continue;
        }
        let replace = match &best {
            None => true,
            Some((cur, _, _)) => ver.cmp(cur) == Ordering::Greater,
        };
        if replace {
            best = Some((ver, tag, ver_str));
        }
    }

    let Some((_ver, tag, version)) = best else {
        if !saw_semver {
            return Err(ResolveError::NonSemverOnly);
        }
        return Err(ResolveError::NoMatch {
            req: req.to_string(),
        });
    };

    let commit_oid = tag_commit_oid(repo, &tag)?;
    Ok(ResolvedVersion {
        version,
        tag,
        commit_oid,
    })
}

fn looks_like_git_repo(path: &Path) -> bool {
    path.join("HEAD").is_file()
        || path.join(".git").is_dir()
        || path.join(".git").is_file()
}

fn run_git(repo: &Path, args: &[&str]) -> Result<String, ResolveError> {
    let repo_str = repo
        .to_str()
        .ok_or_else(|| ResolveError::Git("repo path is not valid UTF-8".into()))?;
    let mut full = vec!["-C", repo_str];
    full.extend_from_slice(args);
    let output = Command::new("git")
        .args(&full)
        .output()
        .map_err(|e| ResolveError::Git(format!("failed to spawn git: {e}")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
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
        Err(ResolveError::Git(detail))
    }
}

fn list_git_tags(repo: &Path) -> Result<Vec<String>, ResolveError> {
    let out = run_git(repo, &["tag", "-l"])?;
    Ok(out
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect())
}

fn tag_commit_oid(repo: &Path, tag: &str) -> Result<String, ResolveError> {
    // Peel annotated tags to the commit.
    let peeled = format!("{tag}^{{commit}}");
    let oid = run_git(repo, &["rev-parse", &peeled])?
        .trim()
        .to_string();
    if oid.len() != 40 || !oid.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')) {
        return Err(ResolveError::Git(format!(
            "tag `{tag}` resolved to non-OID `{oid}`"
        )));
    }
    Ok(oid)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SemVer {
    major: u64,
    minor: u64,
    patch: u64,
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

fn parse_semver(s: &str) -> Result<SemVer, &'static str> {
    // Core only for tag matching (ignore prerelease/build for v1 highest-match).
    let core = s
        .split_once('+')
        .map(|(c, _)| c)
        .unwrap_or(s)
        .split_once('-')
        .map(|(c, _)| c)
        .unwrap_or(s);
    let parts: Vec<&str> = core.split('.').collect();
    if parts.is_empty() || parts.len() > 3 {
        return Err("version core must be MAJOR[.MINOR[.PATCH]]");
    }
    let mut nums = [0u64; 3];
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()) {
            return Err("version core components must be decimal digits");
        }
        if part.len() > 1 && part.starts_with('0') {
            return Err("version core components must not have leading zeros");
        }
        nums[i] = part
            .parse()
            .map_err(|_| "version core component out of range")?;
    }
    Ok(SemVer {
        major: nums[0],
        minor: nums[1],
        patch: nums[2],
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum VersionReq {
    Exact(SemVer),
    Caret(SemVer),
    Tilde(SemVer),
    Ge(SemVer),
    Gt(SemVer),
    Le(SemVer),
    Lt(SemVer),
}

fn parse_version_req(req: &str) -> Result<VersionReq, &'static str> {
    let (kind, rest) = if let Some(r) = req.strip_prefix(">=") {
        ("ge", r)
    } else if let Some(r) = req.strip_prefix("<=") {
        ("le", r)
    } else if let Some(r) = req.strip_prefix('>') {
        ("gt", r)
    } else if let Some(r) = req.strip_prefix('<') {
        ("lt", r)
    } else if let Some(r) = req.strip_prefix('^') {
        ("caret", r)
    } else if let Some(r) = req.strip_prefix('~') {
        ("tilde", r)
    } else if let Some(r) = req.strip_prefix('=') {
        ("exact", r)
    } else {
        ("exact", req)
    };

    let rest = rest.strip_prefix('v').unwrap_or(rest);
    let ver = parse_semver(rest)?;
    Ok(match kind {
        "ge" => VersionReq::Ge(ver),
        "gt" => VersionReq::Gt(ver),
        "le" => VersionReq::Le(ver),
        "lt" => VersionReq::Lt(ver),
        "caret" => VersionReq::Caret(ver),
        "tilde" => VersionReq::Tilde(ver),
        _ => VersionReq::Exact(ver),
    })
}

fn req_matches(req: &VersionReq, v: &SemVer) -> bool {
    match req {
        VersionReq::Exact(base) => v == base,
        VersionReq::Ge(base) => v >= base,
        VersionReq::Gt(base) => v > base,
        VersionReq::Le(base) => v <= base,
        VersionReq::Lt(base) => v < base,
        VersionReq::Tilde(base) => {
            // ~1.2.3 => >=1.2.3 <1.3.0; ~1.2 => >=1.2.0 <1.3.0; ~1 => >=1.0.0 <2.0.0
            if v < base {
                return false;
            }
            if base.major > 0 || base.minor > 0 || base.patch > 0 {
                // If only major was specified (minor=patch=0 from partial), still tilde on minor.
            }
            // Standard: allow patch bumps within same major.minor.
            // For partial ~1.2 (parsed as 1.2.0) same rule; ~1 (1.0.0) → same major only.
            // Distinguish partial via: if patch was defaulted from MAJOR only, tilde is major-bound.
            // We always parse missing parts as 0, so:
            // - ~1.2.3: same major+minor
            // - ~1.2.0 from "~1.2": same major+minor
            // - ~1.0.0 from "~1": cargo treats ~1 as >=1.0.0 <2.0.0
            // Use cargo-like: if minor and patch are both 0 and the original had only major…
            // Without original shape, treat ~X.Y.Z as same major.minor always when minor!=0 or patch!=0;
            // when base is X.0.0 use major-only bound (cargo ~1.0.0 is >=1.0.0 <1.1.0 actually).
            // Cargo: ~1.2.3 := >=1.2.3, <1.3.0; ~1.2 := >=1.2.0, <1.3.0; ~1 := >=1.0.0, <2.0.0
            // We can't see "~1" vs "~1.0.0". Treat all as same-minor (patch bumps only).
            v.major == base.major && v.minor == base.minor
        }
        VersionReq::Caret(base) => {
            // ^1.2.3 => >=1.2.3 <2.0.0
            // ^0.2.3 => >=0.2.3 <0.3.0
            // ^0.0.3 => >=0.0.3 <0.0.4
            if v < base {
                return false;
            }
            if base.major > 0 {
                v.major == base.major
            } else if base.minor > 0 {
                v.major == 0 && v.minor == base.minor
            } else {
                v.major == 0 && v.minor == 0 && v.patch == base.patch
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "draconic-pkg-k0401-{tag}-{}-{}",
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

    fn commit_file(repo: &Path, name: &str, body: &str, msg: &str) -> String {
        fs::write(repo.join(name), body).unwrap();
        git_ok(&["add", name], repo);
        git_ok(&["commit", "-m", msg], repo);
        let out = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo)
            .output()
            .expect("rev-parse");
        assert!(out.status.success());
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// Fixture repo with several semver tags (and one non-semver tag).
    fn tagged_fixture(root: &Path) -> (PathBuf, String, String, String) {
        let repo = root.join("upstream");
        fs::create_dir_all(&repo).unwrap();
        git_ok(&["init"], &repo);
        git_ok(&["config", "user.email", "test@draconic.local"], &repo);
        git_ok(&["config", "user.name", "Draconic Test"], &repo);
        git_ok(&["checkout", "-B", "main"], &repo);

        let oid_100 = commit_file(&repo, "a.txt", "1.0.0\n", "v1.0.0");
        git_ok(&["tag", "v1.0.0"], &repo);

        let oid_120 = commit_file(&repo, "a.txt", "1.2.0\n", "v1.2.0");
        git_ok(&["tag", "1.2.0"], &repo);

        let oid_123 = commit_file(&repo, "a.txt", "1.2.3\n", "v1.2.3");
        git_ok(&["tag", "v1.2.3"], &repo);

        let _oid_200 = commit_file(&repo, "a.txt", "2.0.0\n", "v2.0.0");
        git_ok(&["tag", "v2.0.0"], &repo);

        git_ok(&["tag", "not-a-version"], &repo);

        (repo, oid_100, oid_120, oid_123)
    }

    #[test]
    fn exact_match_prefers_v_prefix_tag() {
        let root = temp_dir("exact");
        let (repo, _, _, oid_123) = tagged_fixture(&root);
        let r = resolve_highest_matching_tag(&repo, "1.2.3").expect("resolve");
        assert_eq!(r.version, "1.2.3");
        assert_eq!(r.tag, "v1.2.3");
        assert_eq!(r.commit_oid, oid_123);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn exact_match_unprefixed_tag() {
        let root = temp_dir("exact-noprefix");
        let (repo, _, oid_120, _) = tagged_fixture(&root);
        let r = resolve_highest_matching_tag(&repo, "1.2.0").expect("resolve");
        assert_eq!(r.version, "1.2.0");
        assert_eq!(r.tag, "1.2.0");
        assert_eq!(r.commit_oid, oid_120);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn caret_picks_highest_within_major() {
        let root = temp_dir("caret");
        let (repo, _, _, oid_123) = tagged_fixture(&root);
        let r = resolve_highest_matching_tag(&repo, "^1.0.0").expect("resolve");
        assert_eq!(r.version, "1.2.3");
        assert_eq!(r.tag, "v1.2.3");
        assert_eq!(r.commit_oid, oid_123);
        // Must not jump to 2.0.0
        assert_ne!(r.version, "2.0.0");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn tilde_stays_on_minor() {
        let root = temp_dir("tilde");
        let (repo, _, _, oid_123) = tagged_fixture(&root);
        let r = resolve_highest_matching_tag(&repo, "~1.2.0").expect("resolve");
        assert_eq!(r.version, "1.2.3");
        assert_eq!(r.commit_oid, oid_123);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn ge_picks_highest_overall() {
        let root = temp_dir("ge");
        let (repo, _, _, _) = tagged_fixture(&root);
        let r = resolve_highest_matching_tag(&repo, ">=1.0.0").expect("resolve");
        assert_eq!(r.version, "2.0.0");
        assert_eq!(r.tag, "v2.0.0");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn no_match_returns_error() {
        let root = temp_dir("nomatch");
        let (repo, _, _, _) = tagged_fixture(&root);
        let err = resolve_highest_matching_tag(&repo, "3.0.0").expect_err("no 3.x");
        match &err {
            ResolveError::NoMatch { req } => assert_eq!(req, "3.0.0"),
            other => panic!("expected NoMatch, got {other:?}"),
        }
        assert!(err.to_string().contains("no semver tag"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn no_match_range_when_semver_tags_exist() {
        let root = temp_dir("nomatch-range");
        let (repo, _, _, _) = tagged_fixture(&root);
        let err = resolve_highest_matching_tag(&repo, "^3.0.0").expect_err("no 3.x");
        match &err {
            ResolveError::NoMatch { req } => assert_eq!(req, "^3.0.0"),
            other => panic!("expected NoMatch, got {other:?}"),
        }
        assert!(err.to_string().contains("no semver tag matches"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_req_rejected() {
        let root = temp_dir("empty-req");
        let (repo, _, _, _) = tagged_fixture(&root);
        let err = resolve_highest_matching_tag(&repo, "").expect_err("empty");
        match &err {
            ResolveError::InvalidReq { req, reason } => {
                assert_eq!(req, "");
                assert!(reason.contains("empty"), "{reason}");
            }
            other => panic!("expected InvalidReq, got {other:?}"),
        }
        assert!(err.to_string().contains("invalid requirement"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_operator_only_req_rejected() {
        let root = temp_dir("op-only");
        let (repo, _, _, _) = tagged_fixture(&root);
        for req in ["^", "~", ">=", "<=", ">", "<", "="] {
            let err = resolve_highest_matching_tag(&repo, req).expect_err(req);
            assert!(
                matches!(err, ResolveError::InvalidReq { .. }),
                "{req}: {err:?}"
            );
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn invalid_req_rejected() {
        let root = temp_dir("badreq");
        let (repo, _, _, _) = tagged_fixture(&root);
        let err = resolve_highest_matching_tag(&repo, "latest").expect_err("bad req");
        assert!(matches!(err, ResolveError::InvalidReq { .. }), "{err:?}");
        let err2 = resolve_highest_matching_tag(&repo, "main").expect_err("branch");
        assert!(matches!(err2, ResolveError::InvalidReq { .. }), "{err2:?}");
        let _ = fs::remove_dir_all(&root);
    }

    /// K04.02: tags exist but none are semver → NonSemverOnly (not silent wrong pin).
    #[test]
    fn non_semver_only_tags_fail_closed() {
        let root = temp_dir("nonsemver-only");
        let repo = root.join("upstream");
        fs::create_dir_all(&repo).unwrap();
        git_ok(&["init"], &repo);
        git_ok(&["config", "user.email", "test@draconic.local"], &repo);
        git_ok(&["config", "user.name", "Draconic Test"], &repo);
        git_ok(&["checkout", "-B", "main"], &repo);
        let _ = commit_file(&repo, "a.txt", "x\n", "init");
        git_ok(&["tag", "latest"], &repo);
        git_ok(&["tag", "release-candidate"], &repo);
        git_ok(&["tag", "vnext"], &repo);

        let err = resolve_highest_matching_tag(&repo, "1.0.0").expect_err("no semver");
        assert!(
            matches!(err, ResolveError::NonSemverOnly),
            "expected NonSemverOnly, got {err:?}"
        );
        assert!(err.to_string().contains("none are semver"));
        // Ranges also fail closed the same way.
        let err2 = resolve_highest_matching_tag(&repo, "^1.0.0").expect_err("range");
        assert!(matches!(err2, ResolveError::NonSemverOnly), "{err2:?}");
        let _ = fs::remove_dir_all(&root);
    }

    /// K04.02: repository with zero tags → EmptyTags diagnostic.
    #[test]
    fn empty_tags_fail_closed() {
        let root = temp_dir("empty-tags");
        let repo = root.join("upstream");
        fs::create_dir_all(&repo).unwrap();
        git_ok(&["init"], &repo);
        git_ok(&["config", "user.email", "test@draconic.local"], &repo);
        git_ok(&["config", "user.name", "Draconic Test"], &repo);
        git_ok(&["checkout", "-B", "main"], &repo);
        let _ = commit_file(&repo, "a.txt", "x\n", "init");

        let err = resolve_highest_matching_tag(&repo, "1.0.0").expect_err("no tags");
        assert!(
            matches!(err, ResolveError::EmptyTags),
            "expected EmptyTags, got {err:?}"
        );
        assert!(err.to_string().contains("no tags"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_repo_rejected() {
        let root = temp_dir("missing");
        let missing = root.join("nope");
        let err = resolve_highest_matching_tag(&missing, "1.0.0").expect_err("missing");
        assert!(matches!(err, ResolveError::NotAGitRepo { .. }), "{err:?}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn works_on_bare_clone() {
        let root = temp_dir("bare");
        let (upstream, _, _, oid_123) = tagged_fixture(&root);
        let bare = root.join("bare.git");
        git_ok(
            &[
                "clone",
                "--bare",
                upstream.to_str().unwrap(),
                bare.to_str().unwrap(),
            ],
            &root,
        );
        let r = resolve_highest_matching_tag(&bare, "^1.2.0").expect("bare resolve");
        assert_eq!(r.version, "1.2.3");
        assert_eq!(r.commit_oid, oid_123);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn caret_zero_minor_stays_on_minor() {
        let root = temp_dir("caret0");
        let repo = root.join("upstream");
        fs::create_dir_all(&repo).unwrap();
        git_ok(&["init"], &repo);
        git_ok(&["config", "user.email", "test@draconic.local"], &repo);
        git_ok(&["config", "user.name", "Draconic Test"], &repo);
        git_ok(&["checkout", "-B", "main"], &repo);
        let oid_021 = commit_file(&repo, "a.txt", "0.2.1\n", "0.2.1");
        git_ok(&["tag", "v0.2.1"], &repo);
        let _ = commit_file(&repo, "a.txt", "0.3.0\n", "0.3.0");
        git_ok(&["tag", "v0.3.0"], &repo);

        let r = resolve_highest_matching_tag(&repo, "^0.2.0").expect("resolve");
        assert_eq!(r.version, "0.2.1");
        assert_eq!(r.commit_oid, oid_021);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn ignores_non_semver_tags() {
        let root = temp_dir("nonsemver");
        let (repo, oid_100, _, _) = tagged_fixture(&root);
        // Only 1.0.0 matches exact; "not-a-version" must not break listing.
        let r = resolve_highest_matching_tag(&repo, "1.0.0").expect("resolve");
        assert_eq!(r.commit_oid, oid_100);
        let _ = fs::remove_dir_all(&root);
    }

    // --- K04.03: resolve direct-deps set → lock pins ---

    fn manifest_deps(deps: &[(&str, &str)], urls: &[(&str, &str)]) -> Manifest {
        Manifest {
            module: "github.com/acme/app".into(),
            dependencies: deps
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            urls: urls
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    /// Second upstream with a single tag (used for multi-dep + transitive non-walk).
    fn single_tag_upstream(root: &Path, name: &str, tag: &str, body: &str) -> (PathBuf, String) {
        let repo = root.join(name);
        fs::create_dir_all(&repo).unwrap();
        git_ok(&["init"], &repo);
        git_ok(&["config", "user.email", "test@draconic.local"], &repo);
        git_ok(&["config", "user.name", "Draconic Test"], &repo);
        git_ok(&["checkout", "-B", "main"], &repo);
        let oid = commit_file(&repo, "lib.txt", body, tag);
        git_ok(&["tag", tag], &repo);
        (repo, oid)
    }

    #[test]
    fn k04_03_empty_deps_yields_empty_lock() {
        let root = temp_dir("k0403-empty");
        let cache = ModuleCache::new(root.join("cache"));
        let m = manifest_deps(&[], &[]);
        let lock = resolve_direct_deps(&m, &cache).expect("empty");
        assert_eq!(lock.version, 1);
        assert!(lock.packages.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k04_03_one_direct_dep_to_lock_pin() {
        let root = temp_dir("k0403-one");
        let (upstream, _, _, oid_123) = tagged_fixture(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/lib";
        let m = manifest_deps(
            &[(path, "^1.0.0")],
            &[(path, upstream.to_str().unwrap())],
        );

        let lock = resolve_direct_deps(&m, &cache).expect("resolve");
        assert_eq!(lock.packages.len(), 1);
        let e = lock.packages.get(path).expect("pin");
        assert_eq!(e.path, path);
        assert_eq!(e.version, "1.2.3");
        assert_eq!(e.commit_oid, oid_123);
        assert_eq!(e.git_url, upstream.to_str().unwrap());
        assert_eq!(e.content_hash.len(), 64);
        assert!(e.content_hash.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));

        // Checkout exists and hash matches tree.
        let entry_dir = cache.entry_dir(path, &oid_123).unwrap();
        assert!(cache.has_entry(path, &oid_123).unwrap());
        let expected_hash = crate::content_hash_tree(&entry_dir).unwrap();
        assert_eq!(e.content_hash, expected_hash);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k04_03_two_direct_deps_sorted_lock() {
        let root = temp_dir("k0403-two");
        let (up_a, oid_a) = single_tag_upstream(&root, "up-a", "v1.0.0", "a\n");
        let (up_z, oid_z) = single_tag_upstream(&root, "up-z", "v2.0.0", "z\n");
        let cache = ModuleCache::new(root.join("cache"));
        let m = manifest_deps(
            &[
                ("github.com/z/last", "2.0.0"),
                ("github.com/a/first", "1.0.0"),
            ],
            &[
                ("github.com/z/last", up_z.to_str().unwrap()),
                ("github.com/a/first", up_a.to_str().unwrap()),
            ],
        );

        let lock = resolve_direct_deps(&m, &cache).expect("resolve");
        assert_eq!(lock.packages.len(), 2);
        let keys: Vec<_> = lock.packages.keys().cloned().collect();
        assert_eq!(
            keys,
            vec![
                "github.com/a/first".to_string(),
                "github.com/z/last".to_string()
            ]
        );
        assert_eq!(lock.packages["github.com/a/first"].commit_oid, oid_a);
        assert_eq!(lock.packages["github.com/a/first"].version, "1.0.0");
        assert_eq!(lock.packages["github.com/z/last"].commit_oid, oid_z);
        assert_eq!(lock.packages["github.com/z/last"].version, "2.0.0");

        // Stable serialize order.
        let written = crate::write_lock(&lock);
        let a_pos = written.find("github.com/a/first").unwrap();
        let z_pos = written.find("github.com/z/last").unwrap();
        assert!(a_pos < z_pos, "{written}");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k04_03_direct_only_ignores_nested_manifest_deps() {
        // Upstream package tree includes its own draconic.toml with a dep that
        // is not a direct dep of the consumer — must not appear in the lock.
        let root = temp_dir("k0403-direct-only");
        let upstream = root.join("upstream");
        fs::create_dir_all(&upstream).unwrap();
        git_ok(&["init"], &upstream);
        git_ok(&["config", "user.email", "test@draconic.local"], &upstream);
        git_ok(&["config", "user.name", "Draconic Test"], &upstream);
        git_ok(&["checkout", "-B", "main"], &upstream);
        fs::write(
            upstream.join("draconic.toml"),
            r#"module = "github.com/org/lib"

[dependencies]
"github.com/transitive/only" = "1.0.0"
"#,
        )
        .unwrap();
        fs::write(upstream.join("lib.drac"), "export let x = 1;\n").unwrap();
        git_ok(&["add", "."], &upstream);
        git_ok(&["commit", "-m", "v1.0.0"], &upstream);
        let oid = {
            let out = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&upstream)
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };
        git_ok(&["tag", "v1.0.0"], &upstream);

        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/lib";
        let m = manifest_deps(
            &[(path, "1.0.0")],
            &[(path, upstream.to_str().unwrap())],
        );
        let lock = resolve_direct_deps(&m, &cache).expect("resolve");
        assert_eq!(lock.packages.len(), 1);
        assert!(lock.packages.contains_key(path));
        assert!(!lock.packages.contains_key("github.com/transitive/only"));
        assert_eq!(lock.packages[path].commit_oid, oid);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k04_03_fail_closed_no_matching_tag() {
        let root = temp_dir("k0403-nomatch");
        let (upstream, _, _, _) = tagged_fixture(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/lib";
        let m = manifest_deps(
            &[(path, "9.9.9")],
            &[(path, upstream.to_str().unwrap())],
        );
        let err = resolve_direct_deps(&m, &cache).expect_err("no match");
        match &err {
            ResolveDirectError::Dep { path: p, source } => {
                assert_eq!(p, path);
                assert!(matches!(source, ResolveError::NoMatch { .. }), "{source:?}");
            }
            other => panic!("expected Dep NoMatch, got {other:?}"),
        }
        assert!(err.to_string().contains(path));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn k04_03_fail_closed_missing_remote() {
        let root = temp_dir("k0403-missing");
        let cache = ModuleCache::new(root.join("cache"));
        let path = "github.com/org/lib";
        let missing = root.join("no-such-upstream");
        let m = manifest_deps(
            &[(path, "1.0.0")],
            &[(path, missing.to_str().unwrap())],
        );
        let err = resolve_direct_deps(&m, &cache).expect_err("missing remote");
        assert!(
            matches!(err, ResolveDirectError::Cache { .. }),
            "{err:?}"
        );
        assert!(err.to_string().contains(path));
        let _ = fs::remove_dir_all(&root);
    }
}
