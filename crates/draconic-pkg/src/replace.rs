//! K11.02: `replace` directive — fork git source or local path override.
//!
//! A Program maps a declared module path to a fork (`git` URL or `module` path)
//! or a local directory. Resolve/fetch use that source instead of the declared
//! identity; lock pins record the replacement, never the silent original.

use std::collections::BTreeMap;
use std::path::Path;

use toml::Value as TomlValue;

use crate::{default_git_url, validate_git_url, validate_module_path, ManifestError};

/// Known keys inside a `[replace]` inline table.
const KNOWN_REPLACE_KEYS: &[&str] = &["git", "module", "path"];

/// Replacement source for one declared module path (K11.02).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplaceSource {
    /// Fork: clone this git URL instead of the declared identity.
    Git {
        /// Clone URL (https/ssh/file/absolute).
        url: String,
    },
    /// Fork identified by a different module path (URL derived).
    Module {
        /// Replacement module path (Go-like).
        path: String,
    },
    /// Local directory (git working tree or bare repo) used as the source.
    Path {
        /// Absolute or relative (`./` / `../`) filesystem path.
        path: String,
    },
}

impl ReplaceSource {
    /// Fetch/clone URL or local path used instead of the declared identity.
    pub fn fetch_url(&self) -> String {
        match self {
            ReplaceSource::Git { url } => url.clone(),
            ReplaceSource::Module { path } => default_git_url(path),
            ReplaceSource::Path { path } => path.clone(),
        }
    }
}

/// Decode `[replace]` from a TOML table (string shorthand or `{ git | module | path }`).
pub(crate) fn parse_replace_table(
    table: &toml::map::Map<String, TomlValue>,
) -> Result<BTreeMap<String, ReplaceSource>, ManifestError> {
    let mut map = BTreeMap::new();
    for (path, value) in table {
        map.insert(path.clone(), parse_replace_value(path, value)?);
    }
    Ok(map)
}

fn parse_replace_value(path: &str, value: &TomlValue) -> Result<ReplaceSource, ManifestError> {
    match value {
        TomlValue::String(s) => parse_replace_string(path, s),
        TomlValue::Table(table) => parse_replace_inline_table(path, table),
        _ => Err(ManifestError::InvalidReplaceValue {
            path: path.to_string(),
        }),
    }
}

fn parse_replace_string(path: &str, raw: &str) -> Result<ReplaceSource, ManifestError> {
    if looks_like_local_path(raw) {
        if let Err(reason) = validate_local_replace_path(raw) {
            return Err(ManifestError::InvalidReplaceSource {
                path: path.to_string(),
                source: raw.to_string(),
                reason,
            });
        }
        return Ok(ReplaceSource::Path {
            path: raw.to_string(),
        });
    }
    if validate_git_url(raw).is_ok() {
        return Ok(ReplaceSource::Git {
            url: raw.to_string(),
        });
    }
    if validate_module_path(raw).is_ok() {
        return Ok(ReplaceSource::Module {
            path: raw.to_string(),
        });
    }
    Err(ManifestError::InvalidReplaceSource {
        path: path.to_string(),
        source: raw.to_string(),
        reason: "must be a git URL, module path, or local path",
    })
}

