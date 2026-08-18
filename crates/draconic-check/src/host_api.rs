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
/// - `envGet` / `envSet` / `envDelete` (H01.02): both — string env; missing get → undefined.
/// - `exit` / `exitCode` / `setExitCode` (H01.03): both — terminate / deferred status (default 0).
/// - `pid` / `ppid` (H01.04): both — read-only OS process / parent process id (number).
/// - `stdoutWrite` (H02.01): both — write string or Uint8Array bytes to stdout.
/// - `stderrWrite` (H02.02): both — write string or Uint8Array bytes to stderr.
/// - `stdinReadLine` / `stdinReadBytes` (H02.03): both — blocking line/bytes from stdin.
/// - `pathJoin` / `pathNormalize` (H03.01): both — pure path string ops (no I/O).
/// - `pathDirname` / `pathBasename` / `pathExtname` / `pathIsAbsolute` (H03.02): both.
/// - `readFileText` / `readFileBytes` (H04.01): both — whole-file read; missing → HostError ENOENT.
/// - `writeFileText` / `writeFileBytes` / `appendFileText` / `appendFileBytes` (H04.02): both — create/truncate or append.
/// - `exists` / `stat` (H04.03): both — path exists bool; stat `{size,isFile,isDir,mtime}` (missing → ENOENT).
/// - `mkdir` / `mkdirAll` / `readdir` / `rmdir` / `removeFile` (H04.04): both — dir create/list/remove + file delete.
/// - `renameFile` / `copyFile` (H04.05): both — rename/move and copy regular files (`removeFile` is delete).
/// - `openFile` / `fileRead` / `fileWrite` / `fileSeek` / `closeFile` (H04.06): native-only open handles.
/// - `nowMs` (H05.01): both — wall clock ms since Unix epoch (`Date.now` equivalent).
/// - `monotonicMs` (H05.02): both — monotonic clock ms for durations (not wall epoch).
/// - `tcpListen` / `tcpLocalPort` / `closeTcp` (H06.01): native-only TCP listen + port query + close.
/// - `tcpAccept` / `tcpPeerAddress` / `tcpPeerPort` (H06.02): accept + peer.
/// - `tcpConnect` (H06.02–H06.03): dial IPv4 host:port; refused/timeout → HostError ECONN.
/// - `tcpRead` / `tcpWrite` / `tcpShutdown` (H06.04): connection bytes + half-close.
/// - H06.06: all TCP listen/accept (and related) APIs hard-error on js until optional Node bridge.
const HOST_APIS: &[HostApiEntry] = &[
    HostApiEntry {
        name: "processArgs",
        availability: HostAvailability::BOTH,
        note: "H01.01 process args",
    },
    HostApiEntry {
        name: "envGet",
        availability: HostAvailability::BOTH,
        note: "H01.02 env get",
    },
    HostApiEntry {
        name: "envSet",
        availability: HostAvailability::BOTH,
        note: "H01.02 env set",
    },
    HostApiEntry {
        name: "envDelete",
        availability: HostAvailability::BOTH,
        note: "H01.02 env delete",
    },
    HostApiEntry {
        name: "exit",
        availability: HostAvailability::BOTH,
        note: "H01.03 process exit",
    },
    HostApiEntry {
        name: "exitCode",
        availability: HostAvailability::BOTH,
        note: "H01.03 get exit code",
    },
    HostApiEntry {
        name: "setExitCode",
        availability: HostAvailability::BOTH,
        note: "H01.03 set exit code",
    },
    HostApiEntry {
        name: "pid",
        availability: HostAvailability::BOTH,
        note: "H01.04 process pid",
    },
    HostApiEntry {
        name: "ppid",
        availability: HostAvailability::BOTH,
        note: "H01.04 process ppid",
    },
    HostApiEntry {
        name: "stdoutWrite",
        availability: HostAvailability::BOTH,
        note: "H02.01 stdout write",
    },
    HostApiEntry {
        name: "stderrWrite",
        availability: HostAvailability::BOTH,
        note: "H02.02 stderr write",
    },
    HostApiEntry {
        name: "stdinReadLine",
        availability: HostAvailability::BOTH,
        note: "H02.03 stdin read line",
    },
    HostApiEntry {
        name: "stdinReadBytes",
        availability: HostAvailability::BOTH,
        note: "H02.03 stdin read bytes",
    },
    HostApiEntry {
        name: "pathJoin",
        availability: HostAvailability::BOTH,
        note: "H03.01 path join",
    },
    HostApiEntry {
        name: "pathNormalize",
        availability: HostAvailability::BOTH,
        note: "H03.01 path normalize",
    },
    HostApiEntry {
        name: "pathDirname",
        availability: HostAvailability::BOTH,
        note: "H03.02 path dirname",
    },
    HostApiEntry {
        name: "pathBasename",
        availability: HostAvailability::BOTH,
        note: "H03.02 path basename",
    },
    HostApiEntry {
        name: "pathExtname",
        availability: HostAvailability::BOTH,
        note: "H03.02 path extname",
    },
    HostApiEntry {
        name: "pathIsAbsolute",
        availability: HostAvailability::BOTH,
        note: "H03.02 path isAbsolute",
    },
    HostApiEntry {
        name: "readFileText",
        availability: HostAvailability::BOTH,
        note: "H04.01 file read text",
    },
    HostApiEntry {
        name: "readFileBytes",
        availability: HostAvailability::BOTH,
        note: "H04.01 file read bytes",
    },
    HostApiEntry {
        name: "writeFileText",
        availability: HostAvailability::BOTH,
        note: "H04.02 file write text",
    },
    HostApiEntry {
        name: "writeFileBytes",
        availability: HostAvailability::BOTH,
        note: "H04.02 file write bytes",
    },
    HostApiEntry {
        name: "appendFileText",
        availability: HostAvailability::BOTH,
        note: "H04.02 file append text",
    },
    HostApiEntry {
        name: "appendFileBytes",
        availability: HostAvailability::BOTH,
        note: "H04.02 file append bytes",
    },
    HostApiEntry {
        name: "exists",
        availability: HostAvailability::BOTH,
        note: "H04.03 path exists",
    },
    HostApiEntry {
        name: "stat",
        availability: HostAvailability::BOTH,
        note: "H04.03 path stat",
    },
    HostApiEntry {
        name: "mkdir",
        availability: HostAvailability::BOTH,
        note: "H04.04 mkdir",
    },
    HostApiEntry {
        name: "mkdirAll",
        availability: HostAvailability::BOTH,
        note: "H04.04 mkdir recursive",
    },
    HostApiEntry {
        name: "readdir",
        availability: HostAvailability::BOTH,
        note: "H04.04 readdir",
    },
    HostApiEntry {
        name: "rmdir",
        availability: HostAvailability::BOTH,
        note: "H04.04 rmdir",
    },
    HostApiEntry {
        name: "removeFile",
        availability: HostAvailability::BOTH,
        note: "H04.04 remove file",
    },
    HostApiEntry {
        name: "renameFile",
        availability: HostAvailability::BOTH,
        note: "H04.05 rename file",
    },
    HostApiEntry {
        name: "copyFile",
        availability: HostAvailability::BOTH,
        note: "H04.05 copy file",
    },
    HostApiEntry {
        name: "openFile",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H04.06 open file handle",
    },
    HostApiEntry {
        name: "fileRead",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H04.06 file handle read",
    },
    HostApiEntry {
        name: "fileWrite",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H04.06 file handle write",
    },
    HostApiEntry {
        name: "fileSeek",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H04.06 file handle seek",
    },
    HostApiEntry {
        name: "closeFile",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H04.06 close file handle",
    },
    HostApiEntry {
        name: "nowMs",
        availability: HostAvailability::BOTH,
        note: "H05.01 wall clock ms",
    },
    HostApiEntry {
        name: "monotonicMs",
        availability: HostAvailability::BOTH,
        note: "H05.02 monotonic clock ms",
    },
    HostApiEntry {
        name: "setTimeout",
        availability: HostAvailability::BOTH,
        note: "H05.03 setTimeout via job queue",
    },
    HostApiEntry {
        name: "clearTimeout",
        availability: HostAvailability::BOTH,
        note: "H05.03 clearTimeout",
    },
    HostApiEntry {
        name: "setInterval",
        availability: HostAvailability::BOTH,
        note: "H05.04 setInterval via job queue",
    },
    HostApiEntry {
        name: "clearInterval",
        availability: HostAvailability::BOTH,
        note: "H05.04 clearInterval",
    },
    HostApiEntry {
        name: "tcpListen",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H06.01 TCP listen",
    },
    HostApiEntry {
        name: "tcpLocalPort",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H06.01 TCP local port",
    },
    HostApiEntry {
        name: "closeTcp",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H06.01 close TCP listen/conn handle",
    },
    HostApiEntry {
        name: "tcpAccept",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H06.02 TCP accept → connection handle",
    },
    HostApiEntry {
        name: "tcpConnect",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H06.03 TCP connect dial host:port; refused/timeout → ECONN",
    },
    HostApiEntry {
        name: "tcpPeerAddress",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H06.02 TCP peer IPv4 address string",
    },
    HostApiEntry {
        name: "tcpPeerPort",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H06.02 TCP peer port",
    },
    HostApiEntry {
        name: "tcpRead",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H06.04 TCP read bytes (partial OK)",
    },
    HostApiEntry {
        name: "tcpWrite",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H06.04 TCP write bytes",
    },
    HostApiEntry {
        name: "tcpShutdown",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H06.04 TCP shutdown (0=RD 1=WR 2=RDWR)",
    },
    HostApiEntry {
        name: "tcpAcceptAsync",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H07.02 TCP accept → Promise (handle)",
    },
    HostApiEntry {
        name: "tcpConnectAsync",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H07.02 TCP connect → Promise (handle)",
    },
    HostApiEntry {
        name: "tcpReadAsync",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H07.02 TCP read → Promise (byte count)",
    },
    HostApiEntry {
        name: "tcpWriteAsync",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H07.02 TCP write → Promise (byte count)",
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
    fn registry_lists_env_apis_both() {
        for name in ["envGet", "envSet", "envDelete"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_exit_apis_both() {
        for name in ["exit", "exitCode", "setExitCode"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_pid_ppid_both() {
        for name in ["pid", "ppid"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_stdout_write_both() {
        let entry = lookup("stdoutWrite").expect("stdoutWrite registered");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_available("stdoutWrite", CompileTarget::Js));
        assert!(is_available("stdoutWrite", CompileTarget::Native));
        assert!(
            unsupported_diagnostic("stdoutWrite", CompileTarget::Js, Span::dummy()).is_none()
        );
    }

    #[test]
    fn registry_lists_stderr_write_both() {
        let entry = lookup("stderrWrite").expect("stderrWrite registered");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_available("stderrWrite", CompileTarget::Js));
        assert!(is_available("stderrWrite", CompileTarget::Native));
        assert!(
            unsupported_diagnostic("stderrWrite", CompileTarget::Js, Span::dummy()).is_none()
        );
    }

    #[test]
    fn registry_lists_stdin_read_both() {
        for name in ["stdinReadLine", "stdinReadBytes"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_path_join_normalize_both() {
        for name in ["pathJoin", "pathNormalize"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_path_dirname_basename_extname_is_absolute_both() {
        for name in [
            "pathDirname",
            "pathBasename",
            "pathExtname",
            "pathIsAbsolute",
        ] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_read_file_both() {
        for name in ["readFileText", "readFileBytes"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_write_file_both() {
        for name in [
            "writeFileText",
            "writeFileBytes",
            "appendFileText",
            "appendFileBytes",
        ] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_exists_stat_both() {
        for name in ["exists", "stat"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_dir_ops_both() {
        for name in ["mkdir", "mkdirAll", "readdir", "rmdir", "removeFile"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_rename_copy_both() {
        for name in ["renameFile", "copyFile"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_open_handle_native_only() {
        for name in ["openFile", "fileRead", "fileWrite", "fileSeek", "closeFile"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(!entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(!is_available(name, CompileTarget::Js), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_some(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_now_ms_both() {
        let entry = lookup("nowMs").expect("nowMs registered");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_available("nowMs", CompileTarget::Js));
        assert!(is_available("nowMs", CompileTarget::Native));
        assert!(unsupported_diagnostic("nowMs", CompileTarget::Js, Span::dummy()).is_none());
    }

    #[test]
    fn registry_lists_monotonic_ms_both() {
        let entry = lookup("monotonicMs").expect("monotonicMs registered");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_available("monotonicMs", CompileTarget::Js));
        assert!(is_available("monotonicMs", CompileTarget::Native));
        assert!(
            unsupported_diagnostic("monotonicMs", CompileTarget::Js, Span::dummy()).is_none()
        );
    }

    #[test]
    fn registry_lists_set_timeout_both() {
        for name in ["setTimeout", "clearTimeout"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_set_interval_both() {
        for name in ["setInterval", "clearInterval"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_tcp_listen_native_only() {
        for name in [
            "tcpListen",
            "tcpLocalPort",
            "closeTcp",
            "tcpAccept",
            "tcpConnect",
            "tcpPeerAddress",
            "tcpPeerPort",
            "tcpRead",
            "tcpWrite",
            "tcpShutdown",
            "tcpAcceptAsync",
            "tcpConnectAsync",
            "tcpReadAsync",
            "tcpWriteAsync",
        ] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(!entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_host_api(name), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(!is_available(name, CompileTarget::Js), "{name}");
        }
        assert!(!is_host_api("notAHostApi"));
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
    fn check_for_target_js_rejects_free_tcp_accept() {
        let program = parse("tcpAccept(0);").unwrap();
        let err = check_for_target(program, CompileTarget::Js).expect_err("js hard diagnostic");
        assert_eq!(err.code, Some(codes::HOST_API_UNSUPPORTED));
        assert!(
            err.message.contains("tcpAccept") && err.message.contains("unsupported on js"),
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
    fn check_for_target_native_allows_free_tcp_accept() {
        let program = parse("tcpAccept(0);").unwrap();
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
