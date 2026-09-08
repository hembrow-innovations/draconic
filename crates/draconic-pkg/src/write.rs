//! Serialize [`Manifest`] to a stable `draconic.toml` document (K01.02).

use crate::{Manifest, ReplaceSource};

/// Serialize a [`Manifest`] to a stable `draconic.toml` document.
///
/// Emit shape (K01.02 / K01.04 / K11.02 / D02.01):
/// - `module = "…"` first
/// - `toolchain = "…"` when optional pin; inline table when `required = true`
/// - blank line then `[dependencies]` only when non-empty
/// - dependency keys in sorted (BTreeMap) order, each quoted
/// - blank line then `[urls]` only when non-empty (sorted keys)
/// - blank line then `[replace]` only when non-empty (sorted keys; inline tables)
/// - trailing newline
///
/// Round-trip: `parse_manifest(&write_manifest(m)) == Ok(m)` (equal after parse).
/// Rewrite is byte-identical: `write_manifest(&parse_manifest(write(m))?) == write(m)`.
pub fn write_manifest(manifest: &Manifest) -> String {
    let mut out = String::new();
    out.push_str("module = ");
    out.push_str(&toml_quoted_string(&manifest.module));
    out.push('\n');

    if let Some(pin) = &manifest.toolchain {
        out.push_str("toolchain = ");
        if pin.required {
            out.push_str("{ version = ");
            out.push_str(&toml_quoted_string(&pin.version));
            out.push_str(", required = true }");
        } else {
            out.push_str(&toml_quoted_string(&pin.version));
        }
        out.push('\n');
    }

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

    if !manifest.replace.is_empty() {
        out.push('\n');
        out.push_str("[replace]\n");
        for (path, source) in &manifest.replace {
            out.push_str(&toml_quoted_string(path));
            out.push_str(" = ");
            out.push_str(&write_replace_source(source));
            out.push('\n');
        }
    }

    out
}

fn write_replace_source(source: &ReplaceSource) -> String {
    match source {
        ReplaceSource::Git { url } => {
            format!("{{ git = {} }}", toml_quoted_string(url))
        }
        ReplaceSource::Module { path } => {
            format!("{{ module = {} }}", toml_quoted_string(path))
        }
        ReplaceSource::Path { path } => {
            format!("{{ path = {} }}", toml_quoted_string(path))
        }
    }
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
    use std::collections::BTreeMap;

    use crate::{parse_manifest, ToolchainPin};

    fn manifest(module: &str, deps: &[(&str, &str)]) -> Manifest {
        manifest_with_urls(module, deps, &[])
    }

    fn manifest_with_urls(module: &str, deps: &[(&str, &str)], urls: &[(&str, &str)]) -> Manifest {
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
            replace: BTreeMap::new(),
            toolchain: None,
        }
    }

    #[test]
    fn write_module_only() {
        let m = manifest("github.com/org/pkg", &[]);
        assert_eq!(write_manifest(&m), "module = \"github.com/org/pkg\"\n");
    }

    #[test]
    fn write_module_and_deps_sorted() {
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
        assert!(parse_manifest(&s).is_err());
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
    fn write_omits_absent_toolchain() {
        let m = manifest("github.com/org/pkg", &[]);
        let s = write_manifest(&m);
        assert!(!s.contains("toolchain"), "{s}");
    }

    #[test]
    fn write_optional_toolchain_as_string() {
        let mut m = manifest("github.com/org/pkg", &[]);
        m.toolchain = Some(ToolchainPin {
            version: "0.1.0".into(),
            required: false,
        });
        assert_eq!(
            write_manifest(&m),
            "module = \"github.com/org/pkg\"\ntoolchain = \"0.1.0\"\n"
        );
    }

    #[test]
    fn write_required_toolchain_as_inline_table() {
        let mut m = manifest("github.com/org/pkg", &[]);
        m.toolchain = Some(ToolchainPin {
            version: "1.2.3".into(),
            required: true,
        });
        assert_eq!(
            write_manifest(&m),
            "module = \"github.com/org/pkg\"\ntoolchain = { version = \"1.2.3\", required = true }\n"
        );
    }

    #[test]
    fn round_trip_optional_and_required_toolchain() {
        for required in [false, true] {
            let mut original = manifest("github.com/acme/app", &[("github.com/org/lib", "1.0.0")]);
            original.toolchain = Some(ToolchainPin {
                version: "0.3.0".into(),
                required,
            });
            let written = write_manifest(&original);
            let parsed = parse_manifest(&written).expect("parse written");
            assert_eq!(parsed, original);
            assert_eq!(write_manifest(&parsed), written);
        }
    }
}
