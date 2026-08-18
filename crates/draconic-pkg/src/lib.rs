//! Package manager support: `draconic.toml` manifests and related types (Roadmap K).
//!
//! K01.01: parse own module path + dependencies map (path → version req).
//! K01.02: write/round-trip `draconic.toml` with stable dependency order.
//! K01.03: schema validation (module paths, version reqs, unknown fields) + diagnostics.
//! K01.04: optional URL map (path → git URL); default derive `https://{module_path}.git`.
//! K02.01: lock entry — path + version + git URL + commit OID + content hash SHA-256.
//! K02.02: parse/write `draconic.lock`; reject malformed.
//! K02.03: stable lock serialize — sorted paths; byte-identical rewrite when unchanged.

mod lock;

pub use lock::{
    parse_lock, write_lock, LockEntry, LockEntryError, LockFile, LockFileError,
};

use std::collections::BTreeMap;
use std::fmt;

use toml::Value as TomlValue;

/// Known top-level keys in `draconic.toml` (K01.01–K01.04).
const KNOWN_TOP_LEVEL_KEYS: &[&str] = &["module", "dependencies", "urls"];

/// Parsed `draconic.toml` (K01: module path + deps + optional URL map).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// This package's module path (Go-like), e.g. `github.com/org/pkg`.
    pub module: String,
    /// Direct dependencies: module path → version requirement string.
    pub dependencies: BTreeMap<String, String>,
    /// Optional path → git URL overrides when default derivation is wrong (K01.04).
    pub urls: BTreeMap<String, String>,
}

/// Error while parsing or validating a `draconic.toml` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// Invalid TOML syntax.
    Toml(String),
    /// Document root is not a table.
    NotATable,
    /// Required top-level `module` string is missing.
    MissingModule,
    /// `module` is present but not a non-empty string.
    InvalidModule,
    /// Own `module` path fails Go-like module path schema.
    InvalidModulePath { path: String, reason: &'static str },
    /// `dependencies` is present but not a table of string → string.
    InvalidDependencies,
    /// A dependency entry has a non-string version requirement.
    InvalidDependencyValue { path: String },
    /// A dependency key fails Go-like module path schema.
    InvalidDependencyPath { path: String, reason: &'static str },
    /// A dependency version requirement is empty or not a semver-shaped req.
    InvalidVersionReq {
        path: String,
        req: String,
        reason: &'static str,
    },
    /// Unknown top-level field (not part of the manifest schema).
    UnknownField { field: String },
    /// Package lists itself as a dependency.
    SelfDependency { path: String },
    /// `urls` is present but not a table of string → string.
    InvalidUrls,
    /// A `urls` entry has a non-string git URL value.
    InvalidUrlValue { path: String },
    /// A `urls` key fails Go-like module path schema.
    InvalidUrlPath { path: String, reason: &'static str },
    /// A `urls` value is empty or not an acceptable git URL.
    InvalidUrl {
        path: String,
        url: String,
        reason: &'static str,
    },
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::Toml(msg) => write!(f, "invalid draconic.toml: {msg}"),
            ManifestError::NotATable => {
                write!(f, "draconic.toml: document root must be a table")
            }
            ManifestError::MissingModule => {
                write!(f, "draconic.toml: missing required field `module`")
            }
            ManifestError::InvalidModule => {
                write!(f, "draconic.toml: `module` must be a non-empty string")
            }
            ManifestError::InvalidModulePath { path, reason } => {
                write!(f, "draconic.toml: invalid module path `{path}`: {reason}")
            }
            ManifestError::InvalidDependencies => write!(
                f,
                "draconic.toml: `dependencies` must be a table of module path → version requirement strings"
            ),
            ManifestError::InvalidDependencyValue { path } => write!(
                f,
                "draconic.toml: dependency `{path}` version requirement must be a string"
            ),
            ManifestError::InvalidDependencyPath { path, reason } => write!(
                f,
                "draconic.toml: invalid dependency module path `{path}`: {reason}"
            ),
            ManifestError::InvalidVersionReq { path, req, reason } => write!(
                f,
                "draconic.toml: dependency `{path}` has invalid version requirement `{req}`: {reason}"
            ),
            ManifestError::UnknownField { field } => write!(
                f,
                "draconic.toml: unknown field `{field}` (expected one of: module, dependencies, urls)"
            ),
            ManifestError::SelfDependency { path } => write!(
                f,
                "draconic.toml: package cannot depend on itself (`{path}`)"
            ),
            ManifestError::InvalidUrls => write!(
                f,
                "draconic.toml: `urls` must be a table of module path → git URL strings"
            ),
            ManifestError::InvalidUrlValue { path } => write!(
                f,
                "draconic.toml: urls entry `{path}` git URL must be a string"
            ),
            ManifestError::InvalidUrlPath { path, reason } => write!(
                f,
                "draconic.toml: invalid urls module path `{path}`: {reason}"
            ),
            ManifestError::InvalidUrl { path, url, reason } => write!(
                f,
                "draconic.toml: urls entry `{path}` has invalid git URL `{url}`: {reason}"
            ),
        }
    }
}

