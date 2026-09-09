//! Host API registry: known symbols + per-target availability (H00 / H00.01).
//!
//! Scaffold for H01+ surfaces. Native-only entries hard-error on the js target
//! (ADR-0008). H00 locks host APIs as **free identifiers** on the global object
//! (not a module import); this module owns the name registry and availability.

use draconic_diagnostics::{codes, Diagnostic, Span};

/// Compile backend a host API may be available on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompileTarget {
    Js,
    Native,
}

impl CompileTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Js => "js",
            Self::Native => "native",
        }
    }
}

/// Which backends may emit a given host API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HostAvailability {
    pub js: bool,
    pub native: bool,
}

impl HostAvailability {
    pub const NATIVE_ONLY: Self = Self {
        js: false,
        native: true,
    };

    pub const BOTH: Self = Self {
        js: true,
        native: true,
    };

    pub fn on(self, target: CompileTarget) -> bool {
        match target {
            CompileTarget::Js => self.js,
            CompileTarget::Native => self.native,
        }
    }
}

/// One known host API symbol in the compiler registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostApiEntry {
    /// Free identifier name used in Programs (H00 locked global shape).
    pub name: &'static str,
    pub availability: HostAvailability,
    /// Short note for diagnostics (e.g. cluster id).
    pub note: &'static str,
}

#[path = "host_api_fs.rs"]
mod host_api_fs;
#[path = "host_api_net.rs"]
mod host_api_net;
#[path = "host_api_process.rs"]
mod host_api_process;
#[path = "host_api_workers.rs"]
mod host_api_workers;

const fn host_api_count() -> usize {
    host_api_process::ENTRIES.len()
        + host_api_fs::ENTRIES.len()
        + host_api_net::ENTRIES.len()
        + host_api_workers::ENTRIES.len()
}

const HOST_APIS: [HostApiEntry; host_api_count()] = {
    let mut out = [HostApiEntry {
        name: "",
        availability: HostAvailability::BOTH,
        note: "",
    }; host_api_count()];
    let groups: [&[HostApiEntry]; 4] = [
        host_api_process::ENTRIES,
        host_api_fs::ENTRIES,
        host_api_net::ENTRIES,
        host_api_workers::ENTRIES,
    ];
    let mut i = 0;
    let mut g = 0;
    while g < groups.len() {
        let mut j = 0;
        while j < groups[g].len() {
            out[i] = groups[g][j];
            i += 1;
            j += 1;
        }
        g += 1;
    }
    out
};

/// All known host API entries.
pub fn host_apis() -> &'static [HostApiEntry] {
    &HOST_APIS
}

/// Look up a host API by free-identifier name.
pub fn lookup(name: &str) -> Option<&'static HostApiEntry> {
    HOST_APIS.iter().find(|e| e.name == name)
}

/// True when `name` is a registered host API symbol.
pub fn is_host_api(name: &str) -> bool {
    lookup(name).is_some()
}

/// True when a registered host API is available on `target`.
///
/// Unknown names return `false` (callers should use [`lookup`] first).
pub fn is_available(name: &str, target: CompileTarget) -> bool {
    lookup(name)
        .map(|e| e.availability.on(target))
        .unwrap_or(false)
}

