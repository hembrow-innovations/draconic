//! Multi-module monorepo: module path → subdirectory of a git repo (Roadmap K11.03).
//!
//! One git checkout can host more than one module. A module path that is longer
//! than the repo identity (the git URL, minus scheme / `.git`) maps to the
//! remainder as a package root inside that checkout. Sibling modules are
//! distinct packages (distinct cache entries, hashes, and import roots).

/// Validate a package subdirectory relative to a git tree (K11.03).
///
/// Empty means the repository root (single-module checkout). Non-empty values
/// are `/`-separated relative paths with no `.` / `..` / empty segments.
pub fn validate_package_subdir(subdir: &str) -> Result<(), &'static str> {
    if subdir.is_empty() {
        return Ok(());
    }
    if subdir != subdir.trim() {
        return Err("must not have leading or trailing whitespace");
    }
    if subdir.starts_with('/') || subdir.ends_with('/') {
        return Err("must not start or end with '/'");
    }
    if subdir.contains("//") {
        return Err("must not contain empty path segments");
    }
    if subdir.chars().any(|c| c.is_whitespace()) {
        return Err("must not contain whitespace");
    }
    if subdir.contains('\\') || subdir.contains(':') || subdir.contains('\0') {
        return Err("must not contain '\\', ':', or NUL");
    }
    for seg in subdir.split('/') {
        if seg.is_empty() {
            return Err("must not contain empty path segments");
        }
        if seg == "." || seg == ".." {
            return Err("must not contain '.' or '..' path segments");
        }
        if !seg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
        {
            return Err("segments may only contain ASCII letters, digits, '.', '-', '_'");
        }
    }
    Ok(())
}

/// Derive the package subdirectory from a module path and the git remote that
/// hosts it (K11.03).
///
/// When `git_url` normalizes to a module-path prefix of `module_path`, the
/// remainder is the subdirectory. Example: module `github.com/org/mono/pkg/foo`
/// + `https://github.com/org/mono.git` → `pkg/foo`.
///
/// Default `https://{module_path}.git` (repo identity equals the module path)
/// yields an empty subdir (repository root). `file://` and absolute local paths
/// cannot encode that prefix and yield empty — callers pass an explicit subdir.
pub fn derive_package_subdir(module_path: &str, git_url: &str) -> String {
    let Some(repo) = repo_path_from_git_url(git_url) else {
        return String::new();
    };
    if module_path == repo {
        return String::new();
    }
    let Some(rest) = module_path.strip_prefix(&repo) else {
        return String::new();
    };
    let Some(sub) = rest.strip_prefix('/') else {
        return String::new();
    };
    if sub.is_empty() || validate_package_subdir(sub).is_err() {
        return String::new();
    }
    sub.to_string()
}

/// Normalize a clone URL to a Go-like module-path prefix (`host/org/repo`).
///
/// Returns `None` for `file://` and absolute local paths (no host/path identity).
pub fn repo_path_from_git_url(git_url: &str) -> Option<String> {
    let url = git_url.trim();
    if url.is_empty() {
        return None;
    }

    let path = if let Some(rest) = url.strip_prefix("https://") {
        rest
    } else if let Some(rest) = url.strip_prefix("http://") {
        rest
    } else if let Some(rest) = url.strip_prefix("git://") {
        rest
    } else if let Some(rest) = url.strip_prefix("ssh://") {
        rest.split_once('@').map(|(_, host)| host).unwrap_or(rest)
    } else if let Some(rest) = url.strip_prefix("git@") {
        let (host, repo) = rest.split_once(':')?;
        let repo = strip_git_suffix(repo.trim_matches('/'));
        if host.is_empty() || repo.is_empty() {
            return None;
        }
        return Some(format!("{host}/{repo}"));
    } else {
        return None;
    };

    let path = path.split_once('?').map(|(p, _)| p).unwrap_or(path);
    let path = path.split_once('#').map(|(p, _)| p).unwrap_or(path);
    let path = path.split_once('@').map(|(_, host)| host).unwrap_or(path);
    let path = strip_git_suffix(path.trim_matches('/'));
    if path.is_empty() || !path.contains('/') {
        return None;
    }
    Some(path.to_string())
}