fn parse_replace_inline_table(
    path: &str,
    table: &toml::map::Map<String, TomlValue>,
) -> Result<ReplaceSource, ManifestError> {
    for key in table.keys() {
        if !KNOWN_REPLACE_KEYS.contains(&key.as_str()) {
            return Err(ManifestError::UnknownField { field: key.clone() });
        }
    }
    let git = string_field(table, "git", path)?;
    let module = string_field(table, "module", path)?;
    let local = string_field(table, "path", path)?;
    let set = [git.is_some(), module.is_some(), local.is_some()]
        .into_iter()
        .filter(|b| *b)
        .count();
    if set == 0 {
        return Err(ManifestError::MissingReplaceSource {
            path: path.to_string(),
        });
    }
    if set > 1 {
        return Err(ManifestError::AmbiguousReplace {
            path: path.to_string(),
        });
    }
    if let Some(url) = git {
        if let Err(reason) = validate_git_url(&url) {
            return Err(ManifestError::InvalidReplaceSource {
                path: path.to_string(),
                source: url,
                reason,
            });
        }
        return Ok(ReplaceSource::Git { url });
    }
    if let Some(module_path) = module {
        if let Err(reason) = validate_module_path(&module_path) {
            return Err(ManifestError::InvalidReplaceSource {
                path: path.to_string(),
                source: module_path,
                reason,
            });
        }
        return Ok(ReplaceSource::Module { path: module_path });
    }
    let local = local.expect("path present");
    if let Err(reason) = validate_local_replace_path(&local) {
        return Err(ManifestError::InvalidReplaceSource {
            path: path.to_string(),
            source: local,
            reason,
        });
    }
    Ok(ReplaceSource::Path { path: local })
}

fn string_field(
    table: &toml::map::Map<String, TomlValue>,
    key: &str,
    path: &str,
) -> Result<Option<String>, ManifestError> {
    match table.get(key) {
        None => Ok(None),
        Some(TomlValue::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(ManifestError::InvalidReplaceValue {
            path: path.to_string(),
        }),
    }
}

fn looks_like_local_path(s: &str) -> bool {
    s == "."
        || s == ".."
        || s.starts_with("./")
        || s.starts_with("../")
        || Path::new(s).is_absolute()
}

fn validate_local_replace_path(path: &str) -> Result<(), &'static str> {
    if path.is_empty() {
        return Err("must not be empty");
    }
    if path != path.trim() {
        return Err("must not have leading or trailing whitespace");
    }
    if path.chars().any(|c| c.is_whitespace()) {
        return Err("must not contain whitespace");
    }
    if !looks_like_local_path(path) {
        return Err("must be an absolute path or start with ./ or ../");
    }
    Ok(())
}