impl std::error::Error for ManifestError {}

/// Parse a `draconic.toml` source string into a schema-valid [`Manifest`].
///
/// Expected shape (K01.01–K01.04):
/// ```toml
/// module = "github.com/org/pkg"
///
/// [dependencies]
/// "github.com/other/lib" = "1.2.3"
///
/// [urls]
/// "github.com/other/lib" = "https://git.example.com/other/lib.git"
/// ```
///
/// `dependencies` and `urls` may be omitted (empty maps). Performs structural
/// decode plus schema validation (module paths, version requirements, git URLs,
/// unknown fields).
pub fn parse_manifest(src: &str) -> Result<Manifest, ManifestError> {
    let value: TomlValue =
        toml::from_str(src).map_err(|e| ManifestError::Toml(e.to_string()))?;
    let table = match value {
        TomlValue::Table(t) => t,
        _ => return Err(ManifestError::NotATable),
    };

    for key in table.keys() {
        if !KNOWN_TOP_LEVEL_KEYS.contains(&key.as_str()) {
            return Err(ManifestError::UnknownField {
                field: key.clone(),
            });
        }
    }

    let module = match table.get("module") {
        None => return Err(ManifestError::MissingModule),
        Some(TomlValue::String(m)) if m.is_empty() => return Err(ManifestError::InvalidModule),
        Some(TomlValue::String(m)) => m.clone(),
        Some(_) => return Err(ManifestError::InvalidModule),
    };

    let dependencies = match table.get("dependencies") {
        None => BTreeMap::new(),
        Some(TomlValue::Table(dep_table)) => {
            let mut deps = BTreeMap::new();
            for (path, value) in dep_table {
                let req = match value {
                    TomlValue::String(s) => s.clone(),
                    _ => {
                        return Err(ManifestError::InvalidDependencyValue {
                            path: path.clone(),
                        });
                    }
                };
                deps.insert(path.clone(), req);
            }
            deps
        }
        Some(_) => return Err(ManifestError::InvalidDependencies),
    };

    let urls = match table.get("urls") {
        None => BTreeMap::new(),
        Some(TomlValue::Table(url_table)) => {
            let mut map = BTreeMap::new();
            for (path, value) in url_table {
                let url = match value {
                    TomlValue::String(s) => s.clone(),
                    _ => {
                        return Err(ManifestError::InvalidUrlValue {
                            path: path.clone(),
                        });
                    }
                };
                map.insert(path.clone(), url);
            }
            map
        }
        Some(_) => return Err(ManifestError::InvalidUrls),
    };

    let manifest = Manifest {
        module,
        dependencies,
        urls,
    };
    validate_manifest(&manifest)?;
    Ok(manifest)
}

