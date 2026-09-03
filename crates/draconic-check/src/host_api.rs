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

/// Built-in host API registry. Expand as H01+ rows land.
///
/// - `processArgs` (H01.01): both targets — user program args as string[].
/// - `envGet` / `envSet` / `envDelete` (H01.02): both — string env; missing get → undefined.
/// - `exit` / `exitCode` / `setExitCode` (H01.03): both — terminate / deferred status (default 0).
/// - `pid` / `ppid` (H01.04): both — read-only OS process / parent process id (number).
/// - `cwd` / `chdir` (H16.01): both — get/set process working directory.
/// - `hostname` / `osType` / `osArch` (H16.02): both — host name, platform, arch strings.
/// - `tempDir` / `homeDir` (H16.03): both — OS temp directory and home directory paths.
/// - `processRun` (H15.01): both — spawn argv, optional cwd + env subset, wait exit code.
/// - `processSpawn` / `processStdinWrite` / `processWait` / `processStdout` /
///   `processStderr` / `processKill` / `processClose` (H15.02): both — pipes + kill.
/// - `onSignal` / `raiseSignal` (H14.01): native-only — SIGINT/SIGTERM watch via job queue;
///   default without watch is OS terminate (SIG_DFL).
/// - `ignoreSignal` / `restoreSignal` (H14.02): native-only — SIG_IGN / SIG_DFL disposition.
/// - `stdoutWrite` (H02.01): both — write string or Uint8Array bytes to stdout.
/// - `stderrWrite` (H02.02): both — write string or Uint8Array bytes to stderr.
/// - `stdinReadLine` / `stdinReadBytes` (H02.03): both — blocking line/bytes from stdin.
/// - `pathJoin` / `pathNormalize` (H03.01): both — pure path string ops (no I/O).
/// - `pathDirname` / `pathBasename` / `pathExtname` / `pathIsAbsolute` (H03.02): both.
/// - `pathResolve` (H03.03): both — absolute path from segments; relative uses cwd.
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
/// - `udpBind` / `udpLocalPort` / `udpSendTo` / `udpRecvFrom` / `closeUdp` (H08.01): native-only UDP.
/// - `dnsLookup` (H09.01): native-only DNS hostname → IPv4 address string[]; failure → HostError EADDR.
/// - H09.03: `dnsLookup` hard-error on js until optional Node bridge.
/// - `httpParseRequest` / `httpRequestHeader` (H10.01): native-only HTTP/1.1 request parse.
/// - `httpWriteResponse` (H10.02): native-only HTTP/1.1 response format (status+headers+body).
/// - H10.03: compose TCP accept + parse + write + close (server one-shot; see `host_http_server`).
/// - `httpWriteRequest` / `httpParseResponse` / `httpResponseHeader` (H10.05): client helpers.
/// - H10.04: same surface, two request/response cycles on one connection (keep-alive).
/// - H10.07: HTTP listen helpers (`httpParseRequest` / `httpRequestHeader` / `httpWriteResponse`
///   and client parse/write) hard-error on js until optional Node bridge.
/// - `tlsClientWrap` / `tlsServerWrap` / `tlsRead` / `tlsWrite` / `closeTls` (H11.01/H11.02): native-only TLS.
/// - `wsHandshakeResponse` (H12.01): native-only WebSocket server opening handshake (RFC 6455).
/// - `wsEncodeText` / `wsEncodeBinary` / `wsEncodeClose` / `wsEncodePing` / `wsEncodePong` /
///   `wsDecodeFrame` (H12.02): native-only WebSocket frames (RFC 6455 §5).
/// - `wsClientHandshakeRequest` / `wsClientCheckAccept` / `wsEncodeTextClient` (H12.03):
///   native-only WebSocket client dial helpers + masked text frames.
/// - `http2ClientPreface` / `http2ServerPreface` / `http2SettingsAck` /
///   `http2EncodeRequest` / `http2EncodeResponse` / `http2ParseRequest` /
///   `http2ParseResponse` (H13.01): native-only HTTP/2 preface + single-stream helpers.
/// - `spawnWorker` (C01.01 / C02.04): both — spawn worker isolate from fn entry or module path;
///   optional second arg is a channel handle passed into the worker fn.
/// - `joinWorker` (C01.02): both — wait for worker exit; 0 success, negative error.
/// - `terminateWorker` (C01.03): both — stop worker; 0 success, negative error.
/// - `workerOsThread` (C01.04): native-only — 1 if handle is a live OS thread
///   distinct from the caller; 0 if same thread; -1 invalid/already dead.
/// - `makeChannel` / `channelSend` / `channelRecv` (C02.01–C02.03): both — FIFO channel of
///   scalars (number/bool), strings, and structured-cloned plain objects (shared refs
///   rejected); optional capacity `makeChannel(n)` (n>0 bounded; full send → -2);
///   send 0 success / negative error.
/// - `makeOnce` / `onceRun` (C03.01): native-only — thread-safe init cell; `onceRun`
///   returns 1 if this caller ran init, 0 if already done, negative if invalid.
/// - C03.02: Runtime-internal mutex only (workers/channels). Not a user Host API;
///   no `makeMutex` / shared-heap lock.
/// - `makeCancelToken` / `cancelTokenAbort` / `cancelTokenAborted` / `cancelTokenLink`
///   (C05.01): both — Abort-like cancel token; abort is sticky/idempotent; link
///   propagates parent abort to child (immediate if parent already aborted).
/// - `withTimeout` / `clearWithTimeout` (C05.02): both — race work vs timer;
///   `withTimeout(ms)` returns a token that auto-aborts after ms; `clearWithTimeout`
///   cancels the pending timer (work won; settle cleanly).
/// - `makeSharedMemory` / `sharedLoad` / `sharedStore` / `sharedAdd` /
///   `sharedCompareExchange` / `sharedWait` / `sharedNotify` (C06): native-only
///   integer buffer visible to worker isolates (no shared JS heap). JS hard-error.
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
        name: "pathResolve",
        availability: HostAvailability::BOTH,
        note: "H03.03 path resolve (cwd-relative)",
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
        name: "tlsClientWrap",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H11.01 TLS client wrap TCP conn (serverName, insecure)",
    },
    HostApiEntry {
        name: "tlsServerWrap",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H11.02 TLS server wrap TCP conn (certPath, keyPath PEM)",
    },
    HostApiEntry {
        name: "tlsRead",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H11.01 TLS read application data",
    },
    HostApiEntry {
        name: "tlsWrite",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H11.01 TLS write application data",
    },
    HostApiEntry {
        name: "closeTls",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H11.01 close TLS handle (and underlying TCP)",
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
    HostApiEntry {
        name: "udpBind",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H08.01 UDP bind (port 0 → ephemeral)",
    },
    HostApiEntry {
        name: "udpLocalPort",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H08.01 UDP local bound port",
    },
    HostApiEntry {
        name: "udpSendTo",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H08.01 UDP sendto host:port",
    },
    HostApiEntry {
        name: "udpRecvFrom",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H08.01 UDP recvfrom → bytes",
    },
    HostApiEntry {
        name: "closeUdp",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H08.01 close UDP handle",
    },
    HostApiEntry {
        name: "dnsLookup",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H09.01 DNS lookup hostname → IPv4 address strings",
    },
    HostApiEntry {
        name: "httpParseRequest",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H10.01 HTTP/1.1 request parse → method/path/version/body",
    },
    HostApiEntry {
        name: "httpRequestHeader",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H10.01 HTTP/1.1 request header lookup (case-insensitive)",
    },
    HostApiEntry {
        name: "httpWriteResponse",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H10.02 HTTP/1.1 response write → status-line + headers + body",
    },
    HostApiEntry {
        name: "httpServeStatic",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H17.03 HTTP/1.1 static file serve on TCP conn under docroot",
    },
    HostApiEntry {
        name: "httpWriteRequest",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H10.05 HTTP/1.1 request write → request-line + headers + body",
    },
    HostApiEntry {
        name: "httpParseResponse",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H10.05 HTTP/1.1 response parse → version/status/reason/body",
    },
    HostApiEntry {
        name: "httpResponseHeader",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H10.05 HTTP/1.1 response header lookup (case-insensitive)",
    },
    HostApiEntry {
        name: "wsHandshakeResponse",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.01 WebSocket server opening handshake response from Sec-WebSocket-Key",
    },
    HostApiEntry {
        name: "wsEncodeText",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.02 WebSocket text frame encode (FIN=1, unmasked)",
    },
    HostApiEntry {
        name: "wsEncodeBinary",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.02 WebSocket binary frame encode (FIN=1, unmasked)",
    },
    HostApiEntry {
        name: "wsEncodeClose",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.02 WebSocket close frame encode (code + reason)",
    },
    HostApiEntry {
        name: "wsEncodePing",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.02 WebSocket ping frame encode",
    },
    HostApiEntry {
        name: "wsEncodePong",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.02 WebSocket pong frame encode",
    },
    HostApiEntry {
        name: "wsDecodeFrame",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.02 WebSocket frame decode (unmask client frames)",
    },
    HostApiEntry {
        name: "wsClientHandshakeRequest",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.03 WebSocket client opening handshake request (path, host, key)",
    },
    HostApiEntry {
        name: "wsClientCheckAccept",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.03 WebSocket client validate 101 + Sec-WebSocket-Accept",
    },
    HostApiEntry {
        name: "wsEncodeTextClient",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.03 WebSocket client text frame encode (FIN=1, masked)",
    },
    HostApiEntry {
        name: "http2ClientPreface",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 client connection preface (magic + SETTINGS)",
    },
    HostApiEntry {
        name: "http2ServerPreface",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 server connection preface (SETTINGS)",
    },
    HostApiEntry {
        name: "http2SettingsAck",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 SETTINGS ACK frame",
    },
    HostApiEntry {
        name: "http2EncodeRequest",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 single-stream request (HEADERS+DATA stream 1)",
    },
    HostApiEntry {
        name: "http2EncodeResponse",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 single-stream response (HEADERS+DATA stream 1)",
    },
    HostApiEntry {
        name: "http2ParseRequest",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 parse single-stream request → method/path/body",
    },
    HostApiEntry {
        name: "http2ParseResponse",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 parse single-stream response → status/body",
    },
    HostApiEntry {
        name: "http2ClientOpen",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 client preface + request in one buffer",
    },
    HostApiEntry {
        name: "http2ServerReply",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 server preface + response in one buffer",
    },
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
    fn registry_lists_path_resolve_both() {
        let name = "pathResolve";
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
        assert!(unsupported_diagnostic("monotonicMs", CompileTarget::Js, Span::dummy()).is_none());
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
            "udpBind",
            "udpLocalPort",
            "udpSendTo",
            "udpRecvFrom",
            "closeUdp",
            "dnsLookup",
            "httpParseRequest",
            "httpRequestHeader",
            "httpWriteResponse",
            "httpWriteRequest",
            "httpParseResponse",
            "httpResponseHeader",
            "tlsClientWrap",
            "tlsServerWrap",
            "tlsRead",
            "tlsWrite",
            "closeTls",
            "wsHandshakeResponse",
            "wsEncodeText",
            "wsEncodeBinary",
            "wsEncodeClose",
            "wsEncodePing",
            "wsEncodePong",
            "wsDecodeFrame",
            "wsClientHandshakeRequest",
            "wsClientCheckAccept",
            "wsEncodeTextClient",
            "http2ClientPreface",
            "http2ServerPreface",
            "http2SettingsAck",
            "http2EncodeRequest",
            "http2EncodeResponse",
            "http2ParseRequest",
            "http2ParseResponse",
            "http2ClientOpen",
            "http2ServerReply",
            "onSignal",
            "raiseSignal",
            "ignoreSignal",
            "restoreSignal",
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
    fn check_for_target_js_rejects_dns_lookup() {
        let program = parse("dnsLookup(\"localhost\");").unwrap();
        let err = check_for_target(program, CompileTarget::Js).expect_err("js hard diagnostic");
        assert_eq!(err.code, Some(codes::HOST_API_UNSUPPORTED));
        assert!(
            err.message.contains("dnsLookup") && err.message.contains("unsupported on js"),
            "got {}",
            err.message
        );
    }

    #[test]
    fn check_for_target_js_rejects_http_listen_helpers() {
        for src in [
            "httpParseRequest(\"\");",
            "httpRequestHeader(null, \"Host\");",
            "httpWriteResponse(200, \"OK\", \"\", \"\");",
            "httpWriteRequest(\"GET\", \"/\", \"\", \"\");",
            "httpParseResponse(\"\");",
            "httpResponseHeader(null, \"Content-Type\");",
        ] {
            let program = parse(src).unwrap();
            let err = check_for_target(program, CompileTarget::Js)
                .expect_err(&format!("js must hard-error: {src}"));
            assert_eq!(err.code, Some(codes::HOST_API_UNSUPPORTED), "src={src}");
            assert!(
                err.message.contains("unsupported on js"),
                "src={src} got {}",
                err.message
            );
        }
    }

    #[test]
    fn check_for_target_native_allows_http_listen_helpers() {
        for src in [
            "httpParseRequest(\"\");",
            "httpWriteResponse(200, \"OK\", \"\", \"\");",
        ] {
            let program = parse(src).unwrap();
            check_for_target(program, CompileTarget::Native)
                .unwrap_or_else(|e| panic!("native allows {src}: {e}"));
        }
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