/// Schema-check `[replace]` keys and sources on an already-decoded map.
pub(crate) fn validate_replace(
    replace: &BTreeMap<String, ReplaceSource>,
) -> Result<(), ManifestError> {
    for (path, source) in replace {
        if let Err(reason) = validate_module_path(path) {
            return Err(ManifestError::InvalidReplacePath {
                path: path.clone(),
                reason,
            });
        }
        match source {
            ReplaceSource::Git { url } => {
                if let Err(reason) = validate_git_url(url) {
                    return Err(ManifestError::InvalidReplaceSource {
                        path: path.clone(),
                        source: url.clone(),
                        reason,
                    });
                }
            }
            ReplaceSource::Module { path: module } => {
                if let Err(reason) = validate_module_path(module) {
                    return Err(ManifestError::InvalidReplaceSource {
                        path: path.clone(),
                        source: module.clone(),
                        reason,
                    });
                }
            }
            ReplaceSource::Path { path: local } => {
                if let Err(reason) = validate_local_replace_path(local) {
                    return Err(ManifestError::InvalidReplaceSource {
                        path: path.clone(),
                        source: local.clone(),
                        reason,
                    });
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        parse_manifest, resolve_direct_deps, resolve_git_url, write_manifest, Manifest,
        ManifestError, ModuleCache,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    fn parse_ok(src: &str) -> Manifest {
        parse_manifest(src).unwrap_or_else(|e| panic!("parse: {e}"))
    }

    #[test]
    fn replace_omitted_is_empty() {
        let m = parse_ok(r#"module = "github.com/acme/app""#);
        assert!(m.replace.is_empty());
    }

    #[test]
    fn replace_empty_table() {
        let m = parse_ok(
            r#"
module = "github.com/acme/app"
[replace]
"#,
        );
        assert!(m.replace.is_empty());
    }

    #[test]
    fn replace_parse_git_table() {
        let m = parse_ok(
            r#"
module = "github.com/acme/app"

[dependencies]
"github.com/org/lib" = "1.0.0"

[replace]
"github.com/org/lib" = { git = "https://github.com/fork/lib.git" }
"#,
        );
        match m.replace.get("github.com/org/lib") {
            Some(ReplaceSource::Git { url }) => {
                assert_eq!(url, "https://github.com/fork/lib.git");
            }
            other => panic!("expected Git replace, got {other:?}"),
        }
        assert_eq!(
            resolve_git_url(&m, "github.com/org/lib"),
            "https://github.com/fork/lib.git"
        );
    }

    #[test]
    fn replace_parse_path_table() {
        let m = parse_ok(
            r#"
module = "github.com/acme/app"

[replace]
"github.com/org/lib" = { path = "/tmp/local-lib" }
"#,
        );
        match m.replace.get("github.com/org/lib") {
            Some(ReplaceSource::Path { path }) => {
                assert_eq!(path, "/tmp/local-lib");
            }
            other => panic!("expected Path replace, got {other:?}"),
        }
        assert_eq!(resolve_git_url(&m, "github.com/org/lib"), "/tmp/local-lib");
    }

    #[test]
    fn replace_parse_module_table() {
        let m = parse_ok(
            r#"
module = "github.com/acme/app"

[replace]
"github.com/org/lib" = { module = "github.com/fork/lib" }
"#,
        );
        match m.replace.get("github.com/org/lib") {
            Some(ReplaceSource::Module { path }) => {
                assert_eq!(path, "github.com/fork/lib");
            }
            other => panic!("expected Module replace, got {other:?}"),
        }
        assert_eq!(
            resolve_git_url(&m, "github.com/org/lib"),
            "https://github.com/fork/lib.git"
        );
    }

    #[test]
    fn replace_parse_string_git_url() {
        let m = parse_ok(
            r#"
module = "github.com/acme/app"

[replace]
"github.com/org/lib" = "https://git.example.com/fork/lib.git"
"#,
        );
        match m.replace.get("github.com/org/lib") {
            Some(ReplaceSource::Git { url }) => {
                assert_eq!(url, "https://git.example.com/fork/lib.git");
            }
            other => panic!("expected Git replace, got {other:?}"),
        }
    }

    #[test]
    fn replace_parse_string_module_path() {
        let m = parse_ok(
            r#"
module = "github.com/acme/app"

[replace]
"github.com/org/lib" = "github.com/fork/lib"
"#,
        );
        match m.replace.get("github.com/org/lib") {
            Some(ReplaceSource::Module { path }) => {
                assert_eq!(path, "github.com/fork/lib");
            }
            other => panic!("expected Module replace, got {other:?}"),
        }
        assert_eq!(
            resolve_git_url(&m, "github.com/org/lib"),
            "https://github.com/fork/lib.git"
        );
    }

    #[test]
    fn replace_parse_string_relative_path() {
        let m = parse_ok(
            r#"
module = "github.com/acme/app"

[replace]
"github.com/org/lib" = "../vendor/lib"
"#,
        );
        match m.replace.get("github.com/org/lib") {
            Some(ReplaceSource::Path { path }) => {
                assert_eq!(path, "../vendor/lib");
            }
            other => panic!("expected Path replace, got {other:?}"),
        }
    }

    #[test]
    fn replace_wins_over_urls_and_default() {
        let m = parse_ok(
            r#"
module = "github.com/acme/app"

[dependencies]
"github.com/org/lib" = "1.0.0"

[urls]
"github.com/org/lib" = "https://git.example.com/mirror/lib.git"

[replace]
"github.com/org/lib" = { git = "https://github.com/fork/lib.git" }
"#,
        );
        assert_eq!(
            resolve_git_url(&m, "github.com/org/lib"),
            "https://github.com/fork/lib.git"
        );
        // Unreplaced paths still use urls/default.
        assert_eq!(
            resolve_git_url(&m, "github.com/other/util"),
            "https://github.com/other/util.git"
        );
    }

    #[test]
    fn replace_write_round_trip() {
        let original = parse_ok(
            r#"
module = "github.com/acme/app"

[dependencies]
"github.com/org/lib" = "1.0.0"

[replace]
"github.com/org/lib" = { git = "https://github.com/fork/lib.git" }
"github.com/org/util" = { path = "/tmp/util" }
"github.com/org/mod" = { module = "github.com/fork/mod" }
"#,
        );
        let written = write_manifest(&original);
        let expected = "\
module = \"github.com/acme/app\"

[dependencies]
\"github.com/org/lib\" = \"1.0.0\"

[replace]
\"github.com/org/lib\" = { git = \"https://github.com/fork/lib.git\" }
\"github.com/org/mod\" = { module = \"github.com/fork/mod\" }
\"github.com/org/util\" = { path = \"/tmp/util\" }
";
        assert_eq!(written, expected);
        let parsed = parse_manifest(&written).expect("parse written");
        assert_eq!(parsed, original);
        assert_eq!(write_manifest(&parsed), written);
    }

    #[test]
    fn replace_write_omits_empty_table() {
        let m = parse_ok(r#"module = "github.com/acme/app""#);
        let s = write_manifest(&m);
        assert!(!s.contains("[replace]"), "{s}");
    }

    #[test]
    fn replace_reject_not_table() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
replace = "nope"
"#,
        )
        .expect_err("replace not table");
        assert_eq!(err, ManifestError::InvalidReplace);
        assert!(err.to_string().contains("replace"), "{err}");
    }

    #[test]
    fn replace_reject_value_not_string_or_table() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
[replace]
"github.com/org/lib" = 123
"#,
        )
        .expect_err("bad value");
        match err {
            ManifestError::InvalidReplaceValue { path } => {
                assert_eq!(path, "github.com/org/lib");
            }
            other => panic!("expected InvalidReplaceValue, got {other:?}"),
        }
    }

    #[test]
    fn replace_reject_invalid_key() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
[replace]
"not-a-path" = { git = "https://github.com/fork/lib.git" }
"#,
        )
        .expect_err("bad key");
        match &err {
            ManifestError::InvalidReplacePath { path, reason } => {
                assert_eq!(path, "not-a-path");
                assert!(!reason.is_empty());
            }
            other => panic!("expected InvalidReplacePath, got {other:?}"),
        }
        assert!(err.to_string().contains("replace"), "{err}");
    }

    #[test]
    fn replace_reject_bad_git_url() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
[replace]
"github.com/org/lib" = { git = "ftp://example.com/lib" }
"#,
        )
        .expect_err("ftp");
        assert!(
            matches!(err, ManifestError::InvalidReplaceSource { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn replace_reject_ambiguous_git_and_path() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
[replace]
"github.com/org/lib" = { git = "https://github.com/fork/lib.git", path = "/tmp/lib" }
"#,
        )
        .expect_err("ambiguous");
        match err {
            ManifestError::AmbiguousReplace { path } => {
                assert_eq!(path, "github.com/org/lib");
            }
            other => panic!("expected AmbiguousReplace, got {other:?}"),
        }
    }

    #[test]
    fn replace_reject_empty_table_source() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
[replace]
"github.com/org/lib" = {}
"#,
        )
        .expect_err("missing source");
        match err {
            ManifestError::MissingReplaceSource { path } => {
                assert_eq!(path, "github.com/org/lib");
            }
            other => panic!("expected MissingReplaceSource, got {other:?}"),
        }
    }

    #[test]
    fn replace_reject_unknown_table_field() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
