//! Lockfile types for `draconic.lock` (Roadmap K02).
//!
//! K02: a Program's package graph pins resolved deps in `draconic.lock`.
//! K02.01: lock entry — path + version + git URL + commit OID + content hash SHA-256.
//! K02.02: parse/write the lock document; reject malformed input.
//! K02.03: stable serialize — packages sorted by path; rewrite of unchanged
//! lock is byte-identical.

use std::collections::BTreeMap;
use std::fmt;

use toml::Value as TomlValue;

use crate::{validate_git_url, validate_module_path, validate_package_subdir, validate_version_req};

/// One pinned dependency in `draconic.lock` (K02.01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockEntry {
    /// Module path (Go-like), e.g. `github.com/org/pkg`.
    pub path: String,
    /// Resolved version string (semver tag without operator), e.g. `1.2.3`.
    pub version: String,
    /// Git remote used to fetch this pin.
    pub git_url: String,
    /// Full commit object id (40 lowercase hex SHA-1).
    pub commit_oid: String,
    /// SHA-256 hex (64 lowercase) of the canonical package tree.
    pub content_hash: String,
    /// Subdirectory inside the git tree that is this package's root (K11.03).
    /// Empty = repository root (single-module checkout).
    pub subdir: String,
}

/// Error while constructing or validating a [`LockEntry`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockEntryError {
    /// Module path fails Go-like schema.
    InvalidPath { path: String, reason: &'static str },
    /// Version string is empty or not a concrete semver (no range operators).
    InvalidVersion {
        version: String,
        reason: &'static str,
    },
    /// Git URL is empty or not an acceptable clone URL.
    InvalidGitUrl { url: String, reason: &'static str },
    /// Commit OID is not a full 40-char lowercase hex SHA-1.
    InvalidCommitOid { oid: String, reason: &'static str },
    /// Content hash is not a 64-char lowercase hex SHA-256 digest.
    InvalidContentHash { hash: String, reason: &'static str },
    /// Package subdirectory is not a safe relative path (K11.03).
    InvalidSubdir { subdir: String, reason: &'static str },
}

impl fmt::Display for LockEntryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockEntryError::InvalidPath { path, reason } => {
                write!(f, "lock entry: invalid module path `{path}`: {reason}")
            }
            LockEntryError::InvalidVersion { version, reason } => {
                write!(f, "lock entry: invalid version `{version}`: {reason}")
            }
            LockEntryError::InvalidGitUrl { url, reason } => {
                write!(f, "lock entry: invalid git URL `{url}`: {reason}")
            }
            LockEntryError::InvalidCommitOid { oid, reason } => {
                write!(f, "lock entry: invalid commit OID `{oid}`: {reason}")
            }
            LockEntryError::InvalidContentHash { hash, reason } => {
                write!(f, "lock entry: invalid content hash `{hash}`: {reason}")
            }
            LockEntryError::InvalidSubdir { subdir, reason } => {
                write!(f, "lock entry: invalid subdir `{subdir}`: {reason}")
            }
        }
    }
}

impl std::error::Error for LockEntryError {}

impl LockEntry {
    /// Build a validated lock entry (K02.01 field set).
    pub fn new(
        path: impl Into<String>,
        version: impl Into<String>,
        git_url: impl Into<String>,
        commit_oid: impl Into<String>,
        content_hash: impl Into<String>,
    ) -> Result<Self, LockEntryError> {
        let entry = Self {
            path: path.into(),
            version: version.into(),
            git_url: git_url.into(),
            commit_oid: commit_oid.into(),
            content_hash: content_hash.into(),
            subdir: String::new(),
        };
        entry.validate()?;
        Ok(entry)
    }

    /// Set the git-tree subdirectory that is this package's root (K11.03).
    pub fn with_subdir(mut self, subdir: impl Into<String>) -> Result<Self, LockEntryError> {
        self.subdir = subdir.into();
        self.validate()?;
        Ok(self)
    }

    /// Validate all fields of this entry.
    pub fn validate(&self) -> Result<(), LockEntryError> {
        if let Err(reason) = validate_module_path(&self.path) {
            return Err(LockEntryError::InvalidPath {
                path: self.path.clone(),
                reason,
            });
        }
        if let Err(reason) = validate_lock_version(&self.version) {
            return Err(LockEntryError::InvalidVersion {
                version: self.version.clone(),
                reason,
            });
        }
        if let Err(reason) = validate_git_url(&self.git_url) {
            return Err(LockEntryError::InvalidGitUrl {
                url: self.git_url.clone(),
                reason,
            });
        }
        if let Err(reason) = validate_commit_oid(&self.commit_oid) {
            return Err(LockEntryError::InvalidCommitOid {
                oid: self.commit_oid.clone(),
                reason,
            });
        }
        if let Err(reason) = validate_content_hash(&self.content_hash) {
            return Err(LockEntryError::InvalidContentHash {
                hash: self.content_hash.clone(),
                reason,
            });
        }
        if let Err(reason) = validate_package_subdir(&self.subdir) {
            return Err(LockEntryError::InvalidSubdir {
                subdir: self.subdir.clone(),
                reason,
            });
        }
        Ok(())
    }
}

/// Locked version must be a concrete semver (no range operators).
fn validate_lock_version(version: &str) -> Result<(), &'static str> {
    if version.is_empty() {
        return Err("must not be empty");
    }
    if version != version.trim() {
        return Err("must not have leading or trailing whitespace");
    }
    // Reject range operators — lock pins an exact resolved version.
    if version.starts_with('^')
        || version.starts_with('~')
        || version.starts_with('>')
        || version.starts_with('<')
        || version.starts_with('=')
    {
        return Err("must be a concrete version, not a range");
    }
    validate_version_req(version)
}

