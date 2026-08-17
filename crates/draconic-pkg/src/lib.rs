//! Package manager support: `draconic.toml` manifests and related types (Roadmap K).
//!
//! K01.01: parse own module path + dependencies map (path → version req).
//! K01.02: write/round-trip `draconic.toml` with stable dependency order.

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;
use toml::Value as TomlValue;

/// Parsed `draconic.toml` (K01.01 subset: module path + deps).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// This package's module path (Go-like), e.g. `github.com/org/pkg`.
    pub module: String,
    /// Direct dependencies: module path → version requirement string.
    pub dependencies: BTreeMap<String, String>,
}

/// Error while parsing a `draconic.toml` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// Invalid TOML syntax or structure that serde/toml cannot decode.
    Toml(String),
    /// Required top-level `module` string is missing.
    MissingModule,
    /// `module` is present but not a non-empty string.
    InvalidModule,
    /// `dependencies` is present but not a table of string → string.
    InvalidDependencies,
    /// A dependency entry has a non-string version requirement.
    InvalidDependencyValue { path: String },
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::Toml(msg) => write!(f, "invalid draconic.toml: {msg}"),
            ManifestError::MissingModule => {
                write!(f, "draconic.toml: missing required field `module`")
            }
            ManifestError::InvalidModule => {
                write!(f, "draconic.toml: `module` must be a non-empty string")
            }
            ManifestError::InvalidDependencies => write!(
                f,
                "draconic.toml: `dependencies` must be a table of module path → version requirement strings"
            ),
            ManifestError::InvalidDependencyValue { path } => write!(
                f,
                "draconic.toml: dependency `{path}` version requirement must be a string"
            ),
        }
    }
}

impl std::error::Error for ManifestError {}

/// Parse a `draconic.toml` source string into a [`Manifest`].
///
/// Expected shape (K01.01):
/// ```toml
/// module = "github.com/org/pkg"
///
/// [dependencies]
/// "github.com/other/lib" = "1.2.3"
/// ```
///
/// `dependencies` may be omitted (empty map). URL map and full schema validation
/// are later K01 children.
pub fn parse_manifest(src: &str) -> Result<Manifest, ManifestError> {
    let raw: RawManifest = toml::from_str(src).map_err(|e| ManifestError::Toml(e.to_string()))?;

    let module = match raw.module {
        None => return Err(ManifestError::MissingModule),
        Some(m) if m.is_empty() => return Err(ManifestError::InvalidModule),
        Some(m) => m,
    };

    let dependencies = match raw.dependencies {
        None => BTreeMap::new(),
        Some(TomlValue::Table(table)) => {
            let mut deps = BTreeMap::new();
            for (path, value) in table {
                let req = match value {
                    TomlValue::String(s) => s,
                    _ => {
                        return Err(ManifestError::InvalidDependencyValue { path });
                    }
                };
                deps.insert(path, req);
            }
            deps
        }
        Some(_) => return Err(ManifestError::InvalidDependencies),
    };

    Ok(Manifest {
        module,
        dependencies,
    })
}

/// Serialize a [`Manifest`] to a stable `draconic.toml` document.
///
/// Emit shape (K01.02):
/// - `module = "…"` first
/// - blank line then `[dependencies]` only when non-empty
/// - dependency keys in sorted (BTreeMap) order, each quoted
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

/// Intermediate decode so we can distinguish missing vs wrong-typed fields.
#[derive(Debug, Deserialize)]
struct RawManifest {
    module: Option<String>,
    #[serde(default)]
    dependencies: Option<TomlValue>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(module: &str, deps: &[(&str, &str)]) -> Manifest {
        Manifest {
            module: module.to_string(),
            dependencies: deps
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
        // `module` as integer → serde fails decoding Option<String> → Toml error
        let err = parse_manifest("module = 42").expect_err("wrong type");
        assert!(
            matches!(
                err,
                ManifestError::Toml(_) | ManifestError::InvalidModule
            ),
            "{err:?}"
        );
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
        let m = manifest(r#"org/pkg"with"quotes"#, &[]);
        let s = write_manifest(&m);
        assert_eq!(s, "module = \"org/pkg\\\"with\\\"quotes\"\n");
        let parsed = parse_manifest(&s).expect("parse");
        assert_eq!(parsed, m);
    }
}