/// Validate schema rules on an already-decoded [`Manifest`].
///
/// Checks Go-like module paths, semver-shaped version requirements, git URL
/// overrides, and rejects self-dependencies. Does not check unknown TOML fields
/// (those are only visible during [`parse_manifest`]).
pub fn validate_manifest(manifest: &Manifest) -> Result<(), ManifestError> {
    if let Err(reason) = validate_module_path(&manifest.module) {
        return Err(ManifestError::InvalidModulePath {
            path: manifest.module.clone(),
            reason,
        });
    }

    for (path, req) in &manifest.dependencies {
        if path == &manifest.module {
            return Err(ManifestError::SelfDependency {
                path: path.clone(),
            });
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

    Ok(())
}

/// Default git URL for a module path: `https://{module_path}.git` (K01.04 / ADR-0009).
pub fn default_git_url(module_path: &str) -> String {
    format!("https://{module_path}.git")
}

/// Resolve the git URL for `module_path`: `[urls]` override if present, else
/// [`default_git_url`].
pub fn resolve_git_url(manifest: &Manifest, module_path: &str) -> String {
    manifest
        .urls
        .get(module_path)
        .cloned()
        .unwrap_or_else(|| default_git_url(module_path))
}

/// Accept https (or git/ssh-style) clone URLs used as path→URL overrides.
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

    Err("must start with https://, http://, git@, ssh://, or git://")
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
        return Err(
            "must contain at least two path segments (e.g. github.com/org/pkg)",
        );
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
            return Err(
                "segments may only contain ASCII letters, digits, '.', '-', '_'",
            );
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
    s.split('.').all(|part| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
    })
}

/// Serialize a [`Manifest`] to a stable `draconic.toml` document.
///
/// Emit shape (K01.02 / K01.04):
/// - `module = "…"` first
/// - blank line then `[dependencies]` only when non-empty
/// - dependency keys in sorted (BTreeMap) order, each quoted
/// - blank line then `[urls]` only when non-empty (sorted keys)
/// - trailing newline
///
/// Round-trip: `parse_manifest(&write_manifest(m)) == Ok(m)` (equal after parse).
/// Rewrite is byte-identical: `write_manifest(&parse_manifest(write(m))?) == write(m)`.
pub fn write_manifest(manifest: &Manifest) -> String {
    let mut out = String::new();
    out.push_str("module = ");
    out.push_str(&toml_quoted_string(&manifest.module));
    out.push('\n');

    if !manifest.dependencies.is_empty() {
        out.push('\n');
        out.push_str("[dependencies]\n");
        for (path, req) in &manifest.dependencies {
            out.push_str(&toml_quoted_string(path));
            out.push_str(" = ");
            out.push_str(&toml_quoted_string(req));
            out.push('\n');
        }
    }

    if !manifest.urls.is_empty() {
        out.push('\n');
        out.push_str("[urls]\n");
        for (path, url) in &manifest.urls {
            out.push_str(&toml_quoted_string(path));
            out.push_str(" = ");
            out.push_str(&toml_quoted_string(url));
            out.push('\n');
        }
    }

    out
}

