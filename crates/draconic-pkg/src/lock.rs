//! Lockfile entry types for `draconic.lock` (Roadmap K02.01).
//!
//! A lock entry pins one direct dependency: module path, resolved version,
//! git URL, commit OID, and package-tree content hash (SHA-256).

use std::fmt;

use crate::{validate_git_url, validate_module_path, validate_version_req};

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
    InvalidCommitOid {
        oid: String,
        reason: &'static str,
    },
    /// Content hash is not a 64-char lowercase hex SHA-256 digest.
    InvalidContentHash {
        hash: String,
        reason: &'static str,
    },
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
        };
        entry.validate()?;
        Ok(entry)
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
        let err = LockEntry::new("not-a-path", VERSION, GIT_URL, OID, HASH)
            .expect_err("bad path");
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
}