/// Build a hard diagnostic when a free host API reference is unsupported on `target`.
///
/// Returns `None` when `name` is not a host API, or when it is available on `target`.
pub fn unsupported_diagnostic(name: &str, target: CompileTarget, span: Span) -> Option<Diagnostic> {
    let entry = lookup(name)?;
    if entry.availability.on(target) {
        return None;
    }
    let msg = if entry.availability.native && !entry.availability.js {
        format!(
            "host API `{name}` is unsupported on {} target (native-only; {})",
            target.as_str(),
            entry.note
        )
    } else {
        format!(
            "host API `{name}` is unsupported on {} target ({})",
            target.as_str(),
            entry.note
        )
    };
    Some(Diagnostic::new(msg, span).with_code(codes::HOST_API_UNSUPPORTED))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bind, check_for_target};
    use draconic_parser::parse;

    #[test]
    fn unknown_name_is_not_a_host_api() {
        assert!(!is_host_api("notAHostApi"));
    }

    #[test]
    fn unsupported_diagnostic_on_js_for_native_only() {
        let d = unsupported_diagnostic("tlsClientWrap", CompileTarget::Js, Span::new(0, 13))
            .expect("js must reject tlsClientWrap");
        assert_eq!(d.code, Some(codes::HOST_API_UNSUPPORTED));
        assert!(
            d.message.contains("host API") && d.message.contains("unsupported on js"),
            "message={:?}",
            d.message
        );
        assert!(d.message.contains("native-only"), "message={:?}", d.message);
        assert!(
            unsupported_diagnostic("tlsClientWrap", CompileTarget::Native, Span::dummy()).is_none()
        );
        assert!(unsupported_diagnostic("console", CompileTarget::Js, Span::dummy()).is_none());
    }

    #[test]
    fn shadowed_host_name_is_not_a_host_api_use() {
        // Local binding wins; not a free host API reference.
        let program = parse("let tcpListen = 1; tcpListen;").unwrap();
        check_for_target(program, CompileTarget::Js).expect("shadowed name ok on js");
    }

    #[test]
    fn bind_leaves_host_api_free() {
        let program = parse("tcpListen;").unwrap();
        let bound = bind(program).unwrap();
        // Free host idents stay unresolved (runtime/host surface).
        let use_span = bound
            .program
            .body
            .iter()
            .find_map(|s| match s {
                draconic_ast::Stmt::Expression {
                    expr: draconic_ast::Expr::Ident(id),
                    ..
                } => Some(id.span),
                _ => None,
            })
            .expect("ident use");
        assert!(bound.resolve(use_span).is_none());
    }

    #[test]
    fn h00_host_apis_are_free_identifiers() {
        for entry in host_apis() {
            assert!(
                entry
                    .name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "H00 shape: `{}` must be a free identifier, not a module path",
                entry.name
            );
            assert!(
                !entry.name.contains('.') && !entry.name.contains('/') && !entry.name.contains(':'),
                "H00 shape: `{}` must not be a module specifier",
                entry.name
            );
            let first = entry.name.chars().next().expect("non-empty host API name");
            assert!(
                first.is_ascii_alphabetic() || first == '_',
                "H00 shape: `{}` must be a JS IdentifierName",
                entry.name
            );
        }
    }

    fn catalog_abi_src() -> &'static str {
        concat!(
            include_str!("../../draconic-runtime/src/abi.rs"),
            include_str!("../../draconic-runtime/src/abi_host.rs"),
            include_str!("../../draconic-runtime/src/abi_host_io.rs"),
            include_str!("../../draconic-runtime/src/abi_host_net.rs"),
            include_str!("../../draconic-runtime/src/abi_host_symbols.rs"),
        )
    }

    fn catalog_abi_symbols() -> Vec<&'static str> {
        use draconic_runtime::{HOST_DECLARES, HOST_SYMBOLS, TIMER_SYMBOLS};
        let mut symbols: Vec<&'static str> = HOST_SYMBOLS.iter().copied().collect();
        symbols.extend(TIMER_SYMBOLS.iter().copied());
        symbols.extend(HOST_DECLARES.iter().map(|f| f.symbol));
        for line in catalog_abi_src().lines() {
            let Some(rest) = line.trim().strip_prefix("symbol: \"") else {
                continue;
            };
            let Some(sym) = rest.strip_suffix("\",") else {
                continue;
            };
            if sym.starts_with("draconic_rt_") {
                symbols.push(sym);
            }
        }
        symbols
    }

    fn catalog_js_polyfill_src() -> &'static str {
        concat!(
            include_str!("../../draconic-runtime/src/host_js_bridge.rs"),
            include_str!("../../draconic-runtime/src/host_process_polyfill.rs"),
            include_str!("../../draconic-runtime/src/host_worker_polyfill.rs"),
            include_str!("../../draconic-runtime/src/host_cancel_polyfill.rs"),
            include_str!("../../draconic-runtime/src/host_timer_polyfill.rs"),
            include_str!("../../draconic-runtime/src/host_stdio_polyfill.rs"),
            include_str!("../../draconic-runtime/src/host_path_polyfill.rs"),
            include_str!("../../draconic-runtime/src/host_fs_polyfill.rs"),
        )
    }

    fn abi_symbol_tail(symbol: &str) -> &str {
        let rest = symbol.strip_prefix("draconic_rt_").unwrap_or(symbol);
        rest.strip_prefix("host_").unwrap_or(rest)
    }

    fn camel_to_snake(name: &str) -> String {
        let mut snake = String::new();
        for (i, c) in name.chars().enumerate() {
            if c.is_ascii_uppercase() {
                if i > 0 {
                    snake.push('_');
                }
                snake.push(c.to_ascii_lowercase());
            } else {
                snake.push(c);
            }
        }
        snake
    }

    fn abi_needles(name: &str) -> Vec<String> {
        let snake = camel_to_snake(name);
        let mut needles = vec![snake.clone()];
        if let Some(rest) = snake.strip_prefix("make_") {
            needles.push(format!("{rest}_make"));
        }
        if let Some(rest) = snake.strip_prefix("spawn_") {
            needles.push(format!("{rest}_spawn"));
        }
        if let Some(rest) = snake.strip_prefix("join_") {
            needles.push(format!("{rest}_join"));
        }
        if let Some(rest) = snake.strip_prefix("terminate_") {
            needles.push(format!("{rest}_terminate"));
        }
        if let Some(rest) = snake.strip_prefix("on_") {
            needles.push(format!("{rest}_watch"));
        }
        for verb in ["ignore", "raise", "restore"] {
            if let Some(rest) = snake.strip_prefix(&format!("{verb}_")) {
                needles.push(format!("{rest}_{verb}"));
            }
        }
        if snake.starts_with("close_") {
            needles.push("handle_close".to_string());
        }
        match name {
            "processArgs" => needles.push("process_user_arg".to_string()),
            "makeCancelToken" => needles.push("cancel_make".to_string()),
            "makeSharedMemory" => needles.push("shared_make".to_string()),
            "sharedCompareExchange" => needles.push("shared_cmpxchg".to_string()),
            "setTimeout" => needles.push("timer_set".to_string()),
            "clearTimeout" | "clearInterval" => needles.push("timer_clear".to_string()),
            "setInterval" => needles.push("timer_set_interval".to_string()),
            "readFileText" => needles.push("fs_read_text".to_string()),
            "readFileBytes" => needles.push("fs_read_file".to_string()),
            "writeFileText" => needles.push("fs_write_text".to_string()),
            "writeFileBytes" => needles.push("fs_write_file".to_string()),
            "appendFileText" => needles.push("fs_append_text".to_string()),
            "appendFileBytes" => needles.push("fs_append_file".to_string()),
            "openFile" => needles.push("fs_open".to_string()),
            "fileRead" => needles.push("fs_handle_read".to_string()),
            "fileWrite" => needles.push("fs_handle_write".to_string()),
            "fileSeek" => needles.push("fs_handle_seek".to_string()),
            "udpSendTo" => needles.push("udp_sendto".to_string()),
            "udpRecvFrom" => needles.push("udp_recvfrom".to_string()),
            "withTimeout" => needles.push("cancel_timeout".to_string()),
            "clearWithTimeout" => needles.push("cancel_clear_timeout".to_string()),
            "cancelTokenAbort" => needles.push("cancel_abort".to_string()),
            "cancelTokenAborted" => needles.push("cancel_aborted".to_string()),
            "cancelTokenLink" => needles.push("cancel_link".to_string()),
            _ => {}
        }
        needles
    }

    fn catalog_has_abi_symbol(name: &str, symbols: &[&str]) -> bool {
        let needles = abi_needles(name);
        symbols.iter().any(|symbol| {
            let tail = abi_symbol_tail(symbol);
            let tail_tokens: Vec<&str> = tail.split('_').collect();
            needles.iter().any(|needle| {
                let needle_tokens: Vec<&str> = needle.split('_').collect();
                tail_tokens == needle_tokens
                    || tail_tokens.ends_with(&needle_tokens)
                    || ((needle == "channel_send" || needle == "channel_recv")
                        && tail_tokens.starts_with(&needle_tokens))
            })
        })
    }

    fn catalog_has_js_polyfill_export(name: &str, src: &str) -> bool {
        let fn_export = format!("function {name}(");
        let global_export = format!("globalThis.{name}");
        src.contains(&fn_export) || src.contains(&global_export)
    }

    #[test]
    fn catalog_sync() {
        let symbols = catalog_abi_symbols();
        let polyfills = catalog_js_polyfill_src();
        let mut missing_abi = Vec::new();
        let mut missing_js = Vec::new();
        for entry in host_apis() {
            if entry.availability.native && !catalog_has_abi_symbol(entry.name, &symbols) {
                missing_abi.push(entry.name);
            }
            if entry.availability.js && !catalog_has_js_polyfill_export(entry.name, &polyfills) {
                missing_js.push(entry.name);
            }
        }
        assert!(
            missing_abi.is_empty(),
            "catalog names lack a matching Runtime ABI symbol: {missing_abi:?}"
        );
        assert!(
            missing_js.is_empty(),
            "BOTH names lack a JS polyfill export: {missing_js:?}"
        );
    }

    #[test]
    fn h00_no_js_only_host_api_and_native_only_hard_errors() {
        for entry in host_apis() {
            assert!(
                entry.availability.native,
                "H00 matrix: `{}` must be available on native (no js-only host API)",
                entry.name
            );
            if entry.availability.js {
                assert!(
                    unsupported_diagnostic(entry.name, CompileTarget::Js, Span::dummy()).is_none(),
                    "H00 matrix: `{}` is BOTH — js must not hard-error",
                    entry.name
                );
            } else {
                let d = unsupported_diagnostic(entry.name, CompileTarget::Js, Span::dummy())
                    .unwrap_or_else(|| {
                        panic!(
                            "H00 matrix: `{}` native-only must hard-error on js",
                            entry.name
                        )
                    });
                assert_eq!(d.code, Some(codes::HOST_API_UNSUPPORTED));
                assert!(
                    d.message.contains("unsupported on js"),
                    "H00 matrix: `{}` message={:?}",
                    entry.name,
                    d.message
                );
            }
        }
    }
}
