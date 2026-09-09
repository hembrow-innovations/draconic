//! Process, OS, subprocess, signal, and stdio host API registry rows
//! (H01, H16, H15, H14, H02).

use super::{HostApiEntry, HostAvailability};

pub(super) const ENTRIES: &[HostApiEntry] = &[
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
        name: "cwd",
        availability: HostAvailability::BOTH,
        note: "H16.01 get cwd",
    },
    HostApiEntry {
        name: "chdir",
        availability: HostAvailability::BOTH,
        note: "H16.01 chdir",
    },
    HostApiEntry {
        name: "hostname",
        availability: HostAvailability::BOTH,
        note: "H16.02 hostname",
    },
    HostApiEntry {
        name: "osType",
        availability: HostAvailability::BOTH,
        note: "H16.02 OS type/platform",
    },
    HostApiEntry {
        name: "osArch",
        availability: HostAvailability::BOTH,
        note: "H16.02 OS arch",
    },
    HostApiEntry {
        name: "tempDir",
        availability: HostAvailability::BOTH,
        note: "H16.03 temp directory path",
    },
    HostApiEntry {
        name: "homeDir",
        availability: HostAvailability::BOTH,
        note: "H16.03 home directory path",
    },
    HostApiEntry {
        name: "processRun",
        availability: HostAvailability::BOTH,
        note: "H15.01 spawn/run argv + optional cwd/env; wait exit code",
    },
    HostApiEntry {
        name: "processSpawn",
        availability: HostAvailability::BOTH,
        note: "H15.02 spawn with pipes; returns handle",
    },
    HostApiEntry {
        name: "processStdinWrite",
        availability: HostAvailability::BOTH,
        note: "H15.02 write child stdin then close",
    },
    HostApiEntry {
        name: "processWait",
        availability: HostAvailability::BOTH,
        note: "H15.02 wait + drain stdout/stderr; exit code",
    },
    HostApiEntry {
        name: "processWaitAsync",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H15.03 async wait → Promise of exit code via job queue",
    },
    HostApiEntry {
        name: "processStdout",
        availability: HostAvailability::BOTH,
        note: "H15.02 captured stdout string after wait",
    },
    HostApiEntry {
        name: "processStderr",
        availability: HostAvailability::BOTH,
        note: "H15.02 captured stderr string after wait",
    },
    HostApiEntry {
        name: "processKill",
        availability: HostAvailability::BOTH,
        note: "H15.02 SIGTERM child",
    },
    HostApiEntry {
        name: "processClose",
        availability: HostAvailability::BOTH,
        note: "H15.02 free spawn handle",
    },
    HostApiEntry {
        name: "onSignal",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H14.01 signal watch SIGINT/SIGTERM → job",
    },
    HostApiEntry {
        name: "raiseSignal",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H14.01 raise SIGINT/SIGTERM to self",
    },
    HostApiEntry {
        name: "ignoreSignal",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H14.02 signal ignore SIG_IGN",
    },
    HostApiEntry {
        name: "restoreSignal",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H14.02 restore SIG_DFL disposition",
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
];

#[cfg(test)]
mod tests {
    use super::super::{is_available, is_host_api, lookup, unsupported_diagnostic, CompileTarget};
    use draconic_diagnostics::Span;

    #[test]
    fn registry_lists_process_args_both() {
        let entry = lookup("processArgs").expect("processArgs registered");
        assert_eq!(entry.name, "processArgs");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_host_api("processArgs"));
        assert!(is_available("processArgs", CompileTarget::Js));
        assert!(is_available("processArgs", CompileTarget::Native));
        assert!(unsupported_diagnostic("processArgs", CompileTarget::Js, Span::dummy()).is_none());
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
    fn registry_lists_cwd_chdir_both() {
        for name in ["cwd", "chdir"] {
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
    fn registry_lists_hostname_os_type_arch_both() {
        for name in ["hostname", "osType", "osArch"] {
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
    fn registry_lists_temp_home_dir_both() {
        for name in ["tempDir", "homeDir"] {
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
    fn registry_lists_process_run_both() {
        let entry = lookup("processRun").expect("processRun registered");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_available("processRun", CompileTarget::Js));
        assert!(is_available("processRun", CompileTarget::Native));
        assert!(unsupported_diagnostic("processRun", CompileTarget::Js, Span::dummy()).is_none());
    }

    #[test]
    fn registry_lists_process_spawn_io_kill_both() {
        for name in [
            "processSpawn",
            "processStdinWrite",
            "processWait",
            "processStdout",
            "processStderr",
            "processKill",
            "processClose",
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
    fn registry_lists_process_wait_async_native_only() {
        let entry = lookup("processWaitAsync").expect("processWaitAsync registered");
        assert!(!entry.availability.js);
        assert!(entry.availability.native);
        assert!(!is_available("processWaitAsync", CompileTarget::Js));
        assert!(is_available("processWaitAsync", CompileTarget::Native));
        assert!(
            unsupported_diagnostic("processWaitAsync", CompileTarget::Js, Span::dummy()).is_some()
        );
    }

    #[test]
    fn registry_lists_stdout_write_both() {
        let entry = lookup("stdoutWrite").expect("stdoutWrite registered");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_available("stdoutWrite", CompileTarget::Js));
        assert!(is_available("stdoutWrite", CompileTarget::Native));
        assert!(unsupported_diagnostic("stdoutWrite", CompileTarget::Js, Span::dummy()).is_none());
    }

    #[test]
    fn registry_lists_stderr_write_both() {
        let entry = lookup("stderrWrite").expect("stderrWrite registered");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_available("stderrWrite", CompileTarget::Js));
        assert!(is_available("stderrWrite", CompileTarget::Native));
        assert!(unsupported_diagnostic("stderrWrite", CompileTarget::Js, Span::dummy()).is_none());
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
    fn registry_lists_signal_native_only() {
        for name in ["onSignal", "raiseSignal", "ignoreSignal", "restoreSignal"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(!entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_host_api(name), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(!is_available(name, CompileTarget::Js), "{name}");
        }
    }
}
