//! Combined `draconic.toml` types (Roadmap K01, K11.02, D02.01).

use std::collections::BTreeMap;
use std::fmt;

use crate::ReplaceSource;

/// Toolchain version pin from `draconic.toml` (D02.01).
///
/// String form `toolchain = "0.1.0"` is an **optional** pin (`required = false`).
/// Table form may set `required = true` so D02.02 can hard-fail on mismatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainPin {
    /// Semver-shaped version the Program expects of the running toolchain.
    pub version: String,
    /// `true` → mismatch is an error (D02.02); `false` → warn only.
    pub required: bool,
}

/// Parsed `draconic.toml` (K01: module path + deps + optional URL map; K11.02 replace; D02.01 pin).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// This package's module path (Go-like), e.g. `github.com/org/pkg`.
    pub module: String,
    /// Direct dependencies: module path → version requirement string.
    pub dependencies: BTreeMap<String, String>,
    /// Optional path → git URL overrides when default derivation is wrong (K01.04).
    pub urls: BTreeMap<String, String>,
    /// Optional module path → fork git source or local path (K11.02).
    pub replace: BTreeMap<String, ReplaceSource>,
    /// Optional toolchain version pin (D02.01). Omitted → no pin.
    pub toolchain: Option<ToolchainPin>,
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
    /// `replace` is present but not a table of module path → source.
    InvalidReplace,
    /// A `replace` entry is not a string or inline table.
    InvalidReplaceValue { path: String },
    /// A `replace` key fails Go-like module path schema.
    InvalidReplacePath { path: String, reason: &'static str },
    /// A `replace` source (git URL, module path, or local path) is invalid.
    InvalidReplaceSource {
        path: String,
        source: String,
        reason: &'static str,
    },
    /// A `replace` inline table sets more than one of `git` / `module` / `path`.
    AmbiguousReplace { path: String },
    /// A `replace` inline table sets none of `git` / `module` / `path`.
    MissingReplaceSource { path: String },
    /// `toolchain` is present but not a version string or table.
    InvalidToolchain,
    /// `toolchain` version is empty or not semver-shaped.
    InvalidToolchainVersion {
        version: String,
        reason: &'static str,
    },
    /// Table form `toolchain` is missing `version`.
    MissingToolchainVersion,
    /// `toolchain.required` is present but not a boolean.
    InvalidToolchainRequired,
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
                "draconic.toml: unknown field `{field}` (expected one of: module, dependencies, urls, replace, toolchain)"
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
            ManifestError::InvalidReplace => write!(
                f,
                "draconic.toml: `replace` must be a table of module path → git source, module path, or local path"
            ),
            ManifestError::InvalidReplaceValue { path } => write!(
                f,
                "draconic.toml: replace entry `{path}` must be a string or a table with `git`, `module`, or `path`"
            ),
            ManifestError::InvalidReplacePath { path, reason } => write!(
                f,
                "draconic.toml: invalid replace module path `{path}`: {reason}"
            ),
            ManifestError::InvalidReplaceSource {
                path,
                source,
                reason,
            } => write!(
                f,
                "draconic.toml: replace entry `{path}` has invalid source `{source}`: {reason}"
            ),
            ManifestError::AmbiguousReplace { path } => write!(
                f,
                "draconic.toml: replace entry `{path}` must set exactly one of `git`, `module`, or `path`"
            ),
            ManifestError::MissingReplaceSource { path } => write!(
                f,
                "draconic.toml: replace entry `{path}` is missing `git`, `module`, or `path`"
            ),
            ManifestError::InvalidToolchain => write!(
                f,
                "draconic.toml: `toolchain` must be a version string or a table with `version` and optional `required`"
            ),
            ManifestError::InvalidToolchainVersion { version, reason } => write!(
                f,
                "draconic.toml: invalid toolchain version `{version}`: {reason}"
            ),
            ManifestError::MissingToolchainVersion => {
                write!(f, "draconic.toml: `toolchain` table is missing `version`")
            }
            ManifestError::InvalidToolchainRequired => {
                write!(f, "draconic.toml: `toolchain.required` must be a boolean")
            }
        }
    }
}

impl std::error::Error for ManifestError {}
