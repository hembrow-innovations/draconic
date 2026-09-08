//! Schema validation for `draconic.toml` (K01.03).

use crate::replace;
use crate::{Manifest, ManifestError};

/// Validate schema rules on an already-decoded [`Manifest`].
///
/// Checks Go-like module paths, semver-shaped version requirements, git URL
/// overrides, and rejects self-dependencies. Does not check unknown TOML fields
/// (those are only visible during [`crate::parse_manifest`]).
pub fn validate_manifest(manifest: &Manifest) -> Result<(), ManifestError> {
    if let Err(reason) = validate_module_path(&manifest.module) {
        return Err(ManifestError::InvalidModulePath {
            path: manifest.module.clone(),
            reason,
        });
    }

    for (path, req) in &manifest.dependencies {
        if path == &manifest.module {
            return Err(ManifestError::SelfDependency { path: path.clone() });
        }
        if let Err(reason) = validate_module_path(path) {
            return Err(ManifestError::InvalidDependencyPath {
                path: path.clone(),
                reason,
            });
        }
        if let Err(reason) = validate_version_req(req) {
            return Err(ManifestError::InvalidVersionReq {
                path: path.clone(),
                req: req.clone(),
                reason,
            });
        }
    }

    for (path, url) in &manifest.urls {
        if let Err(reason) = validate_module_path(path) {
            return Err(ManifestError::InvalidUrlPath {
                path: path.clone(),
                reason,
            });
        }
        if let Err(reason) = validate_git_url(url) {
            return Err(ManifestError::InvalidUrl {
                path: path.clone(),
                url: url.clone(),
                reason,
            });
        }
    }

    replace::validate_replace(&manifest.replace)?;

    if let Some(pin) = &manifest.toolchain {
        if let Err(reason) = validate_version_req(&pin.version) {
            return Err(ManifestError::InvalidToolchainVersion {
                version: pin.version.clone(),
                reason,
            });
        }
    }

    Ok(())
}

/// Accept https (or git/ssh-style) clone URLs used as path→URL overrides.
///
/// Also accepts `file://` and absolute local paths so lock pins can record
/// fixture/cache clone URLs used by K03/K04 tests and local path deps.
pub(crate) fn validate_git_url(url: &str) -> Result<(), &'static str> {
    if url.is_empty() {
        return Err("must not be empty");
    }
    if url != url.trim() {
        return Err("must not have leading or trailing whitespace");
    }
    if url.chars().any(|c| c.is_whitespace()) {
        return Err("must not contain whitespace");
    }

    if let Some(rest) = url.strip_prefix("https://") {
        if rest.is_empty() || !rest.contains('.') {
            return Err("https URL must include a host");
        }
        return Ok(());
    }
    if let Some(rest) = url.strip_prefix("http://") {
        if rest.is_empty() || !rest.contains('.') {
            return Err("http URL must include a host");
        }
        return Ok(());
    }
    if let Some(rest) = url.strip_prefix("file://") {
        if rest.is_empty() {
            return Err("file URL must include a path");
        }
        return Ok(());
    }
    if let Some(rest) = url.strip_prefix("git@") {
        // git@host:path
        if !rest.contains(':') || !rest.contains('.') {
            return Err("ssh git URL must look like git@host:path");
        }
        return Ok(());
    }
    if let Some(rest) = url.strip_prefix("ssh://") {
        if rest.is_empty() {
            return Err("ssh URL must include a host");
        }
        return Ok(());
    }
    if let Some(rest) = url.strip_prefix("git://") {
        if rest.is_empty() || !rest.contains('.') {
            return Err("git URL must include a host");
        }
        return Ok(());
    }
    // Absolute local path (fixture repos / path deps).
    if std::path::Path::new(url).is_absolute() {
        return Ok(());
    }

    Err("must start with https://, http://, file://, git@, ssh://, git://, or be an absolute path")
}

