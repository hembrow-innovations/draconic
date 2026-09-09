use std::collections::BTreeMap;
use std::fmt;

use crate::cache::ModuleCache;
use crate::content_hash_tree;
use crate::lock::{LockEntry, LockFile};
use crate::{resolve_git_url, Manifest};

use super::{resolve_highest_matching_tag, ResolveError};

/// Error while resolving a manifest's direct dependencies to lock pins (K04.03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveDirectError {
    /// One dependency failed version/tag resolve.
    Dep { path: String, source: ResolveError },
    /// Clone/fetch/checkout into the module cache failed.
    Cache { path: String, message: String },
    /// Content hash of a checked-out package tree failed.
    ContentHash { path: String, message: String },
    /// Built lock entry failed validation (should be rare).
    LockEntry { path: String, message: String },
    /// Advisory source refused a yanked or retracted version (K11.05).
    Advisory {
        path: String,
        source: crate::AdvisoryError,
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
            ResolveDirectError::Advisory { path, source } => {
                write!(f, "resolve direct deps: `{path}`: {source}")
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
    resolve_direct_deps_with_advisory(manifest, cache, None)
}

/// [`resolve_direct_deps`] with an optional advisory source (K11.05).
///
/// When `advisory` is `Some`, a yanked or retracted resolved version hard-fails
/// and is not pinned or checked out. `None` does not invent a yank check.
pub fn resolve_direct_deps_with_advisory(
    manifest: &Manifest,
    cache: &ModuleCache,
    advisory: Option<&crate::AdvisorySource>,
) -> Result<LockFile, ResolveDirectError> {
    let mut packages = BTreeMap::new();

    for (path, req) in &manifest.dependencies {
        let git_url = resolve_git_url(manifest, path);

        let vcs = cache
            .clone_or_fetch(path, &git_url)
            .map_err(|e| ResolveDirectError::Cache {
                path: path.clone(),
                message: e.to_string(),
            })?;

        let resolved =
            resolve_highest_matching_tag(&vcs, req).map_err(|source| ResolveDirectError::Dep {
                path: path.clone(),
                source,
            })?;

        if let Some(source) = advisory {
            source
                .refuse(path, &resolved.version)
                .map_err(|e| ResolveDirectError::Advisory {
                    path: path.clone(),
                    source: e,
                })?;
        }

        let subdir = crate::derive_package_subdir(path, &git_url);
        let checkout = cache
            .checkout_with_subdir(path, &resolved.commit_oid, &git_url, &subdir)
            .map_err(|e| ResolveDirectError::Cache {
                path: path.clone(),
                message: e.to_string(),
            })?;

        let content_hash =
            content_hash_tree(&checkout).map_err(|e| ResolveDirectError::ContentHash {
                path: path.clone(),
                message: e.to_string(),
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
        })?
        .with_subdir(subdir)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use crate::resolve::resolve_highest_matching_tag;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "draconic-pkg-k0403-{tag}-{}-{}",
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
            replace: BTreeMap::new(),
            toolchain: None,
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
        let m = manifest_deps(&[(path, "^1.0.0")], &[(path, upstream.to_str().unwrap())]);

        let lock = resolve_direct_deps(&m, &cache).expect("resolve");
        assert_eq!(lock.packages.len(), 1);
        let e = lock.packages.get(path).expect("pin");
        assert_eq!(e.path, path);
        assert_eq!(e.version, "1.2.3");
        assert_eq!(e.commit_oid, oid_123);
        assert_eq!(e.git_url, upstream.to_str().unwrap());
        assert_eq!(e.content_hash.len(), 64);
        assert!(e
            .content_hash
            .chars()
            .all(|c| matches!(c, '0'..='9' | 'a'..='f')));

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
        let m = manifest_deps(&[(path, "1.0.0")], &[(path, upstream.to_str().unwrap())]);
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
        let m = manifest_deps(&[(path, "9.9.9")], &[(path, upstream.to_str().unwrap())]);
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
        let m = manifest_deps(&[(path, "1.0.0")], &[(path, missing.to_str().unwrap())]);
        let err = resolve_direct_deps(&m, &cache).expect_err("missing remote");
        assert!(matches!(err, ResolveDirectError::Cache { .. }), "{err:?}");
        assert!(err.to_string().contains(path));
        let _ = fs::remove_dir_all(&root);
    }

    // --- K04: combined version resolve (parent of K04.01–K04.03) ---

    #[test]
    fn k04_combined_semver_tag_to_oid_fail_closed_direct_pins() {
        let root = temp_dir("k04-combined");
        let (upstream, _, _, oid_123) = tagged_fixture(&root);

        // Highest matching semver tag → commit OID (K04.01).
        let r = resolve_highest_matching_tag(&upstream, "^1.0.0").expect("highest match");
        assert_eq!(r.version, "1.2.3");
        assert_eq!(r.tag, "v1.2.3");
        assert_eq!(r.commit_oid, oid_123);
        assert_eq!(r.commit_oid.len(), 40);
        assert!(r
            .commit_oid
            .chars()
            .all(|c| matches!(c, '0'..='9' | 'a'..='f')));
        assert_ne!(r.version, "2.0.0");

        // Fail closed: no match / empty tags / non-semver-only (K04.02).
        let no_match = resolve_highest_matching_tag(&upstream, "9.9.9").expect_err("no 9.x");
        match &no_match {
            ResolveError::NoMatch { req } => assert_eq!(req, "9.9.9"),
            other => panic!("expected NoMatch, got {other:?}"),
        }
        assert!(no_match.to_string().contains("no semver tag"));

        let empty_repo = root.join("empty-tags");
        fs::create_dir_all(&empty_repo).unwrap();
        git_ok(&["init"], &empty_repo);
        git_ok(
            &["config", "user.email", "test@draconic.local"],
            &empty_repo,
        );
        git_ok(&["config", "user.name", "Draconic Test"], &empty_repo);
        git_ok(&["checkout", "-B", "main"], &empty_repo);
        let _ = commit_file(&empty_repo, "a.txt", "x\n", "init");
        let empty = resolve_highest_matching_tag(&empty_repo, "1.0.0").expect_err("no tags");
        assert!(
            matches!(empty, ResolveError::EmptyTags),
            "expected EmptyTags, got {empty:?}"
        );
        assert!(empty.to_string().contains("no tags"));

        let nonsemver = root.join("nonsemver-only");
        fs::create_dir_all(&nonsemver).unwrap();
        git_ok(&["init"], &nonsemver);
        git_ok(&["config", "user.email", "test@draconic.local"], &nonsemver);
        git_ok(&["config", "user.name", "Draconic Test"], &nonsemver);
        git_ok(&["checkout", "-B", "main"], &nonsemver);
        let _ = commit_file(&nonsemver, "a.txt", "x\n", "init");
        git_ok(&["tag", "latest"], &nonsemver);
        let only = resolve_highest_matching_tag(&nonsemver, "1.0.0").expect_err("no semver");
        assert!(
            matches!(only, ResolveError::NonSemverOnly),
            "expected NonSemverOnly, got {only:?}"
        );
        assert!(only.to_string().contains("none are semver"));

        // Direct-deps set → lock pins; v1 does not walk nested package deps (K04.03).
        let nested = root.join("nested-upstream");
        fs::create_dir_all(&nested).unwrap();
        git_ok(&["init"], &nested);
        git_ok(&["config", "user.email", "test@draconic.local"], &nested);
        git_ok(&["config", "user.name", "Draconic Test"], &nested);
        git_ok(&["checkout", "-B", "main"], &nested);
        fs::write(
            nested.join("draconic.toml"),
            r#"module = "github.com/org/lib"

[dependencies]
"github.com/transitive/only" = "1.0.0"
"#,
        )
        .unwrap();
        fs::write(nested.join("lib.drac"), "export let x = 1;\n").unwrap();
        git_ok(&["add", "."], &nested);
        git_ok(&["commit", "-m", "v1.0.0"], &nested);
        git_ok(&["tag", "v1.0.0"], &nested);
        let nested_oid = {
            let out = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&nested)
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };

        let cache = ModuleCache::new(root.join("cache"));
        let lib_path = "github.com/org/lib";
        let util_path = "github.com/org/util";
        let m = manifest_deps(
            &[(lib_path, "1.0.0"), (util_path, "^1.0.0")],
            &[
                (lib_path, nested.to_str().unwrap()),
                (util_path, upstream.to_str().unwrap()),
            ],
        );
        let lock = resolve_direct_deps(&m, &cache).expect("direct pins");
        assert_eq!(lock.version, 1);
        assert_eq!(lock.packages.len(), 2);
        let keys: Vec<_> = lock.packages.keys().cloned().collect();
        assert_eq!(keys, vec![lib_path.to_string(), util_path.to_string()]);
        assert!(!lock.packages.contains_key("github.com/transitive/only"));

        let lib = lock.packages.get(lib_path).expect("lib pin");
        assert_eq!(lib.version, "1.0.0");
        assert_eq!(lib.commit_oid, nested_oid);
        assert_eq!(lib.git_url, nested.to_str().unwrap());
        assert_eq!(lib.content_hash.len(), 64);

        let util = lock.packages.get(util_path).expect("util pin");
        assert_eq!(util.version, "1.2.3");
        assert_eq!(util.commit_oid, oid_123);
        assert_eq!(util.git_url, upstream.to_str().unwrap());
        assert_eq!(util.content_hash.len(), 64);
        assert!(util
            .content_hash
            .chars()
            .all(|c| matches!(c, '0'..='9' | 'a'..='f')));

        let util_dir = cache.entry_dir(util_path, &oid_123).unwrap();
        assert!(cache.has_entry(util_path, &oid_123).unwrap());
        assert_eq!(
            crate::content_hash_tree(&util_dir).unwrap(),
            util.content_hash
        );

        let _ = fs::remove_dir_all(&root);
    }
}
