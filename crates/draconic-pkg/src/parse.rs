//! Parse `draconic.toml` into a schema-valid [`Manifest`] (K01.01).

use std::collections::BTreeMap;

use toml::Value as TomlValue;

use crate::replace;
use crate::{validate_manifest, Manifest, ManifestError, ToolchainPin};

/// Known top-level keys in `draconic.toml` (K01.01–K01.04, K11.02, D02.01).
const KNOWN_TOP_LEVEL_KEYS: &[&str] = &["module", "dependencies", "urls", "replace", "toolchain"];

/// Known keys inside a `[toolchain]` / inline-table pin (D02.01).
const KNOWN_TOOLCHAIN_KEYS: &[&str] = &["version", "required"];

/// Parse a `draconic.toml` source string into a schema-valid [`Manifest`].
///
/// Expected shape (K01.01–K01.04, K11.02, D02.01):
/// ```toml
/// module = "github.com/org/pkg"
/// toolchain = "0.1.0"
///
/// [dependencies]
/// "github.com/other/lib" = "1.2.3"
///
/// [urls]
/// "github.com/other/lib" = "https://git.example.com/other/lib.git"
///
/// [replace]
/// "github.com/other/lib" = { git = "https://github.com/fork/lib.git" }
/// ```
///
/// `dependencies`, `urls`, `replace`, and `toolchain` may be omitted. `toolchain` may be a
/// version string (optional pin) or a table `{ version, required }`. Performs
/// structural decode plus schema validation (module paths, version requirements,
/// git URLs, replace sources, unknown fields).
pub fn parse_manifest(src: &str) -> Result<Manifest, ManifestError> {
    let value: TomlValue = toml::from_str(src).map_err(|e| ManifestError::Toml(e.to_string()))?;
    let table = match value {
        TomlValue::Table(t) => t,
        _ => return Err(ManifestError::NotATable),
    };

    for key in table.keys() {
        if !KNOWN_TOP_LEVEL_KEYS.contains(&key.as_str()) {
            return Err(ManifestError::UnknownField { field: key.clone() });
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
                        return Err(ManifestError::InvalidDependencyValue { path: path.clone() });
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
                        return Err(ManifestError::InvalidUrlValue { path: path.clone() });
                    }
                };
                map.insert(path.clone(), url);
            }
            map
        }
        Some(_) => return Err(ManifestError::InvalidUrls),
    };

    let replace = match table.get("replace") {
        None => BTreeMap::new(),
        Some(TomlValue::Table(replace_table)) => replace::parse_replace_table(replace_table)?,
        Some(_) => return Err(ManifestError::InvalidReplace),
    };

    let toolchain = parse_toolchain_value(table.get("toolchain"))?;

    let manifest = Manifest {
        module,
        dependencies,
        urls,
        replace,
        toolchain,
    };
    validate_manifest(&manifest)?;
    Ok(manifest)
}