/// Go-like module path: `host.tld/path…` with no empty/`.`/`..` segments.
///
/// Rules (v1):
/// - non-empty, no leading/trailing whitespace
/// - no leading/trailing `/`, no `//`
/// - at least two `/`-separated segments
/// - first segment looks like a domain (contains `.`)
/// - segments are non-empty and not `.` / `..`
/// - ASCII alphanumeric plus `.` `-` `_` only in segments
pub(crate) fn validate_module_path(path: &str) -> Result<(), &'static str> {
    if path.is_empty() {
        return Err("must not be empty");
    }
    if path != path.trim() {
        return Err("must not have leading or trailing whitespace");
    }
    if path.starts_with('/') || path.ends_with('/') {
        return Err("must not start or end with '/'");
    }
    if path.contains("//") {
        return Err("must not contain empty path segments");
    }
    if path.chars().any(|c| c.is_whitespace()) {
        return Err("must not contain whitespace");
    }

    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() < 2 {
        return Err("must contain at least two path segments (e.g. github.com/org/pkg)");
    }

    let host = segments[0];
    if !host.contains('.') {
        return Err("first path segment must look like a domain (contain '.')");
    }

    for seg in &segments {
        if *seg == "." || *seg == ".." {
            return Err("must not contain '.' or '..' path segments");
        }
        if seg.is_empty() {
            return Err("must not contain empty path segments");
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

/// Semver-shaped version requirement (exact or simple range). Full tag resolve is K04.
///
/// Accepted forms:
/// - optional operator: `^` `~` `>=` `<=` `>` `<` `=`
/// - optional leading `v`
/// - `MAJOR.MINOR.PATCH` with optional `-prerelease` and/or `+build`
/// - also `MAJOR.MINOR` or `MAJOR` (partial)
pub(crate) fn validate_version_req(req: &str) -> Result<(), &'static str> {
    if req.is_empty() {
        return Err("must not be empty");
    }
    if req != req.trim() {
        return Err("must not have leading or trailing whitespace");
    }

    let rest = strip_version_operator(req);
    if rest.is_empty() {
        return Err("missing version after operator");
    }

    let rest = rest.strip_prefix('v').unwrap_or(rest);
    if rest.is_empty() {
        return Err("missing version number");
    }

    // Split build metadata first (+…), then prerelease (-…).
    let (core_and_pre, _build) = match rest.split_once('+') {
        Some((left, build)) => {
            if build.is_empty() || !is_semver_ident_chain(build) {
                return Err("invalid build metadata");
            }
            (left, Some(build))
        }
        None => (rest, None),
    };

    let (core, pre) = match core_and_pre.split_once('-') {
        Some((left, pre)) => {
            if pre.is_empty() || !is_semver_ident_chain(pre) {
                return Err("invalid prerelease identifier");
            }
            (left, Some(pre))
        }
        None => (core_and_pre, None),
    };

    if core.is_empty() {
        return Err("missing numeric version core");
    }

    let parts: Vec<&str> = core.split('.').collect();
    if parts.is_empty() || parts.len() > 3 {
        return Err("version core must be MAJOR[.MINOR[.PATCH]]");
    }
    for part in &parts {
        if part.is_empty() {
            return Err("version core has an empty numeric component");
        }
        if !part.chars().all(|c| c.is_ascii_digit()) {
            return Err("version core components must be decimal digits");
        }
        // Disallow leading zeros except plain "0".
        if part.len() > 1 && part.starts_with('0') {
            return Err("version core components must not have leading zeros");
        }
    }

    // Prerelease/build only make sense with a full-ish version; allow with any core.
    let _ = pre;

    Ok(())
}

fn strip_version_operator(req: &str) -> &str {
    if let Some(rest) = req.strip_prefix(">=") {
        rest
    } else if let Some(rest) = req.strip_prefix("<=") {
        rest
    } else if let Some(rest) = req.strip_prefix('>') {
        rest
    } else if let Some(rest) = req.strip_prefix('<') {
        rest
    } else if let Some(rest) = req.strip_prefix('^') {
        rest
    } else if let Some(rest) = req.strip_prefix('~') {
        rest
    } else if let Some(rest) = req.strip_prefix('=') {
        rest
    } else {
        req
    }
}

/// Dot-separated identifiers: alphanumeric and hyphen, non-empty parts (semver).
fn is_semver_ident_chain(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.split('.')
        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::parse_manifest;

    fn manifest(module: &str, deps: &[(&str, &str)]) -> Manifest {
        Manifest {
            module: module.to_string(),
            dependencies: deps
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            urls: BTreeMap::new(),
            replace: BTreeMap::new(),
            toolchain: None,
        }
    }

    #[test]
    fn reject_module_path_no_slash() {
        let err = parse_manifest(r#"module = "lonely""#).expect_err("no slash");
        match &err {
            ManifestError::InvalidModulePath { path, reason } => {
                assert_eq!(path, "lonely");
                assert!(!reason.is_empty(), "{reason}");
            }
            other => panic!("expected InvalidModulePath, got {other:?}"),
        }
        assert!(
            err.to_string().contains("module path"),
            "diagnostic should mention module path: {err}"
        );
    }

    #[test]
    fn reject_module_path_no_domain_dot() {
        let err = parse_manifest(r#"module = "localhost/pkg""#).expect_err("no domain dot");
        match err {
            ManifestError::InvalidModulePath { path, .. } => {
                assert_eq!(path, "localhost/pkg");
            }
            other => panic!("expected InvalidModulePath, got {other:?}"),
        }
    }

    #[test]
    fn reject_module_path_leading_slash() {
        let err = parse_manifest(r#"module = "/github.com/org/pkg""#).expect_err("leading slash");
        assert!(
            matches!(err, ManifestError::InvalidModulePath { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn reject_module_path_trailing_slash() {
        let err = parse_manifest(r#"module = "github.com/org/pkg/""#).expect_err("trailing slash");
        assert!(
            matches!(err, ManifestError::InvalidModulePath { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn reject_module_path_empty_segment() {
        let err = parse_manifest(r#"module = "github.com//pkg""#).expect_err("empty segment");
        assert!(
            matches!(err, ManifestError::InvalidModulePath { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn reject_module_path_dot_segment() {
        let err = parse_manifest(r#"module = "github.com/org/../evil""#).expect_err("dotdot");
        assert!(
            matches!(err, ManifestError::InvalidModulePath { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn reject_module_path_whitespace() {
        let err = parse_manifest(r#"module = "github.com/org/my pkg""#).expect_err("whitespace");
        assert!(
            matches!(err, ManifestError::InvalidModulePath { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn reject_dependency_path_invalid() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
[dependencies]
"not-a-path" = "1.0.0"
"#,
        )
        .expect_err("bad dep path");
        match &err {
            ManifestError::InvalidDependencyPath { path, reason } => {
                assert_eq!(path, "not-a-path");
                assert!(!reason.is_empty());
            }
            other => panic!("expected InvalidDependencyPath, got {other:?}"),
        }
        assert!(
            err.to_string().contains("dependency"),
            "diagnostic should mention dependency: {err}"
        );
    }

    #[test]
    fn reject_empty_version_req() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
[dependencies]
"github.com/org/lib" = ""
"#,
        )
        .expect_err("empty version");
        match &err {
            ManifestError::InvalidVersionReq { path, req, reason } => {
                assert_eq!(path, "github.com/org/lib");
                assert_eq!(req, "");
                assert!(!reason.is_empty());
            }
            other => panic!("expected InvalidVersionReq, got {other:?}"),
        }
        assert!(
            err.to_string().contains("version"),
            "diagnostic should mention version: {err}"
        );
    }

    #[test]
    fn reject_non_semver_version_req() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
[dependencies]
"github.com/org/lib" = "latest"
"#,
        )
        .expect_err("latest not semver");
        assert!(
            matches!(err, ManifestError::InvalidVersionReq { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn reject_branch_name_version_req() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
[dependencies]
"github.com/org/lib" = "main"
"#,
        )
        .expect_err("branch not semver");
        assert!(
            matches!(err, ManifestError::InvalidVersionReq { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn accept_common_version_req_forms() {
        let cases = [
            "1.2.3",
            "v1.2.3",
            "^1.2.3",
            "~1.0.0",
            ">=1.0.0",
            "<=2.0.0",
            ">0.1.0",
            "<3.0.0",
            "1.2.3-alpha.1",
            "1.0.0+build.7",
        ];
        for req in cases {
            let src = format!(
                r#"
module = "github.com/acme/app"
[dependencies]
"github.com/org/lib" = "{req}"
"#
            );
            let m = parse_manifest(&src).unwrap_or_else(|e| panic!("req {req:?}: {e}"));
            assert_eq!(
                m.dependencies.get("github.com/org/lib").map(String::as_str),
                Some(req)
            );
        }
    }

    #[test]
    fn reject_self_dependency() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
[dependencies]
"github.com/acme/app" = "1.0.0"
"#,
        )
        .expect_err("self dep");
        match err {
            ManifestError::SelfDependency { path } => {
                assert_eq!(path, "github.com/acme/app");
            }
            other => panic!("expected SelfDependency, got {other:?}"),
        }
    }

    #[test]
    fn validate_manifest_rejects_bad_constructed() {
        let m = Manifest {
            module: "not-valid".into(),
            dependencies: BTreeMap::new(),
            urls: BTreeMap::new(),
            replace: BTreeMap::new(),
            toolchain: None,
        };
        let err = validate_manifest(&m).expect_err("should fail schema");
        assert!(
            matches!(err, ManifestError::InvalidModulePath { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn validate_manifest_accepts_good() {
        let m = manifest("github.com/acme/app", &[("github.com/org/lib", "^1.2.3")]);
        validate_manifest(&m).expect("valid");
    }

    #[test]
    fn reject_url_path_invalid() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
[urls]
"not-a-path" = "https://example.com/x.git"
"#,
        )
        .expect_err("bad url path");
        match &err {
            ManifestError::InvalidUrlPath { path, reason } => {
                assert_eq!(path, "not-a-path");
                assert!(!reason.is_empty());
            }
            other => panic!("expected InvalidUrlPath, got {other:?}"),
        }
    }

    #[test]
    fn reject_empty_url() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
[urls]
"github.com/org/lib" = ""
"#,
        )
        .expect_err("empty url");
        match &err {
            ManifestError::InvalidUrl { path, url, reason } => {
                assert_eq!(path, "github.com/org/lib");
                assert_eq!(url, "");
                assert!(!reason.is_empty());
            }
            other => panic!("expected InvalidUrl, got {other:?}"),
        }
        assert!(
            err.to_string().contains("git URL") || err.to_string().contains("urls"),
            "diagnostic: {err}"
        );
    }

    #[test]
    fn reject_non_git_url_scheme() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
[urls]
"github.com/org/lib" = "ftp://example.com/lib"
"#,
        )
        .expect_err("ftp not allowed");
        assert!(matches!(err, ManifestError::InvalidUrl { .. }), "{err:?}");
    }

    #[test]
    fn accept_common_git_url_forms() {
        let cases = [
            "https://github.com/org/lib.git",
            "http://git.example.com/org/lib.git",
            "git@github.com:org/lib.git",
            "ssh://git@github.com/org/lib.git",
            "git://github.com/org/lib.git",
            "file:///tmp/fixture-lib.git",
        ];
        for url in cases {
            let src = format!(
                r#"
module = "github.com/acme/app"
[urls]
"github.com/org/lib" = "{url}"
"#
            );
            let m = parse_manifest(&src).unwrap_or_else(|e| panic!("url {url:?}: {e}"));
            assert_eq!(
                m.urls.get("github.com/org/lib").map(String::as_str),
                Some(url)
            );
        }
    }
}
