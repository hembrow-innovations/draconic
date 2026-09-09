//! Worker, channel, once, cancel, and shared-memory host API registry rows
//! (C01–C06).

use super::{HostApiEntry, HostAvailability};

pub(super) const ENTRIES: &[HostApiEntry] = &[
    HostApiEntry {
        name: "spawnWorker",
        availability: HostAvailability::BOTH,
        note: "C01.01/C02.04 spawn worker isolate; optional channel handle",
    },
    HostApiEntry {
        name: "joinWorker",
        availability: HostAvailability::BOTH,
        note: "C01.02 join worker wait + result/error",
    },
    HostApiEntry {
        name: "terminateWorker",
        availability: HostAvailability::BOTH,
        note: "C01.03 terminate worker; no shared JS heap",
    },
    HostApiEntry {
        name: "workerOsThread",
        availability: HostAvailability::NATIVE_ONLY,
        note: "C01.04 live OS thread distinct from caller",
    },
    HostApiEntry {
        name: "makeChannel",
        availability: HostAvailability::BOTH,
        note: "C02.01/C02.03 make FIFO channel handle; optional capacity",
    },
    HostApiEntry {
        name: "channelSend",
        availability: HostAvailability::BOTH,
        note: "C02.01–C02.03 send scalar, string, or plain object clone; 0 ok, -2 full",
    },
    HostApiEntry {
        name: "channelRecv",
        availability: HostAvailability::BOTH,
        note: "C02.01–C02.03 recv FIFO head (number/bool/string/object clone)",
    },
    HostApiEntry {
        name: "makeOnce",
        availability: HostAvailability::NATIVE_ONLY,
        note: "C03.01 thread-safe once cell handle",
    },
    HostApiEntry {
        name: "onceRun",
        availability: HostAvailability::NATIVE_ONLY,
        note: "C03.01 run init at most once; 1 ran / 0 already / negative invalid",
    },
    HostApiEntry {
        name: "makeCancelToken",
        availability: HostAvailability::BOTH,
        note: "C05.01 Abort-like cancel token handle",
    },
    HostApiEntry {
        name: "cancelTokenAbort",
        availability: HostAvailability::BOTH,
        note: "C05.01 abort token; 0 ok sticky, -1 invalid",
    },
    HostApiEntry {
        name: "cancelTokenAborted",
        availability: HostAvailability::BOTH,
        note: "C05.01 1 aborted / 0 not / -1 invalid",
    },
    HostApiEntry {
        name: "cancelTokenLink",
        availability: HostAvailability::BOTH,
        note: "C05.01 link child to parent; parent abort propagates",
    },
    HostApiEntry {
        name: "withTimeout",
        availability: HostAvailability::BOTH,
        note: "C05.02 token that auto-aborts after ms",
    },
    HostApiEntry {
        name: "clearWithTimeout",
        availability: HostAvailability::BOTH,
        note: "C05.02 clear pending timeout; work won race",
    },
    HostApiEntry {
        name: "makeSharedMemory",
        availability: HostAvailability::NATIVE_ONLY,
        note: "C06 shared integer buffer handle",
    },
    HostApiEntry {
        name: "sharedLoad",
        availability: HostAvailability::NATIVE_ONLY,
        note: "C06 atomic load i32 at index",
    },
    HostApiEntry {
        name: "sharedStore",
        availability: HostAvailability::NATIVE_ONLY,
        note: "C06 atomic store i32 at index; 0 ok / -1 invalid",
    },
    HostApiEntry {
        name: "sharedAdd",
        availability: HostAvailability::NATIVE_ONLY,
        note: "C06 atomic add; returns old i32",
    },
    HostApiEntry {
        name: "sharedCompareExchange",
        availability: HostAvailability::NATIVE_ONLY,
        note: "C06 atomic CAS; returns old i32",
    },
    HostApiEntry {
        name: "sharedWait",
        availability: HostAvailability::NATIVE_ONLY,
        note: "C06 wait until not expected; 0 ok / 1 not-eq / 2 timeout / -1 invalid",
    },
    HostApiEntry {
        name: "sharedNotify",
        availability: HostAvailability::NATIVE_ONLY,
        note: "C06 wake waiters on index; count or -1 invalid",
    },
];

#[cfg(test)]
mod tests {
    use super::super::{is_available, is_host_api, lookup, unsupported_diagnostic, CompileTarget};
    use crate::check_for_target;
    use draconic_diagnostics::codes;
    use draconic_diagnostics::Span;
    use draconic_parser::parse;