/// Quote a string as a TOML basic string (escape `\`, `"`, and control chars).
fn toml_quoted_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                // Other controls as TOML \uXXXX
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(module: &str, deps: &[(&str, &str)]) -> Manifest {
        manifest_with_urls(module, deps, &[])
    }

    fn manifest_with_urls(
        module: &str,
        deps: &[(&str, &str)],
        urls: &[(&str, &str)],
    ) -> Manifest {
        Manifest {
            module: module.to_string(),
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

    #[test]
    fn parse_module_only() {
        let m = parse_manifest(
            r#"
module = "github.com/org/pkg"
"#,
        )
        .expect("parse");
        assert_eq!(m.module, "github.com/org/pkg");
        assert!(m.dependencies.is_empty());
        assert!(m.urls.is_empty());
    }

    #[test]
    fn parse_module_and_deps() {
        let m = parse_manifest(
            r#"
module = "github.com/acme/app"

[dependencies]
"github.com/org/lib" = "1.2.3"
"github.com/other/util" = "^2.0"
"#,
        )
        .expect("parse");
        assert_eq!(m.module, "github.com/acme/app");
        assert_eq!(m.dependencies.len(), 2);
        assert_eq!(
            m.dependencies.get("github.com/org/lib").map(String::as_str),
            Some("1.2.3")
        );
        assert_eq!(
            m.dependencies
                .get("github.com/other/util")
                .map(String::as_str),
            Some("^2.0")
        );
        assert!(m.urls.is_empty());
    }

    #[test]
    fn parse_empty_dependencies_table() {
        let m = parse_manifest(
            r#"
module = "github.com/org/pkg"
[dependencies]
"#,
        )
        .expect("parse");
        assert_eq!(m.module, "github.com/org/pkg");
        assert!(m.dependencies.is_empty());
    }

    #[test]
    fn reject_invalid_toml() {
        let err = parse_manifest("module = [").expect_err("invalid toml");
        assert!(matches!(err, ManifestError::Toml(_)), "{err:?}");
    }

    #[test]
    fn reject_missing_module() {
        let err = parse_manifest(
            r#"
[dependencies]
"github.com/org/lib" = "1.0.0"
"#,
        )
        .expect_err("missing module");
        assert_eq!(err, ManifestError::MissingModule);
    }

    #[test]
    fn reject_empty_module() {
        let err = parse_manifest(r#"module = """#).expect_err("empty module");
        assert_eq!(err, ManifestError::InvalidModule);
    }

    #[test]
    fn reject_module_wrong_type() {
        let err = parse_manifest("module = 42").expect_err("wrong type");
        assert_eq!(err, ManifestError::InvalidModule);
    }

    #[test]
    fn reject_dependencies_not_table() {
        let err = parse_manifest(
            r#"
module = "github.com/org/pkg"
dependencies = "nope"
"#,
        )
        .expect_err("deps not table");
        assert_eq!(err, ManifestError::InvalidDependencies);
    }

    #[test]
    fn reject_dependency_value_not_string() {
        let err = parse_manifest(
            r#"
module = "github.com/org/pkg"
[dependencies]
"github.com/org/lib" = 123
"#,
        )
        .expect_err("dep value not string");
        match err {
            ManifestError::InvalidDependencyValue { path } => {
                assert_eq!(path, "github.com/org/lib");
            }
            other => panic!("expected InvalidDependencyValue, got {other:?}"),
        }
    }

    // --- K01.02: write / round-trip ---

    #[test]
    fn write_module_only() {
        let m = manifest("github.com/org/pkg", &[]);
        assert_eq!(write_manifest(&m), "module = \"github.com/org/pkg\"\n");
    }

    #[test]
    fn write_module_and_deps_sorted() {
        // Insert out of order; emit must be sorted by path.
        let m = manifest(
            "github.com/acme/app",
            &[
                ("github.com/z/last", "3.0.0"),
                ("github.com/a/first", "1.0.0"),
                ("github.com/m/mid", "^2.0"),
            ],
        );
        let expected = "\
module = \"github.com/acme/app\"

[dependencies]
\"github.com/a/first\" = \"1.0.0\"
\"github.com/m/mid\" = \"^2.0\"
\"github.com/z/last\" = \"3.0.0\"
";
        assert_eq!(write_manifest(&m), expected);
    }

    #[test]
    fn write_omits_empty_dependencies_table() {
        let m = manifest("github.com/org/pkg", &[]);
        let s = write_manifest(&m);
        assert!(!s.contains("[dependencies]"), "{s}");
        assert!(s.ends_with('\n'));
    }

    #[test]
    fn round_trip_parse_write_eq() {
        let original = manifest(
            "github.com/acme/app",
            &[
                ("github.com/org/lib", "1.2.3"),
                ("github.com/other/util", "^2.0"),
            ],
        );
        let written = write_manifest(&original);
        let parsed = parse_manifest(&written).expect("parse written");
        assert_eq!(parsed, original);
    }

    #[test]
    fn round_trip_module_only() {
        let original = manifest("github.com/org/pkg", &[]);
        let written = write_manifest(&original);
        let parsed = parse_manifest(&written).expect("parse written");
        assert_eq!(parsed, original);
    }

    #[test]
    fn rewrite_is_byte_identical() {
        let m = manifest(
            "github.com/acme/app",
            &[
                ("github.com/z/last", "3.0.0"),
                ("github.com/a/first", "1.0.0"),
            ],
        );
        let once = write_manifest(&m);
        let twice = write_manifest(&parse_manifest(&once).expect("parse"));
        assert_eq!(once, twice);
    }

    #[test]
    fn write_escapes_quotes_in_module() {
        // Constructed paths need not pass schema (write is serialization only).
        let m = manifest(r#"org/pkg"with"quotes"#, &[]);
        let s = write_manifest(&m);
        assert_eq!(s, "module = \"org/pkg\\\"with\\\"quotes\"\n");
        // Re-parse will fail schema (quotes invalid in module path) — round-trip
        // of schema-valid manifests is covered above.
        assert!(parse_manifest(&s).is_err());
    }

    // --- K01.03: schema validation + diagnostics ---

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
        assert!(matches!(err, ManifestError::InvalidModulePath { .. }), "{err:?}");
    }

    #[test]
    fn reject_module_path_trailing_slash() {
        let err =
            parse_manifest(r#"module = "github.com/org/pkg/""#).expect_err("trailing slash");
        assert!(matches!(err, ManifestError::InvalidModulePath { .. }), "{err:?}");
    }

    #[test]
    fn reject_module_path_empty_segment() {
        let err =
            parse_manifest(r#"module = "github.com//pkg""#).expect_err("empty segment");
        assert!(matches!(err, ManifestError::InvalidModulePath { .. }), "{err:?}");
    }

    #[test]
    fn reject_module_path_dot_segment() {
        let err =
            parse_manifest(r#"module = "github.com/org/../evil""#).expect_err("dotdot");
        assert!(matches!(err, ManifestError::InvalidModulePath { .. }), "{err:?}");
    }

    #[test]
    fn reject_module_path_whitespace() {
        let err =
            parse_manifest(r#"module = "github.com/org/my pkg""#).expect_err("whitespace");
        assert!(matches!(err, ManifestError::InvalidModulePath { .. }), "{err:?}");
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
    fn reject_unknown_top_level_field() {
        let err = parse_manifest(
            r#"
module = "github.com/acme/app"
license = "MIT"
"#,
        )
        .expect_err("unknown field");
        match &err {
            ManifestError::UnknownField { field } => assert_eq!(field, "license"),
            other => panic!("expected UnknownField, got {other:?}"),
        }
        assert!(
            err.to_string().contains("unknown") || err.to_string().contains("license"),
            "diagnostic should name the field: {err}"
        );
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
        };
        let err = validate_manifest(&m).expect_err("should fail schema");
        assert!(matches!(err, ManifestError::InvalidModulePath { .. }), "{err:?}");
    }

    #[test]
    fn validate_manifest_accepts_good() {
        let m = manifest(
            "github.com/acme/app",
            &[("github.com/org/lib", "^1.2.3")],
        );
        validate_manifest(&m).expect("valid");
    }

    #[test]
    fn module_wrong_type_is_invalid_module_not_opaque_toml() {
        let err = parse_manifest("module = 42").expect_err("wrong type");
        assert_eq!(err, ManifestError::InvalidModule);
        assert!(
            err.to_string().contains("module"),
            "clear diagnostic: {err}"
        );
    }

    // --- K01.04: optional URL map + default derive ---

    #[test]
    fn default_git_url_derives_https_module_path_git() {
        assert_eq!(
            default_git_url("github.com/org/pkg"),
            "https://github.com/org/pkg.git"
        );
        assert_eq!(
            default_git_url("gitlab.com/group/sub/mod"),
            "https://gitlab.com/group/sub/mod.git"
        );
    }

    #[test]
    fn resolve_git_url_uses_default_when_urls_empty() {
        let m = manifest("github.com/acme/app", &[("github.com/org/lib", "1.0.0")]);
        assert_eq!(
            resolve_git_url(&m, "github.com/org/lib"),
            "https://github.com/org/lib.git"
        );
        assert_eq!(
            resolve_git_url(&m, "github.com/other/util"),
            "https://github.com/other/util.git"
        );
    }

    #[test]
    fn resolve_git_url_prefers_urls_map_override() {
        let m = manifest_with_urls(
            "github.com/acme/app",
            &[("github.com/org/lib", "1.0.0")],
            &[(
                "github.com/org/lib",
                "https://git.example.com/mirror/lib.git",
            )],
        );
        assert_eq!(
            resolve_git_url(&m, "github.com/org/lib"),
            "https://git.example.com/mirror/lib.git"
        );
        // Unmapped path still derives default.
        assert_eq!(
            resolve_git_url(&m, "github.com/other/util"),
            "https://github.com/other/util.git"
        );
    }

    #[test]
    fn parse_urls_table() {
        let m = parse_manifest(
            r#"
module = "github.com/acme/app"

[dependencies]
"github.com/org/lib" = "1.2.3"

[urls]
"github.com/org/lib" = "https://git.example.com/org/lib.git"
"github.com/private/tool" = "git@github.com:private/tool.git"
"#,
        )
        .expect("parse");
        assert_eq!(m.urls.len(), 2);
        assert_eq!(
            m.urls.get("github.com/org/lib").map(String::as_str),
            Some("https://git.example.com/org/lib.git")
        );
        assert_eq!(
            m.urls
                .get("github.com/private/tool")
                .map(String::as_str),
            Some("git@github.com:private/tool.git")
        );
        assert_eq!(
            resolve_git_url(&m, "github.com/org/lib"),
            "https://git.example.com/org/lib.git"
        );
    }

    #[test]
    fn parse_empty_urls_table() {
        let m = parse_manifest(
            r#"
module = "github.com/org/pkg"
[urls]
"#,
        )
        .expect("parse");
        assert!(m.urls.is_empty());
    }

    #[test]
    fn write_urls_sorted() {
        let m = manifest_with_urls(
            "github.com/acme/app",
            &[],
            &[
                ("github.com/z/last", "https://z.example/last.git"),
                ("github.com/a/first", "https://a.example/first.git"),
            ],
        );
        let expected = "\
module = \"github.com/acme/app\"

[urls]
\"github.com/a/first\" = \"https://a.example/first.git\"
\"github.com/z/last\" = \"https://z.example/last.git\"
";
        assert_eq!(write_manifest(&m), expected);
    }

    #[test]
    fn write_deps_then_urls() {
        let m = manifest_with_urls(
            "github.com/acme/app",
            &[("github.com/org/lib", "1.0.0")],
            &[("github.com/org/lib", "https://mirror.example/lib.git")],
        );
        let expected = "\
module = \"github.com/acme/app\"

[dependencies]
\"github.com/org/lib\" = \"1.0.0\"

[urls]
\"github.com/org/lib\" = \"https://mirror.example/lib.git\"
";
        assert_eq!(write_manifest(&m), expected);
    }

    #[test]
    fn write_omits_empty_urls_table() {
        let m = manifest("github.com/org/pkg", &[]);
        let s = write_manifest(&m);
        assert!(!s.contains("[urls]"), "{s}");
    }

    #[test]
    fn round_trip_with_urls() {
        let original = manifest_with_urls(
            "github.com/acme/app",
            &[("github.com/org/lib", "1.2.3")],
            &[("github.com/org/lib", "https://git.example.com/lib.git")],
        );
        let written = write_manifest(&original);
        let parsed = parse_manifest(&written).expect("parse written");
        assert_eq!(parsed, original);
        let twice = write_manifest(&parsed);
        assert_eq!(written, twice);
    }

    #[test]
    fn reject_urls_not_table() {
        let err = parse_manifest(
            r#"
module = "github.com/org/pkg"
urls = "nope"
"#,
        )
        .expect_err("urls not table");
        assert_eq!(err, ManifestError::InvalidUrls);
    }

    #[test]
    fn reject_url_value_not_string() {
        let err = parse_manifest(
            r#"
module = "github.com/org/pkg"
[urls]
"github.com/org/lib" = 123
"#,
        )
        .expect_err("url value not string");
        match err {
            ManifestError::InvalidUrlValue { path } => {
                assert_eq!(path, "github.com/org/lib");
            }
            other => panic!("expected InvalidUrlValue, got {other:?}"),
        }
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
        assert!(
            matches!(err, ManifestError::InvalidUrl { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn accept_common_git_url_forms() {
        let cases = [
            "https://github.com/org/lib.git",
            "http://git.example.com/org/lib.git",
            "git@github.com:org/lib.git",
            "ssh://git@github.com/org/lib.git",
            "git://github.com/org/lib.git",
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
