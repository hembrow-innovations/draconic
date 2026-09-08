use std::path::Path;

use draconic_ast::Program;
use draconic_diagnostics::{codes, Diagnostic, Span};
use draconic_parser::parse_module;

/// E19.84.03: JSON modules (`.json`) — ParseJSONModule. The raw JSON source is
/// embedded as a JS string literal and parsed by the runtime's `JSON.parse` at
/// eval, so the synthetic `default` binding is the parsed JSON value.
pub(crate) fn parse_json_module(source: &str, path: &Path) -> Result<Program, Diagnostic> {
    const LOCAL: &str = "__json_default";
    let lit = js_string_literal(source);
    let synthetic = format!("const {LOCAL} = JSON.parse({lit});\nexport {{ {LOCAL} as default }};");
    parse_module(&synthetic).map_err(|e| {
        Diagnostic::new(
            format!("failed to synthesize JSON module {}: {e}", path.display()),
            Span::dummy(),
        )
        .with_code(codes::LINKER_INTERNAL)
    })
}

/// Quote `s` as a double-quoted JS string literal (no U+2028/U+2029 in output).
pub(crate) fn js_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\u{b}' => out.push_str("\\u000b"),
            '\u{0}' => out.push_str("\\u0000"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use crate::link_entry;
    use std::fs;

    #[test]
    fn link_json_module_deferred_default() {
        // E19.84.03: `.json` files link as JSON modules whose `default` export is
        // the parsed JSON value; `import defer * as ns` yields a deferred namespace.
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-json-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let data = dir.join("data.json");
        let main = dir.join("main.drac");
        fs::write(&data, "{ \"test262\": \"JSON module\", \"number\": 42 }\n").unwrap();
        fs::write(
            &main,
            "import defer * as ns from \"./data.json\" with { type: \"json\" };\nlet x = ns;\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("json module link");
        let dump = draconic_ast::dump_program(&program);
        assert!(
            dump.contains("JSON.parse") || dump.contains("json_default"),
            "expected synthesized JSON.parse default, got:\n{dump}"
        );
        assert!(
            dump.contains("__draconic_deferred_ns") || dump.contains("draconic_deferred"),
            "expected deferred ns helper, got:\n{dump}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
