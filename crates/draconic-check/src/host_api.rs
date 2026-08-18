//! Host API registry: known symbols + per-target availability (H00.01).
//!
//! Scaffold for H01+ surfaces. Native-only entries hard-error on the js target
//! (ADR-0008). Module/global shape of the full host surface is H00; this module
//! owns the name registry and availability checks only.

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
    /// Free identifier name used in Programs (provisional until H00 locks shape).
    pub name: &'static str,
    pub availability: HostAvailability,
    /// Short note for diagnostics (e.g. cluster id).
    pub note: &'static str,
}

/// Built-in host API registry. Expand as H01+ rows land.
///
/// - `processArgs` (H01.01): both targets — user program args as string[].
/// - `tcpListen` is a native-only scaffold for H06 (sockets-first); js must hard-error.
const HOST_APIS: &[HostApiEntry] = &[
    HostApiEntry {
        name: "processArgs",
        availability: HostAvailability::BOTH,
        note: "H01.01 process args",
    },
    HostApiEntry {
        name: "tcpListen",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H06 TCP listen",
    },
];

/// All known host API entries.
pub fn host_apis() -> &'static [HostApiEntry] {
    HOST_APIS
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
pub fn unsupported_diagnostic(
    name: &str,
    target: CompileTarget,
    span: Span,
) -> Option<Diagnostic> {
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
    fn registry_lists_process_args_both() {
        let entry = lookup("processArgs").expect("processArgs registered");
        assert_eq!(entry.name, "processArgs");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_host_api("processArgs"));
        assert!(is_available("processArgs", CompileTarget::Js));
        assert!(is_available("processArgs", CompileTarget::Native));
        assert!(
            unsupported_diagnostic("processArgs", CompileTarget::Js, Span::dummy()).is_none()
        );
        assert!(
            unsupported_diagnostic("processArgs", CompileTarget::Native, Span::dummy()).is_none()
        );
    }

    #[test]
    fn registry_lists_tcp_listen_native_only() {
        let entry = lookup("tcpListen").expect("tcpListen registered");
        assert_eq!(entry.name, "tcpListen");
        assert!(!entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_host_api("tcpListen"));
        assert!(!is_host_api("notAHostApi"));
        assert!(is_available("tcpListen", CompileTarget::Native));
        assert!(!is_available("tcpListen", CompileTarget::Js));
    }

    #[test]
    fn unsupported_diagnostic_on_js_for_native_only() {
        let d = unsupported_diagnostic("tcpListen", CompileTarget::Js, Span::new(0, 9))
            .expect("js must reject tcpListen");
        assert_eq!(d.code, Some(codes::HOST_API_UNSUPPORTED));
        assert!(
            d.message.contains("host API") && d.message.contains("unsupported on js"),
            "message={:?}",
            d.message
        );
        assert!(d.message.contains("native-only"), "message={:?}", d.message);
        assert!(
            unsupported_diagnostic("tcpListen", CompileTarget::Native, Span::dummy()).is_none()
        );
        assert!(unsupported_diagnostic("console", CompileTarget::Js, Span::dummy()).is_none());
    }

    #[test]
    fn check_for_target_js_rejects_free_tcp_listen() {
        let program = parse("tcpListen(8080);").unwrap();
        let err = check_for_target(program, CompileTarget::Js).expect_err("js hard diagnostic");
        assert_eq!(err.code, Some(codes::HOST_API_UNSUPPORTED));
        assert!(
            err.message.contains("tcpListen") && err.message.contains("unsupported on js"),
            "got {}",
            err.message
        );
    }

    #[test]
    fn check_for_target_native_allows_free_tcp_listen() {
        let program = parse("tcpListen(8080);").unwrap();
        check_for_target(program, CompileTarget::Native).expect("native allows host API ref");
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
}