fn strip_git_suffix(path: &str) -> &str {
    path.strip_suffix(".git").unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::ModuleCache;
    use crate::content_hash_tree;
    use crate::lock::{parse_lock, write_lock, LockEntry, LockFile};
    use crate::resolve_module_import;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    const OID: &str = "0123456789abcdef0123456789abcdef01234567";
    const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const FOO_PATH: &str = "github.com/org/monorepo/pkg/foo";
    const BAR_PATH: &str = "github.com/org/monorepo/pkg/bar";
    const MONO_URL: &str = "https://github.com/org/monorepo.git";

    fn temp_dir(tag: &str) -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "draconic-pkg-k1103-{tag}-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            N.fetch_add(1, Ordering::Relaxed)
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

    /// Monorepo with sibling modules `pkg/foo` and `pkg/bar` plus a root README.
    fn monorepo_fixture(root: &Path) -> (PathBuf, String) {
        let repo = root.join("upstream");
        fs::create_dir_all(repo.join("pkg/foo")).unwrap();
        fs::create_dir_all(repo.join("pkg/bar")).unwrap();
        git_ok(&["init"], &repo);
        git_ok(&["config", "user.email", "test@draconic.local"], &repo);
        git_ok(&["config", "user.name", "Draconic Test"], &repo);
        git_ok(&["checkout", "-B", "main"], &repo);
        fs::write(repo.join("README.md"), "monorepo root\n").unwrap();
        fs::write(
            repo.join("pkg/foo/index.drac"),
            "export let name = \"foo\";\n",
        )
        .unwrap();
        fs::write(repo.join("pkg/foo/hello.txt"), "from foo\n").unwrap();
        fs::write(
            repo.join("pkg/bar/index.drac"),
            "export let name = \"bar\";\n",
        )
        .unwrap();
        fs::write(repo.join("pkg/bar/hello.txt"), "from bar\n").unwrap();
        git_ok(&["add", "."], &repo);
        git_ok(&["commit", "-m", "monorepo modules"], &repo);
        git_ok(&["tag", "v1.0.0"], &repo);
        let oid = {
            let out = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&repo)
                .output()
                .expect("rev-parse");
            assert!(out.status.success());
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };
        (repo, oid)
    }

    // --- derive / validate ---

    #[test]
    fn derive_package_subdir_https_prefix() {
        assert_eq!(derive_package_subdir(FOO_PATH, MONO_URL), "pkg/foo");
        assert_eq!(derive_package_subdir(BAR_PATH, MONO_URL), "pkg/bar");
        assert_eq!(
            derive_package_subdir("github.com/org/monorepo", MONO_URL),
            ""
        );
    }

    #[test]
    fn derive_package_subdir_ssh_and_git_urls() {
        assert_eq!(
            derive_package_subdir(FOO_PATH, "git@github.com:org/monorepo.git"),
            "pkg/foo"
        );
        assert_eq!(
            derive_package_subdir(FOO_PATH, "ssh://git@github.com/org/monorepo.git"),
            "pkg/foo"
        );
        assert_eq!(
            derive_package_subdir(FOO_PATH, "git://github.com/org/monorepo.git"),
            "pkg/foo"
        );
    }

    #[test]
    fn derive_package_subdir_default_url_is_repo_root() {
        assert_eq!(
            derive_package_subdir("github.com/org/lib", "https://github.com/org/lib.git"),
            ""
        );
        assert_eq!(
            derive_package_subdir(
                "gitlab.com/group/sub/mod",
                "https://gitlab.com/group/sub/mod.git"
            ),
            ""
        );
    }

    #[test]
    fn derive_package_subdir_local_path_empty() {
        assert_eq!(derive_package_subdir(FOO_PATH, "/tmp/fixture-monorepo"), "");
        assert_eq!(
            derive_package_subdir(FOO_PATH, "file:///tmp/fixture-monorepo.git"),
            ""
        );
    }

    #[test]
    fn validate_package_subdir_accepts_relative() {
        validate_package_subdir("").expect("empty root");
        validate_package_subdir("pkg").expect("one");
        validate_package_subdir("pkg/foo").expect("nested");
    }

    #[test]
    fn validate_package_subdir_rejects_dotdot_and_absolute() {
        assert!(validate_package_subdir("..").is_err());
        assert!(validate_package_subdir("pkg/../evil").is_err());
        assert!(validate_package_subdir("/pkg/foo").is_err());
        assert!(validate_package_subdir("pkg/foo/").is_err());
        assert!(validate_package_subdir("pkg//foo").is_err());
        assert!(validate_package_subdir("pkg:foo").is_err());
    }

    // --- lock parse / write ---

    #[test]
    fn lock_parse_write_subdir_round_trip() {
        let src = format!(
            r#"version = 1

[[package]]
path = "{FOO_PATH}"
version = "1.0.0"
git_url = "{MONO_URL}"
commit_oid = "{OID}"
content_hash = "{HASH}"
subdir = "pkg/foo"
"#
        );
        let lock = parse_lock(&src).expect("parse lock with subdir");
        let entry = lock.packages.get(FOO_PATH).expect("foo pin");
        assert_eq!(entry.subdir, "pkg/foo");
        assert_eq!(entry.git_url, MONO_URL);
        assert_eq!(entry.path, FOO_PATH);

        let written = write_lock(&lock);
        assert!(
            written.contains("subdir = \"pkg/foo\""),
            "write must emit subdir: {written}"
        );
        let again = parse_lock(&written).expect("reparse");
        assert_eq!(again, lock);
        assert_eq!(write_lock(&again), written);
    }

    #[test]
    fn lock_parse_omitted_subdir_is_empty_and_write_omits_it() {
        let src = format!(
            r#"version = 1

[[package]]
path = "github.com/org/lib"
version = "1.0.0"
git_url = "https://github.com/org/lib.git"
commit_oid = "{OID}"
content_hash = "{HASH}"
"#
        );
        let lock = parse_lock(&src).expect("parse without subdir");
        let entry = lock.packages.get("github.com/org/lib").unwrap();
        assert_eq!(entry.subdir, "");
        let written = write_lock(&lock);
        assert!(
            !written.contains("subdir"),
            "empty subdir must not be written (K02.03 byte-identical): {written}"
        );
        assert_eq!(written, src);
    }

    #[test]
    fn lock_rejects_invalid_subdir() {
        let src = format!(
            r#"version = 1

[[package]]
path = "{FOO_PATH}"
version = "1.0.0"
git_url = "{MONO_URL}"
commit_oid = "{OID}"
content_hash = "{HASH}"
subdir = "../evil"
"#
        );
        let err = parse_lock(&src).expect_err("bad subdir");
        let msg = err.to_string();
        assert!(
            msg.contains("subdir") || msg.contains(".."),
            "diagnostic should mention subdir: {msg}"
        );
    }

    #[test]
    fn lock_entry_with_subdir_distinct_from_sibling() {
        let foo = LockEntry::new(FOO_PATH, "1.0.0", MONO_URL, OID, HASH)
            .unwrap()
            .with_subdir("pkg/foo")
            .unwrap();
        let bar = LockEntry::new(BAR_PATH, "1.0.0", MONO_URL, OID, HASH)
            .unwrap()
            .with_subdir("pkg/bar")
            .unwrap();
        assert_ne!(foo.path, bar.path);
        assert_ne!(foo.subdir, bar.subdir);
        assert_eq!(foo.git_url, bar.git_url);
        assert_eq!(foo.commit_oid, bar.commit_oid);
    }

    // --- checkout / import apply ---

    #[test]
    fn checkout_subdir_is_package_root_not_repo_root() {
        let root = temp_dir("checkout-foo");
        let (upstream, oid) = monorepo_fixture(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();

        let dest = cache
            .checkout_with_subdir(FOO_PATH, &oid, url, "pkg/foo")
            .expect("checkout foo subdir");

        assert!(dest.ends_with(&oid), "{dest:?}");
        assert!(dest.to_string_lossy().contains("pkg/foo"));
        let body = fs::read_to_string(dest.join("index.drac")).expect("foo index");
        assert!(body.contains("foo"), "{body}");
        assert_eq!(
            fs::read_to_string(dest.join("hello.txt")).unwrap(),
            "from foo\n"
        );
        assert!(
            !dest.join("README.md").exists(),
            "repo root must not be the package root"
        );
        assert!(
            !dest.join("pkg").exists(),
            "sibling prefix pkg/ must not appear under foo package root"
        );
        assert!(
            !dest.join("bar").exists() && !dest.join("pkg/bar").exists(),
            "sibling bar must not be inside foo package"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn checkout_sibling_modules_are_distinct_packages() {
        let root = temp_dir("siblings");
        let (upstream, oid) = monorepo_fixture(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();

        let foo = cache
            .checkout_with_subdir(FOO_PATH, &oid, url, "pkg/foo")
            .expect("foo");
        let bar = cache
            .checkout_with_subdir(BAR_PATH, &oid, url, "pkg/bar")
            .expect("bar");

        assert_ne!(foo, bar, "siblings must not share a cache entry");
        assert_eq!(
            fs::read_to_string(foo.join("hello.txt")).unwrap(),
            "from foo\n"
        );
        assert_eq!(
            fs::read_to_string(bar.join("hello.txt")).unwrap(),
            "from bar\n"
        );
        let foo_hash = content_hash_tree(&foo).expect("foo hash");
        let bar_hash = content_hash_tree(&bar).expect("bar hash");
        assert_ne!(
            foo_hash, bar_hash,
            "sibling package trees must not share a content hash"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn checkout_missing_subdir_fails_closed() {
        let root = temp_dir("missing");
        let (upstream, oid) = monorepo_fixture(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();

        let err = cache
            .checkout_with_subdir(FOO_PATH, &oid, url, "pkg/missing")
            .expect_err("missing subdir");
        let msg = err.to_string();
        assert!(
            msg.contains("pkg/missing") || msg.contains("subdir") || msg.contains("git"),
            "missing subdir diagnostic: {msg}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn checkout_rejects_dotdot_subdir() {
        let root = temp_dir("dotdot");
        let (upstream, oid) = monorepo_fixture(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();

        let err = cache
            .checkout_with_subdir(FOO_PATH, &oid, url, "pkg/../pkg/foo")
            .expect_err("dotdot");
        let msg = err.to_string();
        assert!(
            msg.contains("subdir") || msg.contains(".."),
            "dotdot subdir diagnostic: {msg}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn import_resolve_honors_subdir_package_root_not_sibling() {
        let root = temp_dir("import");
        let (upstream, oid) = monorepo_fixture(&root);
        let cache = ModuleCache::new(root.join("cache"));
        let url = upstream.to_str().unwrap();

        let foo_dir = cache
            .checkout_with_subdir(FOO_PATH, &oid, url, "pkg/foo")
            .expect("foo");
        let bar_dir = cache
            .checkout_with_subdir(BAR_PATH, &oid, url, "pkg/bar")
            .expect("bar");
        let foo_hash = content_hash_tree(&foo_dir).unwrap();
        let bar_hash = content_hash_tree(&bar_dir).unwrap();

        let mut packages = BTreeMap::new();
        packages.insert(
            FOO_PATH.to_string(),
            LockEntry::new(FOO_PATH, "1.0.0", MONO_URL, &oid, foo_hash)
                .unwrap()
                .with_subdir("pkg/foo")
                .unwrap(),
        );
        packages.insert(
            BAR_PATH.to_string(),
            LockEntry::new(BAR_PATH, "1.0.0", MONO_URL, &oid, bar_hash)
                .unwrap()
                .with_subdir("pkg/bar")
                .unwrap(),
        );
        let lock = LockFile {
            version: 1,
            packages,
        };

        let foo = resolve_module_import(FOO_PATH, &lock, &cache).expect("import foo");
        assert_eq!(foo.module_path, FOO_PATH);
        let foo_body = fs::read_to_string(&foo.file).unwrap();
        assert!(foo_body.contains("foo"), "{foo_body}");
        assert!(
            !foo.package_root.join("README.md").exists(),
            "import package root must be the subdir, not the repo"
        );

        let bar = resolve_module_import(BAR_PATH, &lock, &cache).expect("import bar");
        assert_eq!(bar.module_path, BAR_PATH);
        let bar_body = fs::read_to_string(&bar.file).unwrap();
        assert!(bar_body.contains("bar"), "{bar_body}");
        assert_ne!(foo.package_root, bar.package_root);

        let _ = fs::remove_dir_all(&root);
    }
}
