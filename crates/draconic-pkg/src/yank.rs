//! K11.05: yank/retract when an advisory source is configured.
//!
//! An advisory source lists yanked or retracted (module path, version) pairs.
//! When configured, resolve/fetch hard-fails those versions and does not pin
//! them. With no advisory source, yank is not a silent v1 check — resolve
//! proceeds as today.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Environment variable naming an advisory file (`file://` or filesystem path).
pub const ADVISORY_ENV: &str = "DRACONIC_ADVISORY";

/// Why a configured advisory refuses a version (K11.05).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum YankKind {
    /// Version was yanked (must not be newly pinned).
    Yank,
    /// Version was retracted (must not be newly pinned).
    Retract,
}

impl YankKind {
    fn as_str(self) -> &'static str {
        match self {
            YankKind::Yank => "yanked",
            YankKind::Retract => "retracted",
        }
    }
}

/// Configured advisory list of yanked/retracted versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvisorySource {
    /// Origin path (for diagnostics); empty when parsed from a string.
    pub origin: String,
    entries: BTreeSet<(String, String, YankKind)>,
}

/// Error while loading or applying an advisory source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdvisoryError {
    /// Advisory path is empty or not a readable file.
    Missing { path: String },
    /// Advisory text is malformed.
    Parse { origin: String, detail: String },
    /// Resolved version is yanked or retracted; do not pin.
    Refused {
        path: String,
        version: String,
        kind: YankKind,
    },
}

impl fmt::Display for AdvisoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdvisoryError::Missing { path } => {
                write!(f, "advisory source: missing or unreadable `{path}`")
            }
            AdvisoryError::Parse { origin, detail } => {
                write!(f, "advisory source `{origin}`: {detail}")
            }
            AdvisoryError::Refused {
                path,
                version,
                kind,
            } => write!(
                f,
                "advisory source: `{path}`@{version} is {} and will not be pinned",
                kind.as_str()
            ),
        }
    }
}

impl std::error::Error for AdvisoryError {}

impl AdvisorySource {
    /// Parse advisory text. Empty / comments-only → no refusals.
    pub fn parse(origin: &str, src: &str) -> Result<Self, AdvisoryError> {
        let mut entries = BTreeSet::new();
        for (i, raw) in src.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split_whitespace();
            let kind_tok = parts.next().unwrap_or("");
            let path = parts.next();
            let version = parts.next();
            let extra = parts.next();
            let kind = match kind_tok {
                "yank" => YankKind::Yank,
                "retract" => YankKind::Retract,
                other => {
                    return Err(AdvisoryError::Parse {
                        origin: origin.to_string(),
                        detail: format!(
                            "line {}: unknown kind `{other}` (expected yank or retract)",
                            i + 1
                        ),
                    });
                }
            };
            let (Some(path), Some(version)) = (path, version) else {
                return Err(AdvisoryError::Parse {
                    origin: origin.to_string(),
                    detail: format!(
                        "line {}: expected `{kind_tok} <module_path> <version>`",
                        i + 1
                    ),
                });
            };
            if extra.is_some() {
                return Err(AdvisoryError::Parse {
                    origin: origin.to_string(),
                    detail: format!("line {}: unexpected extra tokens", i + 1),
                });
            }
            if crate::validate_module_path(path).is_err() {
                return Err(AdvisoryError::Parse {
                    origin: origin.to_string(),
                    detail: format!("line {}: invalid module path `{path}`", i + 1),
                });
            }
            if crate::validate_version_req(version).is_err() {
                return Err(AdvisoryError::Parse {
                    origin: origin.to_string(),
                    detail: format!("line {}: invalid version `{version}`", i + 1),
                });
            }
            entries.insert((path.to_string(), normalize_version(version), kind));
        }
        Ok(Self {
            origin: origin.to_string(),
            entries,
        })
    }

    /// Load an advisory file from a filesystem path or `file://` URL.
    pub fn load_path(path: impl AsRef<Path>) -> Result<Self, AdvisoryError> {
        let path = path.as_ref();
        let origin = path.display().to_string();
        let src = fs::read_to_string(path).map_err(|_| AdvisoryError::Missing {
            path: origin.clone(),
        })?;
        Self::parse(&origin, &src)
    }

    /// Resolve from process env (`DRACONIC_ADVISORY`). Unset/empty → `Ok(None)`.
    pub fn from_env() -> Result<Option<Self>, AdvisoryError> {
        advisory_from_vars(|k| std::env::var(k).ok())
    }

    /// Hard-fail when `(module_path, version)` is yanked or retracted.
    pub fn refuse(&self, module_path: &str, version: &str) -> Result<(), AdvisoryError> {
        let version = normalize_version(version);
        for kind in [YankKind::Yank, YankKind::Retract] {
            if self
                .entries
                .contains(&(module_path.to_string(), version.clone(), kind))
            {
                return Err(AdvisoryError::Refused {
                    path: module_path.to_string(),
                    version,
                    kind,
                });
            }
        }
        Ok(())
    }
}