[replace]
"github.com/org/lib" = { extra = true }
"#,
        )
        .expect_err("unknown field");
        match err {
            ManifestError::UnknownField { field } => {
                assert_eq!(field, "extra");
            }
            other => panic!("expected UnknownField, got {other:?}"),
        }
    }

    #[test]
    fn replace_reject_empty_local_path() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
[replace]
"github.com/org/lib" = { path = "" }
"#,
        )
        .expect_err("empty path");
        assert!(
            matches!(err, ManifestError::InvalidReplaceSource { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn replace_reject_opaque_string() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
[replace]
"github.com/org/lib" = "latest"
"#,
        )
        .expect_err("opaque");
        assert!(
            matches!(err, ManifestError::InvalidReplaceSource { .. }),
            "{err:?}"
        );
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "draconic-pkg-replace-{tag}-{}-{}",
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

    fn tagged_repo(root: &Path, name: &str, tag: &str, body: &str) -> (PathBuf, String) {
        let repo = root.join(name);
        fs::create_dir_all(&repo).unwrap();
        git_ok(&["init"], &repo);
        git_ok(&["config", "user.email", "test@draconic.local"], &repo);
        git_ok(&["config", "user.name", "Draconic Test"], &repo);
        git_ok(&["checkout", "-B", "main"], &repo);
        fs::write(repo.join("lib.txt"), body).unwrap();
        git_ok(&["add", "lib.txt"], &repo);
        git_ok(&["commit", "-m", tag], &repo);
        git_ok(&["tag", tag], &repo);
        let out = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&repo)
            .output()
            .expect("rev-parse");
        assert!(out.status.success());
        let oid = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (repo, oid)
    }

    #[test]
    fn replace_resolve_fetch_uses_fork_git_not_original_pin() {
        let root = temp_dir("fork");
        let (original, oid_orig) = tagged_repo(&root, "original", "v1.0.0", "original\n");
        let (fork, oid_fork) = tagged_repo(&root, "fork", "v1.0.0", "forked\n");
        assert_ne!(oid_orig, oid_fork);

        let src = format!(
            r#"
module = "github.com/acme/app"

[dependencies]
"github.com/org/lib" = "1.0.0"

[urls]
"github.com/org/lib" = "{}"

[replace]
"github.com/org/lib" = {{ git = "{}" }}
"#,
            original.display(),
            fork.display()
        );
        let m = parse_ok(&src);
        assert_eq!(
            resolve_git_url(&m, "github.com/org/lib"),
            fork.to_str().unwrap()
        );

        let cache = ModuleCache::new(root.join("cache"));
        let lock = resolve_direct_deps(&m, &cache).expect("resolve with replace");
        let e = lock.packages.get("github.com/org/lib").expect("pin");
        assert_eq!(e.path, "github.com/org/lib");
        assert_eq!(e.version, "1.0.0");
        assert_eq!(e.git_url, fork.to_str().unwrap());
        assert_eq!(e.commit_oid, oid_fork);
        assert_ne!(e.commit_oid, oid_orig);

        let checkout = cache.entry_dir("github.com/org/lib", &oid_fork).unwrap();
        let body = fs::read_to_string(checkout.join("lib.txt")).expect("lib.txt");
        assert_eq!(body, "forked\n");
        assert_ne!(body, "original\n");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn replace_resolve_fetch_uses_local_path_not_original_pin() {
        let root = temp_dir("local");
        let (original, oid_orig) = tagged_repo(&root, "original", "v1.0.0", "from-url\n");
        let (local, oid_local) = tagged_repo(&root, "local", "v1.0.0", "from-local\n");
        assert_ne!(oid_orig, oid_local);

        let src = format!(
            r#"
module = "github.com/acme/app"

[dependencies]
"github.com/org/lib" = "1.0.0"

[urls]
"github.com/org/lib" = "{}"

[replace]
"github.com/org/lib" = {{ path = "{}" }}
"#,
            original.display(),
            local.display()
        );
        let m = parse_ok(&src);
        assert_eq!(
            resolve_git_url(&m, "github.com/org/lib"),
            local.to_str().unwrap()
        );

        let cache = ModuleCache::new(root.join("cache"));
        let lock = resolve_direct_deps(&m, &cache).expect("resolve local replace");
        let e = lock.packages.get("github.com/org/lib").expect("pin");
        assert_eq!(e.git_url, local.to_str().unwrap());
        assert_eq!(e.commit_oid, oid_local);
        assert_ne!(e.git_url, original.to_str().unwrap());

        let checkout = cache.entry_dir("github.com/org/lib", &oid_local).unwrap();
        let body = fs::read_to_string(checkout.join("lib.txt")).expect("lib.txt");
        assert_eq!(body, "from-local\n");

        let _ = fs::remove_dir_all(&root);
    }
}