/// Full git commit SHA-1: exactly 40 lowercase hex digits.
fn validate_commit_oid(oid: &str) -> Result<(), &'static str> {
    if oid.len() != 40 {
        return Err("must be exactly 40 hexadecimal characters");
    }
    if !oid.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')) {
        return Err("must be lowercase hexadecimal");
    }
    Ok(())
}

/// Package tree SHA-256: exactly 64 lowercase hex digits.
fn validate_content_hash(hash: &str) -> Result<(), &'static str> {
    if hash.len() != 64 {
        return Err("must be exactly 64 hexadecimal characters");
    }
    if !hash.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')) {
        return Err("must be lowercase hexadecimal");
    }
    Ok(())
}

/// Parsed `draconic.lock` document (K02.02).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockFile {
    /// Format version (v1 = 1).
    pub version: u32,
    /// Pinned packages keyed by module path (sorted on write).
    pub packages: BTreeMap<String, LockEntry>,
}

/// Error while parsing or validating a `draconic.lock` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockFileError {
    /// Invalid TOML syntax.
    Toml(String),
    /// Document root is not a table.
    NotATable,
    /// Required top-level `version` is missing.
    MissingVersion,
    /// `version` is present but not a positive integer format id we support.
    InvalidVersion { got: String },
    /// Unsupported lock format version.
    UnsupportedVersion { version: u32 },
    /// Unknown top-level field.
    UnknownField { field: String },
    /// `package` is present but not an array of tables.
    InvalidPackageArray,
    /// A `[[package]]` table is missing a required string field.
    MissingPackageField { field: &'static str },
    /// A `[[package]]` field has the wrong type.
    InvalidPackageField { field: &'static str },
    /// Duplicate `path` among `[[package]]` entries.
    DuplicatePath { path: String },
    /// Package entry failed field validation.
    Entry(LockEntryError),
}

impl fmt::Display for LockFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockFileError::Toml(msg) => write!(f, "invalid draconic.lock: {msg}"),
            LockFileError::NotATable => {
                write!(f, "draconic.lock: document root must be a table")
            }
            LockFileError::MissingVersion => {
                write!(f, "draconic.lock: missing required field `version`")
            }
            LockFileError::InvalidVersion { got } => {
                write!(
                    f,
                    "draconic.lock: `version` must be a positive integer, got {got}"
                )
            }
            LockFileError::UnsupportedVersion { version } => {
                write!(
                    f,
                    "draconic.lock: unsupported format version {version} (expected 1)"
                )
            }
            LockFileError::UnknownField { field } => write!(
                f,
                "draconic.lock: unknown field `{field}` (expected one of: version, package)"
            ),
            LockFileError::InvalidPackageArray => write!(
                f,
                "draconic.lock: `package` must be an array of tables (`[[package]]`)"
            ),
            LockFileError::MissingPackageField { field } => {
                write!(
                    f,
                    "draconic.lock: package entry missing required field `{field}`"
                )
            }
            LockFileError::InvalidPackageField { field } => {
                write!(f, "draconic.lock: package field `{field}` has invalid type")
            }
            LockFileError::DuplicatePath { path } => {
                write!(f, "draconic.lock: duplicate package path `{path}`")
            }
            LockFileError::Entry(e) => write!(f, "draconic.lock: {e}"),
        }
    }
}