/// Resolve [`AdvisorySource`] from a key/value lookup (env in production; map in tests).
///
/// Unset or blank `DRACONIC_ADVISORY` → `Ok(None)` (no yank check).
pub fn advisory_from_vars<F>(mut get: F) -> Result<Option<AdvisorySource>, AdvisoryError>
where
    F: FnMut(&str) -> Option<String>,
{
    match get(ADVISORY_ENV) {
        Some(spec) if !spec.trim().is_empty() => {
            Ok(Some(AdvisorySource::load_path(advisory_path(&spec)?)?))
        }
        _ => Ok(None),
    }
}

fn advisory_path(spec: &str) -> Result<PathBuf, AdvisoryError> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err(AdvisoryError::Missing {
            path: spec.to_string(),
        });
    }
    if let Some(rest) = spec.strip_prefix("file://") {
        if rest.is_empty() {
            return Err(AdvisoryError::Missing {
                path: spec.to_string(),
            });
        }
        return Ok(PathBuf::from(rest));
    }
    Ok(PathBuf::from(spec))
}

fn normalize_version(version: &str) -> String {
    version.strip_prefix('v').unwrap_or(version).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        parse_manifest, resolve_direct_deps, resolve_direct_deps_with_advisory, Manifest,
        ModuleCache, ResolveDirectError,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    const PATH: &str = "github.com/org/lib";

    fn parse_ok(src: &str) -> AdvisorySource {
        AdvisorySource::parse("memory", src).unwrap_or_else(|e| panic!("parse: {e}"))
    }

    #[test]
    fn yank_parse_yank_and_retract_lines() {
        let src = parse_ok(
            r#"
# comment
yank github.com/org/lib 1.2.3
retract github.com/org/lib 1.0.0
"#,
        );
        src.refuse(PATH, "1.2.3").expect_err("yanked");
        src.refuse(PATH, "1.0.0").expect_err("retracted");
        src.refuse(PATH, "1.2.4").expect("not listed");
    }

    #[test]
    fn yank_parse_empty_is_no_refusals() {
        let src = parse_ok("\n# only comments\n");
        src.refuse(PATH, "1.0.0").expect("empty advisory");
    }

    #[test]
    fn yank_refuse_strips_v_prefix() {
        let src = parse_ok("yank github.com/org/lib v1.2.3\n");
        let err = src.refuse(PATH, "1.2.3").expect_err("v-prefix yank");
        match &err {
            AdvisoryError::Refused {
                path,
                version,
                kind,
            } => {
                assert_eq!(path, PATH);
                assert_eq!(version, "1.2.3");
                assert_eq!(*kind, YankKind::Yank);
            }
            other => panic!("expected Refused, got {other:?}"),
        }
        assert!(err.to_string().contains("yanked"), "{err}");
        assert!(err.to_string().contains(PATH), "{err}");
    }

    #[test]
    fn yank_refuse_retract_diagnostic() {
        let src = parse_ok("retract github.com/org/lib 1.0.0\n");
        let err = src.refuse(PATH, "v1.0.0").expect_err("retract");
        match &err {
            AdvisoryError::Refused { kind, .. } => assert_eq!(*kind, YankKind::Retract),
            other => panic!("expected Refused, got {other:?}"),
        }
        assert!(err.to_string().contains("retracted"), "{err}");
        assert!(err.to_string().contains("will not be pinned"), "{err}");
    }

    #[test]
    fn yank_parse_rejects_unknown_kind() {
        let err = AdvisorySource::parse("memory", "ban github.com/org/lib 1.0.0\n")
            .expect_err("unknown kind");
        match err {
            AdvisoryError::Parse { detail, .. } => {
                assert!(detail.contains("unknown kind"), "{detail}");
            }
            other => panic!("expected Parse, got {other:?}"),
        }
    }

    #[test]
    fn yank_parse_rejects_invalid_module_path() {
        let err = AdvisorySource::parse("memory", "yank not-a-path 1.0.0\n").expect_err("path");
        assert!(matches!(err, AdvisoryError::Parse { .. }), "{err:?}");
        assert!(err.to_string().contains("advisory source"), "{err}");
    }

    #[test]
    fn yank_from_vars_unset_is_none() {
        let got = advisory_from_vars(|_| None).expect("unset");
        assert!(got.is_none(), "no advisory → no yank check");
        let got = advisory_from_vars(|k| {
            if k == ADVISORY_ENV {
                Some(String::new())
            } else {
                None
            }
        })
        .expect("empty");
        assert!(got.is_none());
    }

    #[test]
    fn yank_from_vars_missing_file_fails_closed() {
        let err = advisory_from_vars(|k| {
            if k == ADVISORY_ENV {
                Some("/no/such/draconic-advisory.txt".into())
            } else {
                None
            }
        })
        .expect_err("missing");
        assert!(matches!(err, AdvisoryError::Missing { .. }), "{err:?}");
        assert!(err.to_string().contains("advisory source"), "{err}");
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "draconic-pkg-k1105-{tag}-{}-{}",
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

    fn tagged_lib(root: &Path) -> (PathBuf, String) {
        let repo = root.join("upstream");
        fs::create_dir_all(&repo).unwrap();
        git_ok(&["init"], &repo);
        git_ok(&["config", "user.email", "test@draconic.local"], &repo);
        git_ok(&["config", "user.name", "Draconic Test"], &repo);
        git_ok(&["checkout", "-B", "main"], &repo);
        fs::write(repo.join("lib.txt"), "1.2.3\n").unwrap();
        git_ok(&["add", "lib.txt"], &repo);
        git_ok(&["commit", "-m", "v1.2.3"], &repo);
        git_ok(&["tag", "v1.2.3"], &repo);
        let out = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&repo)
            .output()
            .expect("rev-parse");
        assert!(out.status.success());
        let oid = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (repo, oid)
    }

    fn consumer_manifest(git_url: &str) -> Manifest {
        parse_manifest(&format!(
            r#"
module = "github.com/acme/app"

[dependencies]
"{PATH}" = "1.2.3"

[urls]
"{PATH}" = "{git_url}"
"#
        ))
        .expect("manifest")
    }

    #[test]
    fn yank_without_advisory_resolve_still_pins() {
        // No advisory source → yank is not invented as a silent v1 check.
        let root = temp_dir("no-advisory");
        let (upstream, oid) = tagged_lib(&root);
        let m = consumer_manifest(upstream.to_str().unwrap());
        let cache = ModuleCache::new(root.join("cache"));
        let lock = resolve_direct_deps(&m, &cache).expect("pin without advisory");
        let e = lock.packages.get(PATH).expect("pin");
        assert_eq!(e.version, "1.2.3");
        assert_eq!(e.commit_oid, oid);
        let none = resolve_direct_deps_with_advisory(&m, &cache, None).expect("explicit none");
        assert_eq!(none.packages.get(PATH).unwrap().commit_oid, oid);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn yank_advisory_resolve_hard_fails_and_does_not_pin() {
        let root = temp_dir("yank-refuse");
        let (upstream, oid) = tagged_lib(&root);
        let m = consumer_manifest(upstream.to_str().unwrap());
        let cache = ModuleCache::new(root.join("cache"));
        let advisory = parse_ok("yank github.com/org/lib 1.2.3\n");

        let err = resolve_direct_deps_with_advisory(&m, &cache, Some(&advisory))
            .expect_err("yanked version");
        match &err {
            ResolveDirectError::Advisory { path, source } => {
                assert_eq!(path, PATH);
                match source {
                    AdvisoryError::Refused { version, kind, .. } => {
                        assert_eq!(version, "1.2.3");
                        assert_eq!(*kind, YankKind::Yank);
                    }
                    other => panic!("expected Refused, got {other:?}"),
                }
            }
            other => panic!("expected Advisory, got {other:?}"),
        }
        assert!(err.to_string().contains("yanked"), "{err}");
        assert!(!cache.has_entry(PATH, &oid).unwrap());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn yank_retract_advisory_resolve_hard_fails_and_does_not_pin() {
        let root = temp_dir("retract-refuse");
        let (upstream, oid) = tagged_lib(&root);
        let m = consumer_manifest(upstream.to_str().unwrap());
        let cache = ModuleCache::new(root.join("cache"));
        let advisory = parse_ok("retract github.com/org/lib 1.2.3\n");

        let err = resolve_direct_deps_with_advisory(&m, &cache, Some(&advisory))
            .expect_err("retracted version");
        match &err {
            ResolveDirectError::Advisory { source, .. } => {
                assert!(
                    matches!(
                        source,
                        AdvisoryError::Refused {
                            kind: YankKind::Retract,
                            ..
                        }
                    ),
                    "{source:?}"
                );
            }
            other => panic!("expected Advisory, got {other:?}"),
        }
        assert!(err.to_string().contains("retracted"), "{err}");
        assert!(!cache.has_entry(PATH, &oid).unwrap());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn yank_advisory_allows_unlisted_version_pin() {
        let root = temp_dir("unlisted");
        let (upstream, oid) = tagged_lib(&root);
        let m = consumer_manifest(upstream.to_str().unwrap());
        let cache = ModuleCache::new(root.join("cache"));
        let advisory = parse_ok("yank github.com/org/lib 9.9.9\n");
        let lock = resolve_direct_deps_with_advisory(&m, &cache, Some(&advisory))
            .expect("unlisted version");
        let e = lock.packages.get(PATH).expect("pin");
        assert_eq!(e.version, "1.2.3");
        assert_eq!(e.commit_oid, oid);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn yank_from_vars_loads_file_and_refuses() {
        let root = temp_dir("from-vars");
        let file = root.join("advisory.txt");
        fs::write(&file, "yank github.com/org/lib 1.2.3\n").unwrap();
        let loaded = advisory_from_vars(|k| {
            if k == ADVISORY_ENV {
                Some(file.display().to_string())
            } else {
                None
            }
        })
        .expect("load")
        .expect("some");
        loaded.refuse(PATH, "1.2.3").expect_err("yanked");
        let file_url = format!("file://{}", file.display());
        let via_url = advisory_from_vars(|k| {
            if k == ADVISORY_ENV {
                Some(file_url.clone())
            } else {
                None
            }
        })
        .expect("file url")
        .expect("some");
        via_url
            .refuse(PATH, "1.2.3")
            .expect_err("yanked via file://");
        let _ = fs::remove_dir_all(&root);
    }
}