fn parse_toolchain_value(value: Option<&TomlValue>) -> Result<Option<ToolchainPin>, ManifestError> {
    match value {
        None => Ok(None),
        Some(TomlValue::String(version)) => Ok(Some(ToolchainPin {
            version: version.clone(),
            required: false,
        })),
        Some(TomlValue::Table(table)) => {
            for key in table.keys() {
                if !KNOWN_TOOLCHAIN_KEYS.contains(&key.as_str()) {
                    return Err(ManifestError::UnknownField { field: key.clone() });
                }
            }
            let version = match table.get("version") {
                None => return Err(ManifestError::MissingToolchainVersion),
                Some(TomlValue::String(v)) => v.clone(),
                Some(_) => return Err(ManifestError::InvalidToolchain),
            };
            let required = match table.get("required") {
                None => false,
                Some(TomlValue::Boolean(b)) => *b,
                Some(_) => return Err(ManifestError::InvalidToolchainRequired),
            };
            Ok(Some(ToolchainPin { version, required }))
        }
        Some(_) => Err(ManifestError::InvalidToolchain),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{default_git_url, resolve_git_url, validate_manifest, write_manifest};

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
    fn module_wrong_type_is_invalid_module_not_opaque_toml() {
        let err = parse_manifest("module = 42").expect_err("wrong type");
        assert_eq!(err, ManifestError::InvalidModule);
        assert!(
            err.to_string().contains("module"),
            "clear diagnostic: {err}"
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
            m.urls.get("github.com/private/tool").map(String::as_str),
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
    fn parse_omitted_toolchain_is_none() {
        let m = parse_manifest(r#"module = "github.com/org/pkg""#).expect("parse");
        assert!(m.toolchain.is_none());
    }

    #[test]
    fn parse_optional_toolchain_string() {
        let m = parse_manifest(
            r#"
module = "github.com/org/pkg"
toolchain = "0.1.0"
"#,
        )
        .expect("parse");
        let pin = m.toolchain.expect("optional pin");
        assert_eq!(pin.version, "0.1.0");
        assert!(!pin.required);
    }

    #[test]
    fn parse_required_toolchain_table() {
        let m = parse_manifest(
            r#"
module = "github.com/org/pkg"
toolchain = { version = "1.2.3", required = true }
"#,
        )
        .expect("parse");
        let pin = m.toolchain.expect("required pin");
        assert_eq!(pin.version, "1.2.3");
        assert!(pin.required);
    }

    #[test]
    fn parse_optional_toolchain_table() {
        let m = parse_manifest(
            r#"
module = "github.com/org/pkg"
[toolchain]
version = "0.2.0"
required = false
"#,
        )
        .expect("parse");
        let pin = m.toolchain.expect("optional table pin");
        assert_eq!(pin.version, "0.2.0");
        assert!(!pin.required);
    }

    #[test]
    fn parse_toolchain_table_version_only_is_optional() {
        let m = parse_manifest(
            r#"
module = "github.com/org/pkg"
[toolchain]
version = "1.0.0"
"#,
        )
        .expect("parse");
        let pin = m.toolchain.expect("version-only table");
        assert_eq!(pin.version, "1.0.0");
        assert!(!pin.required);
    }

    #[test]
    fn reject_toolchain_wrong_type() {
        let err = parse_manifest(
            r#"
module = "github.com/org/pkg"
toolchain = 12
"#,
        )
        .expect_err("wrong type");
        assert!(matches!(err, ManifestError::InvalidToolchain), "{err:?}");
        assert!(err.to_string().contains("toolchain"), "diagnostic: {err}");
    }

    #[test]
    fn reject_toolchain_empty_version() {
        let err = parse_manifest(
            r#"
module = "github.com/org/pkg"
toolchain = ""
"#,
        )
        .expect_err("empty version");
        match &err {
            ManifestError::InvalidToolchainVersion { version, reason } => {
                assert_eq!(version, "");
                assert!(!reason.is_empty());
            }
            other => panic!("expected InvalidToolchainVersion, got {other:?}"),
        }
    }

    #[test]
    fn reject_toolchain_invalid_version() {
        let err = parse_manifest(
            r#"
module = "github.com/org/pkg"
toolchain = "not-a-version"
"#,
        )
        .expect_err("bad version");
        assert!(
            matches!(err, ManifestError::InvalidToolchainVersion { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn reject_toolchain_table_missing_version() {
        let err = parse_manifest(
            r#"
module = "github.com/org/pkg"
toolchain = { required = true }
"#,
        )
        .expect_err("missing version");
        assert_eq!(err, ManifestError::MissingToolchainVersion);
    }

    #[test]
    fn reject_toolchain_unknown_table_field() {
        let err = parse_manifest(
            r#"
module = "github.com/org/pkg"
toolchain = { version = "1.0.0", extra = true }
"#,
        )
        .expect_err("unknown table field");
        match err {
            ManifestError::UnknownField { field } => {
                assert_eq!(field, "extra");
            }
            other => panic!("expected UnknownField, got {other:?}"),
        }
    }

    #[test]
    fn k01_combined_manifest_parse_write_validate_and_url_map() {
        let src = "\
module = \"github.com/acme/app\"

[dependencies]
\"github.com/z/last\" = \"3.0.0\"
\"github.com/a/first\" = \"^1.2.3\"

[urls]
\"github.com/a/first\" = \"https://git.example.com/mirror/first.git\"
";
        let m = parse_manifest(src).expect("parse honest manifest");
        validate_manifest(&m).expect("schema");
        assert_eq!(m.module, "github.com/acme/app");
        assert_eq!(
            m.dependencies.get("github.com/a/first").map(String::as_str),
            Some("^1.2.3")
        );
        assert_eq!(
            m.dependencies.get("github.com/z/last").map(String::as_str),
            Some("3.0.0")
        );

        let expected = "\
module = \"github.com/acme/app\"

[dependencies]
\"github.com/a/first\" = \"^1.2.3\"
\"github.com/z/last\" = \"3.0.0\"

[urls]
\"github.com/a/first\" = \"https://git.example.com/mirror/first.git\"
";
        let written = write_manifest(&m);
        assert_eq!(written, expected);
        let again = parse_manifest(&written).expect("round-trip");
        assert_eq!(again, m);
        assert_eq!(write_manifest(&again), written);

        assert_eq!(
            resolve_git_url(&m, "github.com/a/first"),
            "https://git.example.com/mirror/first.git"
        );
        assert_eq!(
            resolve_git_url(&m, "github.com/z/last"),
            "https://github.com/z/last.git"
        );
        assert_eq!(
            default_git_url("github.com/acme/app"),
            "https://github.com/acme/app.git"
        );

        let unknown = parse_manifest(
            r#"
module = "github.com/acme/app"
license = "MIT"
[dependencies]
"github.com/org/lib" = "1.0.0"
[urls]
"github.com/org/lib" = "https://git.example.com/lib.git"
"#,
        )
        .expect_err("unknown field");
        match &unknown {
            ManifestError::UnknownField { field } => assert_eq!(field, "license"),
            other => panic!("expected UnknownField, got {other:?}"),
        }
        assert!(
            unknown.to_string().contains("draconic.toml"),
            "diagnostic: {unknown}"
        );

        let self_dep = parse_manifest(
            r#"
module = "github.com/acme/app"
[dependencies]
"github.com/acme/app" = "1.0.0"
[urls]
"github.com/acme/app" = "https://git.example.com/app.git"
"#,
        )
        .expect_err("self dependency");
        match self_dep {
            ManifestError::SelfDependency { path } => {
                assert_eq!(path, "github.com/acme/app");
            }
            other => panic!("expected SelfDependency, got {other:?}"),
        }
    }
}