    #[test]
    fn registry_lists_spawn_worker_both() {
        let entry = lookup("spawnWorker").expect("spawnWorker registered");
        assert_eq!(entry.name, "spawnWorker");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_host_api("spawnWorker"));
        assert!(is_available("spawnWorker", CompileTarget::Js));
        assert!(is_available("spawnWorker", CompileTarget::Native));
        assert!(unsupported_diagnostic("spawnWorker", CompileTarget::Js, Span::dummy()).is_none());
        assert!(
            unsupported_diagnostic("spawnWorker", CompileTarget::Native, Span::dummy()).is_none()
        );
    }

    #[test]
    fn registry_lists_join_worker_both() {
        let entry = lookup("joinWorker").expect("joinWorker registered");
        assert_eq!(entry.name, "joinWorker");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_host_api("joinWorker"));
        assert!(is_available("joinWorker", CompileTarget::Js));
        assert!(is_available("joinWorker", CompileTarget::Native));
        assert!(unsupported_diagnostic("joinWorker", CompileTarget::Js, Span::dummy()).is_none());
        assert!(
            unsupported_diagnostic("joinWorker", CompileTarget::Native, Span::dummy()).is_none()
        );
    }

    #[test]
    fn registry_lists_terminate_worker_both() {
        let entry = lookup("terminateWorker").expect("terminateWorker registered");
        assert_eq!(entry.name, "terminateWorker");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_host_api("terminateWorker"));
        assert!(is_available("terminateWorker", CompileTarget::Js));
        assert!(is_available("terminateWorker", CompileTarget::Native));
        assert!(
            unsupported_diagnostic("terminateWorker", CompileTarget::Js, Span::dummy()).is_none()
        );
        assert!(
            unsupported_diagnostic("terminateWorker", CompileTarget::Native, Span::dummy())
                .is_none()
        );
    }

    #[test]
    fn registry_lists_worker_os_thread_native_only() {
        let entry = lookup("workerOsThread").expect("workerOsThread registered");
        assert_eq!(entry.name, "workerOsThread");
        assert!(!entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_host_api("workerOsThread"));
        assert!(!is_available("workerOsThread", CompileTarget::Js));
        assert!(is_available("workerOsThread", CompileTarget::Native));
        assert!(
            unsupported_diagnostic("workerOsThread", CompileTarget::Js, Span::dummy()).is_some()
        );
        assert!(
            unsupported_diagnostic("workerOsThread", CompileTarget::Native, Span::dummy())
                .is_none()
        );
    }

    #[test]
    fn registry_lists_make_channel_both() {
        let entry = lookup("makeChannel").expect("makeChannel registered");
        assert_eq!(entry.name, "makeChannel");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_host_api("makeChannel"));
        assert!(is_available("makeChannel", CompileTarget::Js));
        assert!(is_available("makeChannel", CompileTarget::Native));
        assert!(unsupported_diagnostic("makeChannel", CompileTarget::Js, Span::dummy()).is_none());
        assert!(
            unsupported_diagnostic("makeChannel", CompileTarget::Native, Span::dummy()).is_none()
        );
    }

    #[test]
    fn registry_lists_channel_send_both() {
        let entry = lookup("channelSend").expect("channelSend registered");
        assert_eq!(entry.name, "channelSend");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_host_api("channelSend"));
        assert!(is_available("channelSend", CompileTarget::Js));
        assert!(is_available("channelSend", CompileTarget::Native));
        assert!(unsupported_diagnostic("channelSend", CompileTarget::Js, Span::dummy()).is_none());
        assert!(
            unsupported_diagnostic("channelSend", CompileTarget::Native, Span::dummy()).is_none()
        );
    }

    #[test]
    fn registry_lists_channel_recv_both() {
        let entry = lookup("channelRecv").expect("channelRecv registered");
        assert_eq!(entry.name, "channelRecv");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_host_api("channelRecv"));
        assert!(is_available("channelRecv", CompileTarget::Js));
        assert!(is_available("channelRecv", CompileTarget::Native));
        assert!(unsupported_diagnostic("channelRecv", CompileTarget::Js, Span::dummy()).is_none());
        assert!(
            unsupported_diagnostic("channelRecv", CompileTarget::Native, Span::dummy()).is_none()
        );
    }

    #[test]
    fn registry_lists_make_once_native_only() {
        let entry = lookup("makeOnce").expect("makeOnce registered");
        assert_eq!(entry.name, "makeOnce");
        assert!(!entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_host_api("makeOnce"));
        assert!(!is_available("makeOnce", CompileTarget::Js));
        assert!(is_available("makeOnce", CompileTarget::Native));
        assert!(unsupported_diagnostic("makeOnce", CompileTarget::Js, Span::dummy()).is_some());
        assert!(unsupported_diagnostic("makeOnce", CompileTarget::Native, Span::dummy()).is_none());
    }

    #[test]
    fn registry_lists_once_run_native_only() {
        let entry = lookup("onceRun").expect("onceRun registered");
        assert_eq!(entry.name, "onceRun");
        assert!(!entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_host_api("onceRun"));
        assert!(!is_available("onceRun", CompileTarget::Js));
        assert!(is_available("onceRun", CompileTarget::Native));
        assert!(unsupported_diagnostic("onceRun", CompileTarget::Js, Span::dummy()).is_some());
        assert!(unsupported_diagnostic("onceRun", CompileTarget::Native, Span::dummy()).is_none());
    }

    #[test]
    fn registry_lists_make_cancel_token_both() {
        let entry = lookup("makeCancelToken").expect("makeCancelToken registered");
        assert_eq!(entry.name, "makeCancelToken");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_host_api("makeCancelToken"));
        assert!(is_available("makeCancelToken", CompileTarget::Js));
        assert!(is_available("makeCancelToken", CompileTarget::Native));
        assert!(
            unsupported_diagnostic("makeCancelToken", CompileTarget::Js, Span::dummy()).is_none()
        );
        assert!(
            unsupported_diagnostic("makeCancelToken", CompileTarget::Native, Span::dummy())
                .is_none()
        );
    }

    #[test]
    fn registry_lists_cancel_token_abort_both() {
        let entry = lookup("cancelTokenAbort").expect("cancelTokenAbort registered");
        assert_eq!(entry.name, "cancelTokenAbort");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_host_api("cancelTokenAbort"));
        assert!(is_available("cancelTokenAbort", CompileTarget::Js));
        assert!(is_available("cancelTokenAbort", CompileTarget::Native));
    }

    #[test]
    fn registry_lists_cancel_token_aborted_both() {
        let entry = lookup("cancelTokenAborted").expect("cancelTokenAborted registered");
        assert_eq!(entry.name, "cancelTokenAborted");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_host_api("cancelTokenAborted"));
    }

    #[test]
    fn registry_lists_cancel_token_link_both() {
        let entry = lookup("cancelTokenLink").expect("cancelTokenLink registered");
        assert_eq!(entry.name, "cancelTokenLink");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_host_api("cancelTokenLink"));
    }

    #[test]
    fn registry_lists_with_timeout_both() {
        for name in ["withTimeout", "clearWithTimeout"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_host_api(name), "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_has_no_user_facing_mutex() {
        assert!(lookup("makeMutex").is_none());
        assert!(lookup("mutexLock").is_none());
        assert!(lookup("mutexUnlock").is_none());
        assert!(!is_host_api("makeMutex"));
        assert!(!is_host_api("mutexLock"));
        assert!(!is_host_api("mutexUnlock"));
    }

    #[test]
    fn registry_lists_shared_memory_atomics_native_only() {
        for name in [
            "makeSharedMemory",
            "sharedLoad",
            "sharedStore",
            "sharedAdd",
            "sharedCompareExchange",
            "sharedWait",
            "sharedNotify",
        ] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(!entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_host_api(name), "{name}");
            assert!(!is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_some(),
                "{name}"
            );
            assert!(
                unsupported_diagnostic(name, CompileTarget::Native, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn check_for_target_js_rejects_shared_memory_atomics() {
        for src in [
            "makeSharedMemory(1);",
            "sharedLoad(1, 0);",
            "sharedStore(1, 0, 1);",
            "sharedAdd(1, 0, 1);",
            "sharedCompareExchange(1, 0, 0, 1);",
            "sharedWait(1, 0, 0, 1);",
            "sharedNotify(1, 0);",
        ] {
            let program = parse(src).unwrap();
            let err = check_for_target(program, CompileTarget::Js)
                .expect_err(&format!("js must hard-error: {src}"));
            assert_eq!(err.code, Some(codes::HOST_API_UNSUPPORTED), "src={src}");
            assert!(
                err.message.contains("unsupported on js") && err.message.contains("native-only"),
                "src={src} got {}",
                err.message
            );
        }
    }
}
