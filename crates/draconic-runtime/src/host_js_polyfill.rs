//! Host JS polyfills keyed by catalog free-identifier names (`HOST_APIS`).

use crate::{
    cancel_token_js_polyfill, channel_js_polyfill, cwd_chdir_js_polyfill, dns_js_polyfill,
    fs_read_js_polyfill, hostname_os_js_polyfill, http_js_polyfill, monotonic_ms_js_polyfill,
    now_ms_js_polyfill, path_js_polyfill, process_args_js_polyfill, process_env_js_polyfill,
    process_exit_js_polyfill, process_pid_js_polyfill, process_run_js_polyfill,
    process_spawn_js_polyfill, set_interval_js_polyfill, set_timeout_js_polyfill,
    spawn_worker_js_polyfill, stderr_write_js_polyfill, stdin_read_js_polyfill,
    stdout_write_js_polyfill, tcp_js_polyfill, temp_home_js_polyfill,
};

/// JS polyfill body for a host catalog name, or `None` when the name is unknown
/// or native-only.
pub fn host_js_polyfill(name: &str) -> Option<&'static str> {
    Some(match name {
        "processArgs" => process_args_js_polyfill(),
        "envGet" | "envSet" | "envDelete" => process_env_js_polyfill(),
        "exit" | "exitCode" | "setExitCode" => process_exit_js_polyfill(),
        "pid" | "ppid" => process_pid_js_polyfill(),
        "cwd" | "chdir" => cwd_chdir_js_polyfill(),
        "hostname" | "osType" | "osArch" => hostname_os_js_polyfill(),
        "tempDir" | "homeDir" => temp_home_js_polyfill(),
        "processRun" => process_run_js_polyfill(),
        "processSpawn" | "processStdinWrite" | "processWait" | "processStdout"
        | "processStderr" | "processKill" | "processClose" => process_spawn_js_polyfill(),
        "stdoutWrite" => stdout_write_js_polyfill(),
        "stderrWrite" => stderr_write_js_polyfill(),
        "stdinReadLine" | "stdinReadBytes" => stdin_read_js_polyfill(),
        "spawnWorker" | "joinWorker" | "terminateWorker" => spawn_worker_js_polyfill(),
        "makeChannel" | "channelSend" | "channelRecv" => channel_js_polyfill(),
        "makeCancelToken" | "cancelTokenAbort" | "cancelTokenAborted" | "cancelTokenLink"
        | "withTimeout" | "clearWithTimeout" => cancel_token_js_polyfill(),
        "pathJoin" | "pathNormalize" | "pathDirname" | "pathBasename" | "pathExtname"
        | "pathIsAbsolute" | "pathResolve" => path_js_polyfill(),
        "readFileText" | "readFileBytes" | "writeFileText" | "writeFileBytes"
        | "appendFileText" | "appendFileBytes" | "exists" | "stat" | "mkdir" | "mkdirAll"
        | "readdir" | "rmdir" | "removeFile" | "renameFile" | "copyFile" => fs_read_js_polyfill(),
        "nowMs" => now_ms_js_polyfill(),
        "monotonicMs" => monotonic_ms_js_polyfill(),
        "setTimeout" | "clearTimeout" => set_timeout_js_polyfill(),
        "setInterval" | "clearInterval" => set_interval_js_polyfill(),
        "tcpListen" | "tcpLocalPort" | "closeTcp" | "tcpAccept" | "tcpConnect"
        | "tcpPeerAddress" | "tcpPeerPort" | "tcpRead" | "tcpWrite" | "tcpShutdown" => {
            tcp_js_polyfill()
        }
        "dnsLookup" => dns_js_polyfill(),
        "httpParseRequest" | "httpRequestHeader" | "httpWriteResponse" | "httpWriteRequest"
        | "httpParseResponse" | "httpResponseHeader" => http_js_polyfill(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::host_js_polyfill;

    const CATALOG_JS_NAMES: &[&str] = &[
        "processArgs",
        "envGet",
        "envSet",
        "envDelete",
        "exit",
        "exitCode",
        "setExitCode",
        "pid",
        "ppid",
        "cwd",
        "chdir",
        "hostname",
        "osType",
        "osArch",
        "tempDir",
        "homeDir",
        "processRun",
        "processSpawn",
        "processStdinWrite",
        "processWait",
        "processStdout",
        "processStderr",
        "processKill",
        "processClose",
        "stdoutWrite",
        "stderrWrite",
        "stdinReadLine",
        "stdinReadBytes",
        "spawnWorker",
        "joinWorker",
        "terminateWorker",
        "makeChannel",
        "channelSend",
        "channelRecv",
        "makeCancelToken",
        "cancelTokenAbort",
        "cancelTokenAborted",
        "cancelTokenLink",
        "withTimeout",
        "clearWithTimeout",
        "pathJoin",
        "pathNormalize",
        "pathDirname",
        "pathBasename",
        "pathExtname",
        "pathIsAbsolute",
        "pathResolve",
        "readFileText",
        "readFileBytes",
        "writeFileText",
        "writeFileBytes",
        "appendFileText",
        "appendFileBytes",
        "exists",
        "stat",
        "mkdir",
        "mkdirAll",
        "readdir",
        "rmdir",
        "removeFile",
        "renameFile",
        "copyFile",
        "nowMs",
        "monotonicMs",
        "setTimeout",
        "clearTimeout",
        "setInterval",
        "clearInterval",
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
        "dnsLookup",
        "httpParseRequest",
        "httpRequestHeader",
        "httpWriteResponse",
        "httpWriteRequest",
        "httpParseResponse",
        "httpResponseHeader",
    ];

    fn exports_catalog_name(src: &str, name: &str) -> bool {
        src.contains(&format!("function {name}(")) || src.contains(&format!("globalThis.{name}"))
    }

    #[test]
    fn lookup_by_catalog_name_returns_host_js_polyfills() {
        for name in CATALOG_JS_NAMES {
            let src = host_js_polyfill(name)
                .unwrap_or_else(|| panic!("catalog name `{name}` must have a JS polyfill"));
            assert!(
                exports_catalog_name(src, name),
                "polyfill for `{name}` must export that catalog name"
            );
        }
    }

    #[test]
    fn lookup_skips_unknown_and_native_only_catalog_names() {
        assert!(host_js_polyfill("notAHostApi").is_none());
        for name in [
            "tlsClientWrap",
            "openFile",
            "processWaitAsync",
            "workerOsThread",
            "makeOnce",
            "tcpAcceptAsync",
            "udpBind",
            "httpServeStatic",
            "wsHandshakeResponse",
            "http2ClientPreface",
        ] {
            assert!(
                host_js_polyfill(name).is_none(),
                "native-only `{name}` must not have a JS polyfill"
            );
        }
    }

    #[test]
    fn lookup_shares_polyfill_body_within_catalog_group() {
        let env = host_js_polyfill("envGet").expect("envGet");
        assert!(std::ptr::eq(
            env,
            host_js_polyfill("envSet").expect("envSet")
        ));
        assert!(std::ptr::eq(
            env,
            host_js_polyfill("envDelete").expect("envDelete")
        ));
        let fs = host_js_polyfill("readFileText").expect("readFileText");
        assert!(std::ptr::eq(
            fs,
            host_js_polyfill("writeFileText").expect("writeFileText")
        ));
        let http = host_js_polyfill("httpParseRequest").expect("httpParseRequest");
        assert!(std::ptr::eq(
            http,
            host_js_polyfill("httpWriteResponse").expect("httpWriteResponse")
        ));
    }
}