impl std::error::Error for LockFileError {}

const LOCK_FORMAT_VERSION: u32 = 1;
const KNOWN_LOCK_TOP_LEVEL: &[&str] = &["version", "package"];
const KNOWN_PACKAGE_KEYS: &[&str] =
    &["path", "version", "git_url", "commit_oid", "content_hash", "subdir"];

/// Parse a `draconic.lock` source string into a validated [`LockFile`].
///
/// Expected shape (K02.02):
/// ```toml
/// version = 1
///
/// [[package]]
/// path = "github.com/org/lib"
/// version = "1.2.3"
/// git_url = "https://github.com/org/lib.git"
/// commit_oid = "0123456789abcdef0123456789abcdef01234567"
/// content_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
/// ```
pub fn parse_lock(src: &str) -> Result<LockFile, LockFileError> {
    let value: TomlValue = toml::from_str(src).map_err(|e| LockFileError::Toml(e.to_string()))?;
    let table = match value {
        TomlValue::Table(t) => t,
        _ => return Err(LockFileError::NotATable),
    };

    for key in table.keys() {
        if !KNOWN_LOCK_TOP_LEVEL.contains(&key.as_str()) {
            return Err(LockFileError::UnknownField { field: key.clone() });
        }
    }

    let version = match table.get("version") {
        None => return Err(LockFileError::MissingVersion),
        Some(TomlValue::Integer(n)) if *n > 0 && *n <= i64::from(u32::MAX) => *n as u32,
        Some(other) => {
            return Err(LockFileError::InvalidVersion {
                got: other.to_string(),
            });
        }
    };
    if version != LOCK_FORMAT_VERSION {
        return Err(LockFileError::UnsupportedVersion { version });
    }

    let mut packages = BTreeMap::new();
    match table.get("package") {
        None => {}
        Some(TomlValue::Array(arr)) => {
            for item in arr {
                let pkg_table = match item {
                    TomlValue::Table(t) => t,
                    _ => return Err(LockFileError::InvalidPackageArray),
                };
                for key in pkg_table.keys() {
                    if !KNOWN_PACKAGE_KEYS.contains(&key.as_str()) {
                        return Err(LockFileError::UnknownField {
                            field: format!("package.{key}"),
                        });
                    }
                }
                let path = require_pkg_string(pkg_table, "path")?;
                let ver = require_pkg_string(pkg_table, "version")?;
                let git_url = require_pkg_string(pkg_table, "git_url")?;
                let commit_oid = require_pkg_string(pkg_table, "commit_oid")?;
                let content_hash = require_pkg_string(pkg_table, "content_hash")?;
                let subdir = optional_pkg_string(pkg_table, "subdir")?.unwrap_or_default();
                let entry = LockEntry::new(path.clone(), ver, git_url, commit_oid, content_hash)
                    .map_err(LockFileError::Entry)?
                    .with_subdir(subdir)
                    .map_err(LockFileError::Entry)?;
                if packages.contains_key(&path) {
                    return Err(LockFileError::DuplicatePath { path });
                }
                packages.insert(path, entry);
            }
        }
        Some(_) => return Err(LockFileError::InvalidPackageArray),
    }

    Ok(LockFile { version, packages })
}

fn require_pkg_string(
    table: &toml::map::Map<String, TomlValue>,
    field: &'static str,
) -> Result<String, LockFileError> {
    match table.get(field) {
        None => Err(LockFileError::MissingPackageField { field }),
        Some(TomlValue::String(s)) => Ok(s.clone()),
        Some(_) => Err(LockFileError::InvalidPackageField { field }),
    }
}

fn optional_pkg_string(
    table: &toml::map::Map<String, TomlValue>,
    field: &'static str,
) -> Result<Option<String>, LockFileError> {
    match table.get(field) {
        None => Ok(None),
        Some(TomlValue::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(LockFileError::InvalidPackageField { field }),
    }
}

/// Serialize a [`LockFile`] to a stable `draconic.lock` document.
///
/// Emit shape (K02.02–K02.03):
/// - `version = N` first
/// - one `[[package]]` table per entry, paths in sorted (BTreeMap) order
/// - fields: path, version, git_url, commit_oid, content_hash
/// - trailing newline
/// - parse → write → write is byte-identical when the lock is unchanged (K02.03)
pub fn write_lock(lock: &LockFile) -> String {
    let mut out = String::new();
    out.push_str(&format!("version = {}\n", lock.version));
    for entry in lock.packages.values() {
        out.push('\n');
        out.push_str("[[package]]\n");
        out.push_str("path = ");
        out.push_str(&toml_quoted_string(&entry.path));
        out.push('\n');
        out.push_str("version = ");
        out.push_str(&toml_quoted_string(&entry.version));
        out.push('\n');
        out.push_str("git_url = ");
        out.push_str(&toml_quoted_string(&entry.git_url));
        out.push('\n');
        out.push_str("commit_oid = ");
        out.push_str(&toml_quoted_string(&entry.commit_oid));
        out.push('\n');
        out.push_str("content_hash = ");
        out.push_str(&toml_quoted_string(&entry.content_hash));
        out.push('\n');
        if !entry.subdir.is_empty() {
            out.push_str("subdir = ");
            out.push_str(&toml_quoted_string(&entry.subdir));
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

    const PATH: &str = "github.com/org/lib";
    const VERSION: &str = "1.2.3";
    const GIT_URL: &str = "https://github.com/org/lib.git";
    const OID: &str = "0123456789abcdef0123456789abcdef01234567";
    const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn valid_entry() -> LockEntry {
        LockEntry::new(PATH, VERSION, GIT_URL, OID, HASH).expect("valid entry")
    }

    fn sample_lock_src() -> String {
        format!(
            r#"version = 1

[[package]]
path = "{PATH}"
version = "{VERSION}"
git_url = "{GIT_URL}"
commit_oid = "{OID}"
content_hash = "{HASH}"
"#
        )
    }

    #[test]
    fn new_accepts_full_field_set() {
        let e = valid_entry();
        assert_eq!(e.path, PATH);
        assert_eq!(e.version, VERSION);
        assert_eq!(e.git_url, GIT_URL);
        assert_eq!(e.commit_oid, OID);
        assert_eq!(e.content_hash, HASH);
    }

    #[test]
    fn validate_ok_on_valid_entry() {
        assert!(valid_entry().validate().is_ok());
    }

    #[test]
    fn reject_invalid_module_path() {
        let err = LockEntry::new("not-a-path", VERSION, GIT_URL, OID, HASH).expect_err("bad path");
        match &err {
            LockEntryError::InvalidPath { path, reason } => {
                assert_eq!(path, "not-a-path");
                assert!(!reason.is_empty());
            }
            other => panic!("expected InvalidPath, got {other:?}"),
        }
        assert!(err.to_string().contains("module path"));
    }

    #[test]
    fn reject_empty_version() {
        let err = LockEntry::new(PATH, "", GIT_URL, OID, HASH).expect_err("empty version");
        assert!(matches!(err, LockEntryError::InvalidVersion { .. }));
    }

    #[test]
    fn reject_range_version() {
        let err = LockEntry::new(PATH, "^1.2.3", GIT_URL, OID, HASH).expect_err("range");
        match err {
            LockEntryError::InvalidVersion { version, reason } => {
                assert_eq!(version, "^1.2.3");
                assert!(reason.contains("concrete") || reason.contains("range"));
            }
            other => panic!("expected InvalidVersion, got {other:?}"),
        }
    }

    #[test]
    fn accept_concrete_versions() {
        for v in ["1.2.3", "0.1.0", "2.0.0-alpha.1", "1.0.0+build.5", "v1.2.3"] {
            LockEntry::new(PATH, v, GIT_URL, OID, HASH)
                .unwrap_or_else(|e| panic!("version {v:?}: {e}"));
        }
    }

    #[test]
    fn reject_bad_git_url() {
        let err = LockEntry::new(PATH, VERSION, "ftp://x", OID, HASH).expect_err("bad url");
        assert!(matches!(err, LockEntryError::InvalidGitUrl { .. }));
    }

    #[test]
    fn reject_short_commit_oid() {
        let err = LockEntry::new(PATH, VERSION, GIT_URL, "abc123", HASH).expect_err("short oid");
        match err {
            LockEntryError::InvalidCommitOid { oid, reason } => {
                assert_eq!(oid, "abc123");
                assert!(reason.contains("40"));
            }
            other => panic!("expected InvalidCommitOid, got {other:?}"),
        }
    }

    #[test]
    fn reject_uppercase_commit_oid() {
        let upper = "0123456789ABCDEF0123456789ABCDEF01234567";
        let err = LockEntry::new(PATH, VERSION, GIT_URL, upper, HASH).expect_err("upper oid");
        assert!(matches!(err, LockEntryError::InvalidCommitOid { .. }));
    }

    #[test]
    fn reject_short_content_hash() {
        let err = LockEntry::new(PATH, VERSION, GIT_URL, OID, "deadbeef").expect_err("short hash");
        match err {
            LockEntryError::InvalidContentHash { hash, reason } => {
                assert_eq!(hash, "deadbeef");
                assert!(reason.contains("64"));
            }
            other => panic!("expected InvalidContentHash, got {other:?}"),
        }
    }

    #[test]
    fn reject_uppercase_content_hash() {
        let upper = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let err = LockEntry::new(PATH, VERSION, GIT_URL, OID, upper).expect_err("upper hash");
        assert!(matches!(err, LockEntryError::InvalidContentHash { .. }));
    }

    #[test]
    fn equality_compares_all_fields() {
        let a = valid_entry();
        let b = LockEntry::new(PATH, VERSION, GIT_URL, OID, HASH).unwrap();
        assert_eq!(a, b);
        let mut c = a.clone();
        c.version = "9.9.9".into();
        assert_ne!(a, c);
    }

    // --- K02.02: parse / write lock; reject malformed ---

    #[test]
    fn parse_empty_packages() {
        let lock = parse_lock("version = 1\n").expect("parse");
        assert_eq!(lock.version, 1);
        assert!(lock.packages.is_empty());
    }

    #[test]
    fn parse_one_package() {
        let lock = parse_lock(&sample_lock_src()).expect("parse");
        assert_eq!(lock.version, 1);
        assert_eq!(lock.packages.len(), 1);
        let e = lock.packages.get(PATH).expect("path key");
        assert_eq!(e.path, PATH);
        assert_eq!(e.version, VERSION);
        assert_eq!(e.git_url, GIT_URL);
        assert_eq!(e.commit_oid, OID);
        assert_eq!(e.content_hash, HASH);
    }

    #[test]
    fn parse_two_packages() {
        let src = format!(
            r#"version = 1

[[package]]
path = "github.com/z/last"
version = "3.0.0"
git_url = "https://github.com/z/last.git"
commit_oid = "{OID}"
content_hash = "{HASH}"

[[package]]
path = "github.com/a/first"
version = "1.0.0"
git_url = "https://github.com/a/first.git"
commit_oid = "{OID}"
content_hash = "{HASH}"
"#
        );
        let lock = parse_lock(&src).expect("parse");
        assert_eq!(lock.packages.len(), 2);
        assert!(lock.packages.contains_key("github.com/a/first"));
        assert!(lock.packages.contains_key("github.com/z/last"));
    }

    #[test]
    fn write_empty_packages() {
        let lock = LockFile {
            version: 1,
            packages: BTreeMap::new(),
        };
        assert_eq!(write_lock(&lock), "version = 1\n");
    }

    #[test]
    fn write_one_package() {
        let mut packages = BTreeMap::new();
        packages.insert(PATH.to_string(), valid_entry());
        let lock = LockFile {
            version: 1,
            packages,
        };
        let expected = sample_lock_src();
        assert_eq!(write_lock(&lock), expected);
    }

    #[test]
    fn write_packages_sorted_by_path() {
        let mut packages = BTreeMap::new();
        packages.insert(
            "github.com/z/last".into(),
            LockEntry::new(
                "github.com/z/last",
                "3.0.0",
                "https://github.com/z/last.git",
                OID,
                HASH,
            )
            .unwrap(),
        );
        packages.insert(
            "github.com/a/first".into(),
            LockEntry::new(
                "github.com/a/first",
                "1.0.0",
                "https://github.com/a/first.git",
                OID,
                HASH,
            )
            .unwrap(),
        );
        let lock = LockFile {
            version: 1,
            packages,
        };
        let written = write_lock(&lock);
        let a_pos = written.find("github.com/a/first").expect("a");
        let z_pos = written.find("github.com/z/last").expect("z");
        assert!(a_pos < z_pos, "paths should be sorted: {written}");
    }

    #[test]
    fn round_trip_parse_write() {
        let original = parse_lock(&sample_lock_src()).expect("parse");
        let written = write_lock(&original);
        let again = parse_lock(&written).expect("reparse");
        assert_eq!(again, original);
        assert_eq!(write_lock(&again), written);
    }

    // --- K02.03: stable lock serialize — sorted paths; byte-identical rewrite ---

    #[test]
    fn k02_03_unsorted_input_writes_sorted_paths() {
        // Input packages appear z-before-a; write must emit a-before-z.
        let src = format!(
            r#"version = 1

[[package]]
path = "github.com/z/last"
version = "3.0.0"
git_url = "https://github.com/z/last.git"
commit_oid = "{OID}"
content_hash = "{HASH}"

[[package]]
path = "github.com/m/mid"
version = "2.0.0"
git_url = "https://github.com/m/mid.git"
commit_oid = "{OID}"
content_hash = "{HASH}"

[[package]]
path = "github.com/a/first"
version = "1.0.0"
git_url = "https://github.com/a/first.git"
commit_oid = "{OID}"
content_hash = "{HASH}"
"#
        );
        let lock = parse_lock(&src).expect("parse reverse-order input");
        let written = write_lock(&lock);
        let a = written.find("path = \"github.com/a/first\"").expect("a");
        let m = written.find("path = \"github.com/m/mid\"").expect("m");
        let z = written.find("path = \"github.com/z/last\"").expect("z");
        assert!(a < m && m < z, "expected a < m < z in:\n{written}");
    }

    #[test]
    fn k02_03_rewrite_unchanged_is_byte_identical() {
        let src = format!(
            r#"version = 1

[[package]]
path = "github.com/z/last"
version = "3.0.0"
git_url = "https://github.com/z/last.git"
commit_oid = "{OID}"
content_hash = "{HASH}"

[[package]]
path = "github.com/a/first"
version = "1.0.0"
git_url = "https://github.com/a/first.git"
commit_oid = "{OID}"
content_hash = "{HASH}"
"#
        );
        let lock = parse_lock(&src).expect("parse");
        let once = write_lock(&lock);
        let twice = write_lock(&parse_lock(&once).expect("reparse"));
        assert_eq!(once.as_bytes(), twice.as_bytes());
        // Canonical form is also stable across a third rewrite.
        assert_eq!(
            write_lock(&parse_lock(&twice).unwrap()).as_bytes(),
            once.as_bytes()
        );
    }

    #[test]
    fn reject_invalid_toml() {
        let err = parse_lock("version = [").expect_err("bad toml");
        assert!(matches!(err, LockFileError::Toml(_)), "{err:?}");
    }

    #[test]
    fn reject_missing_version() {
        let err = parse_lock("[[package]]\npath = \"x\"\n").expect_err("missing version");
        assert_eq!(err, LockFileError::MissingVersion);
    }

    #[test]
    fn reject_unsupported_format_version() {
        let err = parse_lock("version = 99\n").expect_err("bad format");
        match err {
            LockFileError::UnsupportedVersion { version } => assert_eq!(version, 99),
            other => panic!("expected UnsupportedVersion, got {other:?}"),
        }
    }

    #[test]
    fn reject_version_wrong_type() {
        let err = parse_lock("version = \"1\"\n").expect_err("string version");
        assert!(
            matches!(err, LockFileError::InvalidVersion { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn reject_unknown_top_level_field() {
        let err = parse_lock("version = 1\nextra = true\n").expect_err("unknown");
        match err {
            LockFileError::UnknownField { field } => assert_eq!(field, "extra"),
            other => panic!("expected UnknownField, got {other:?}"),
        }
    }

    #[test]
    fn reject_package_not_array() {
        let err = parse_lock("version = 1\npackage = \"nope\"\n").expect_err("not array");
        assert_eq!(err, LockFileError::InvalidPackageArray);
    }

    #[test]
    fn reject_missing_package_field() {
        let src = r#"version = 1

[[package]]
path = "github.com/org/lib"
version = "1.2.3"
git_url = "https://github.com/org/lib.git"
"#;
        let err = parse_lock(src).expect_err("missing fields");
        assert!(
            matches!(err, LockFileError::MissingPackageField { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn reject_invalid_entry_fields() {
        let src = format!(
            r#"version = 1

[[package]]
path = "not-a-path"
version = "{VERSION}"
git_url = "{GIT_URL}"
commit_oid = "{OID}"
content_hash = "{HASH}"
"#
        );
        let err = parse_lock(&src).expect_err("bad path");
        assert!(matches!(err, LockFileError::Entry(_)), "{err:?}");
        assert!(err.to_string().contains("module path"), "{err}");
    }

    #[test]
    fn reject_duplicate_path() {
        let src = format!(
            r#"version = 1

[[package]]
path = "{PATH}"
version = "{VERSION}"
git_url = "{GIT_URL}"
commit_oid = "{OID}"
content_hash = "{HASH}"

[[package]]
path = "{PATH}"
version = "9.9.9"
git_url = "{GIT_URL}"
commit_oid = "{OID}"
content_hash = "{HASH}"
"#
        );
        let err = parse_lock(&src).expect_err("duplicate");
        match err {
            LockFileError::DuplicatePath { path } => assert_eq!(path, PATH),
            other => panic!("expected DuplicatePath, got {other:?}"),
        }
    }

    #[test]
    fn reject_unknown_package_field() {
        let src = format!(
            r#"version = 1

[[package]]
path = "{PATH}"
version = "{VERSION}"
git_url = "{GIT_URL}"
commit_oid = "{OID}"
content_hash = "{HASH}"
extra = true
"#
        );
        let err = parse_lock(&src).expect_err("unknown pkg field");
        match err {
            LockFileError::UnknownField { field } => {
                assert!(field.contains("extra"), "{field}");
            }
            other => panic!("expected UnknownField, got {other:?}"),
        }
    }

    // --- K02: combined lockfile resolved pins (parent of K02.01–K02.03) ---

    #[test]
    fn k02_combined_lockfile_resolved_pins() {
        // Unsorted input; each pin carries path + version + git URL + commit OID + tree SHA-256.
        let src = format!(
            r#"version = 1

[[package]]
path = "github.com/z/last"
version = "3.0.0"
git_url = "https://github.com/z/last.git"
commit_oid = "{OID}"
content_hash = "{HASH}"

[[package]]
path = "github.com/a/first"
version = "1.2.3"
git_url = "https://git.example.com/mirror/first.git"
commit_oid = "{OID}"
content_hash = "{HASH}"
"#
        );
        let lock = parse_lock(&src).expect("parse honest lock");
        assert_eq!(lock.version, 1);
        assert_eq!(lock.packages.len(), 2);

        let a = lock.packages.get("github.com/a/first").expect("a");
        assert_eq!(a.path, "github.com/a/first");
        assert_eq!(a.version, "1.2.3");
        assert_eq!(a.git_url, "https://git.example.com/mirror/first.git");
        assert_eq!(a.commit_oid, OID);
        assert_eq!(a.content_hash, HASH);
        a.validate().expect("entry schema");

        let z = lock.packages.get("github.com/z/last").expect("z");
        assert_eq!(z.path, "github.com/z/last");
        assert_eq!(z.version, "3.0.0");
        assert_eq!(z.git_url, "https://github.com/z/last.git");
        assert_eq!(z.commit_oid, OID);
        assert_eq!(z.content_hash, HASH);
        z.validate().expect("entry schema");

        // Stable serialize: sorted by path; rewrite of unchanged lock is byte-identical (K02.03).
        let written = write_lock(&lock);
        let a_pos = written
            .find("path = \"github.com/a/first\"")
            .expect("a path");
        let z_pos = written
            .find("path = \"github.com/z/last\"")
            .expect("z path");
        assert!(a_pos < z_pos, "packages sorted by path:\n{written}");
        let again = parse_lock(&written).expect("round-trip");
        assert_eq!(again, lock);
        assert_eq!(write_lock(&again).as_bytes(), written.as_bytes());

        // Parse/write reject malformed (K02.02).
        let malformed = parse_lock("version = 1\nextra = true\n").expect_err("unknown field");
        match &malformed {
            LockFileError::UnknownField { field } => assert_eq!(field, "extra"),
            other => panic!("expected UnknownField, got {other:?}"),
        }
        assert!(
            malformed.to_string().contains("draconic.lock"),
            "diagnostic: {malformed}"
        );

        let bad_entry = parse_lock(&format!(
            r#"version = 1

[[package]]
path = "not-a-path"
version = "{VERSION}"
git_url = "{GIT_URL}"
commit_oid = "{OID}"
content_hash = "{HASH}"
"#
        ))
        .expect_err("bad path");
        assert!(
            matches!(bad_entry, LockFileError::Entry(_)),
            "{bad_entry:?}"
        );
        assert!(
            bad_entry.to_string().contains("module path"),
            "diagnostic: {bad_entry}"
        );
    }
}
