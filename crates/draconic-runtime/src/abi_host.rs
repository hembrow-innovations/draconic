use super::{
    AbiFn, GC_INIT, JOB_DRAIN, PRINT_BOOL, PRINT_I64, PRINT_STR, PROMISE_THEN,
};

// --- Host I/O substrate (H00.02–H00.03): errors, handles, path, bytes ---
//
// Stable integer codes shared with `draconic_rt.h` / `draconic_rt_host.c`.
// No real fs/tcp/process yet — later H rows open handles and map OS errno.
// H00.03: DraconicHostBytes views model ArrayBuffer / Uint8Array OS buffers.

/// Success.
pub const HOST_OK: i32 = 0;
/// Invalid argument (bad UTF-8 path, null out-param, etc.).
pub const HOST_E_INVAL: i32 = 1;
/// No such file or directory.
pub const HOST_E_NOENT: i32 = 2;
/// Function not implemented / unsupported on this build.
pub const HOST_E_NOSYS: i32 = 3;
/// Bad file handle / closed handle.
pub const HOST_E_BADF: i32 = 4;
/// Already exists.
pub const HOST_E_EXIST: i32 = 5;
/// Permission denied.
pub const HOST_E_PERM: i32 = 6;
/// I/O error.
pub const HOST_E_IO: i32 = 7;
/// Out of memory.
pub const HOST_E_NOMEM: i32 = 8;
/// Resource temporarily unavailable / would block.
pub const HOST_E_AGAIN: i32 = 9;
/// Connection error (refused, reset, aborted).
pub const HOST_E_CONN: i32 = 10;
/// Address error (in use, not available).
pub const HOST_E_ADDR: i32 = 11;

/// Language `HostError.code` / native stderr token for an ABI failure (H00).
///
/// `HOST_OK` and unknown codes return `None` (not an error token).
/// Native Programs print this token on stderr and exit 1; js throws a catchable
/// `Error` with `.name === "HostError"` and `.code` equal to this string.
pub fn host_error_name(code: i32) -> Option<&'static str> {
    match code {
        HOST_E_INVAL => Some("EINVAL"),
        HOST_E_NOENT => Some("ENOENT"),
        HOST_E_NOSYS => Some("ENOSYS"),
        HOST_E_BADF => Some("EBADF"),
        HOST_E_EXIST => Some("EEXIST"),
        HOST_E_PERM => Some("EPERM"),
        HOST_E_IO => Some("EIO"),
        HOST_E_NOMEM => Some("ENOMEM"),
        HOST_E_AGAIN => Some("EAGAIN"),
        HOST_E_CONN => Some("ECONN"),
        HOST_E_ADDR => Some("EADDR"),
        _ => None,
    }
}

/// Sentinel for an unset / closed host handle (`DraconicHostHandle`).
pub const HOST_HANDLE_INVALID: i64 = -1;

pub const HOST_HANDLE_IS_VALID: AbiFn = AbiFn {
    symbol: "draconic_rt_host_handle_is_valid",
    ret: "i32",
    params: "i64",
};
pub const HOST_HANDLE_CLOSE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_handle_close",
    ret: "i32",
    params: "i64",
};
pub const HOST_PATH_FROM_UTF8: AbiFn = AbiFn {
    symbol: "draconic_rt_host_path_from_utf8",
    ret: "i32",
    params: "ptr, i64, ptr",
};
pub const HOST_PATH_FREE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_path_free",
    ret: "void",
    params: "ptr",
};

/* H00.03: I/O bytes boundary (ArrayBuffer / Uint8Array as OS buffers). */
pub const HOST_BYTES_FROM_RAW: AbiFn = AbiFn {
    symbol: "draconic_rt_host_bytes_from_raw",
    ret: "i32",
    params: "ptr, i64, ptr",
};
pub const HOST_BYTES_VIEW: AbiFn = AbiFn {
    symbol: "draconic_rt_host_bytes_view",
    ret: "i32",
    params: "ptr, i64, i64, ptr",
};
pub const HOST_BYTES_ALLOC: AbiFn = AbiFn {
    symbol: "draconic_rt_host_bytes_alloc",
    ret: "i32",
    params: "i64, ptr",
};
pub const HOST_BYTES_STORAGE_FREE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_bytes_storage_free",
    ret: "void",
    params: "ptr",
};
pub const HOST_BYTES_COPY_IN: AbiFn = AbiFn {
    symbol: "draconic_rt_host_bytes_copy_in",
    ret: "i32",
    params: "ptr, ptr, i64, ptr",
};
pub const HOST_BYTES_COPY_OUT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_bytes_copy_out",
    ret: "i32",
    params: "ptr, ptr, i64, ptr",
};

/* H01.01: process user args (OS argv without argv[0]). */
pub const HOST_PROCESS_SET_ARGV: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_set_argv",
    ret: "void",
    params: "i32, ptr",
};
pub const HOST_PROCESS_USER_ARGC: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_user_argc",
    ret: "i32",
    params: "",
};
pub const HOST_PROCESS_USER_ARG: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_user_arg",
    ret: "ptr",
    params: "i32",
};

/* H01.02: process env get/set/delete (string values; missing get → null). */
pub const HOST_ENV_GET: AbiFn = AbiFn {
    symbol: "draconic_rt_host_env_get",
    ret: "ptr",
    params: "ptr",
};
pub const HOST_ENV_SET: AbiFn = AbiFn {
    symbol: "draconic_rt_host_env_set",
    ret: "i32",
    params: "ptr, ptr",
};
pub const HOST_ENV_DELETE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_env_delete",
    ret: "i32",
    params: "ptr",
};

/* H01.03: process exit / exitCode. */
pub const HOST_PROCESS_EXIT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_exit",
    ret: "void",
    params: "i32",
};
pub const HOST_PROCESS_SET_EXIT_CODE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_set_exit_code",
    ret: "void",
    params: "i32",
};
pub const HOST_PROCESS_GET_EXIT_CODE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_get_exit_code",
    ret: "i32",
    params: "",
};

/* H01.04: process pid / ppid (read-only). */
pub const HOST_PROCESS_PID: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_pid",
    ret: "i32",
    params: "",
};
pub const HOST_PROCESS_PPID: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_ppid",
    ret: "i32",
    params: "",
};

/* H16.01: cwd get + chdir. */
pub const HOST_CWD: AbiFn = AbiFn {
    symbol: "draconic_rt_host_cwd",
    ret: "ptr",
    params: "",
};
pub const HOST_CHDIR: AbiFn = AbiFn {
    symbol: "draconic_rt_host_chdir",
    ret: "i32",
    params: "ptr",
};

/* H16.02: hostname / OS type / arch strings. */
pub const HOST_HOSTNAME: AbiFn = AbiFn {
    symbol: "draconic_rt_host_hostname",
    ret: "ptr",
    params: "",
};
pub const HOST_OS_TYPE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_os_type",
    ret: "ptr",
    params: "",
};
pub const HOST_OS_ARCH: AbiFn = AbiFn {
    symbol: "draconic_rt_host_os_arch",
    ret: "ptr",
    params: "",
};

/* H16.03: temp / home directory paths. */
pub const HOST_TEMP_DIR: AbiFn = AbiFn {
    symbol: "draconic_rt_host_temp_dir",
    ret: "ptr",
    params: "",
};
pub const HOST_HOME_DIR: AbiFn = AbiFn {
    symbol: "draconic_rt_host_home_dir",
    ret: "ptr",
    params: "",
};

/* H15.01: processRun — spawn argv, optional cwd/env subset, wait exit code. */
pub const HOST_PROCESS_RUN: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_run",
    ret: "i32",
    params: "i32, ptr, ptr, i32, ptr, ptr",
};

/* H15.02: process spawn + pipes (stdin write, stdout/stderr capture, kill). */
pub const HOST_PROCESS_SPAWN: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_spawn",
    ret: "i32",
    params: "i32, ptr, ptr, i32, ptr, ptr",
};
pub const HOST_PROCESS_STDIN_WRITE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_stdin_write",
    ret: "i32",
    params: "i32, ptr, i64",
};
pub const HOST_PROCESS_WAIT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_wait",
    ret: "i32",
    params: "i32",
};
/* H15.03: async process wait → Promise. */
pub const HOST_PROCESS_WAIT_ASYNC: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_wait_async",
    ret: "ptr",
    params: "i32",
};
pub const HOST_PROCESS_STDOUT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_stdout",
    ret: "i32",
    params: "i32, ptr",
};
pub const HOST_PROCESS_STDERR: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_stderr",
    ret: "i32",
    params: "i32, ptr",
};
pub const HOST_PROCESS_KILL: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_kill",
    ret: "i32",
    params: "i32",
};
pub const HOST_PROCESS_CLOSE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_process_close",
    ret: "i32",
    params: "i32",
};
/* C01.01: spawnWorker — isolate from fn entry (kind 0) or module path (kind 1). */
pub const HOST_WORKER_SPAWN: AbiFn = AbiFn {
    symbol: "draconic_rt_host_worker_spawn",
    ret: "i32",
    params: "i32, ptr",
};
/* C01.02: joinWorker — wait for exit; 0 success, negative error. */
pub const HOST_WORKER_JOIN: AbiFn = AbiFn {
    symbol: "draconic_rt_host_worker_join",
    ret: "i32",
    params: "i32",
};
/* C01.03: terminateWorker — force-stop isolate; 0 success, negative error. */
pub const HOST_WORKER_TERMINATE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_worker_terminate",
    ret: "i32",
    params: "i32",
};
/* C01.04: 1 if handle is a live OS thread distinct from the caller; 0 if same
thread / no OS thread; -1 invalid or already joined/terminated. */
pub const HOST_WORKER_OS_THREAD: AbiFn = AbiFn {
    symbol: "draconic_rt_host_worker_os_thread",
    ret: "i32",
    params: "i32",
};
/* C02.01/C02.03: makeChannel — FIFO handle >= 1, or -1 on failure.
cap > 0 bounds the buffer; cap <= 0 is unbounded. Send on a full
bounded channel returns -2 (backpressure). */
pub const HOST_CHANNEL_MAKE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_channel_make",
    ret: "i32",
    params: "i32",
};
/* C02.01: channelSend number; 0 success, -1 invalid handle, -2 full. */
pub const HOST_CHANNEL_SEND_F64: AbiFn = AbiFn {
    symbol: "draconic_rt_host_channel_send_f64",
    ret: "i32",
    params: "i32, double",
};
/* C02.01: channelSend string; 0 success, -1 invalid handle. */
pub const HOST_CHANNEL_SEND_STR: AbiFn = AbiFn {
    symbol: "draconic_rt_host_channel_send_str",
    ret: "i32",
    params: "i32, ptr",
};
/* C02.01: channelSend bool (i32 0/1); 0 success, -1 invalid handle. */
pub const HOST_CHANNEL_SEND_BOOL: AbiFn = AbiFn {
    symbol: "draconic_rt_host_channel_send_bool",
    ret: "i32",
    params: "i32, i32",
};
/* C02.01: channelRecv number into out ptr; 0 success, -1 fail. */
pub const HOST_CHANNEL_RECV_F64: AbiFn = AbiFn {
    symbol: "draconic_rt_host_channel_recv_f64",
    ret: "i32",
    params: "i32, ptr",
};
/* C02.01: channelRecv string into out ptr; 0 success, -1 fail. */
pub const HOST_CHANNEL_RECV_STR: AbiFn = AbiFn {
    symbol: "draconic_rt_host_channel_recv_str",
    ret: "i32",
    params: "i32, ptr",
};
/* C02.01: channelRecv bool into out ptr; 0 success, -1 fail. */
pub const HOST_CHANNEL_RECV_BOOL: AbiFn = AbiFn {
    symbol: "draconic_rt_host_channel_recv_bool",
    ret: "i32",
    params: "i32, ptr",
};
/* C03.01: makeOnce — thread-safe init cell; handle >= 1, or -1 on failure. */
pub const HOST_ONCE_MAKE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_once_make",
    ret: "i32",
    params: "",
};
/* C03.01: onceRun — call fn at most once per handle. 1 ran, 0 already done,
-1 invalid. fn may be null (empty init). Concurrent callers wait. */
pub const HOST_ONCE_RUN: AbiFn = AbiFn {
    symbol: "draconic_rt_host_once_run",
    ret: "i32",
    params: "i32, ptr",
};
/* C06: makeSharedMemory(len) — integer slots; handle >= 1, or -1. */
pub const HOST_SHARED_MAKE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_shared_make",
    ret: "i32",
    params: "i32",
};
/* C06: sharedLoad(handle, index) — atomic i32; invalid → 0. */
pub const HOST_SHARED_LOAD: AbiFn = AbiFn {
    symbol: "draconic_rt_host_shared_load",
    ret: "i32",
    params: "i32, i32",
};
/* C06: sharedStore(handle, index, value) — 0 ok, -1 invalid. */
pub const HOST_SHARED_STORE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_shared_store",
    ret: "i32",
    params: "i32, i32, i32",
};
/* C06: sharedAdd(handle, index, delta) — old i32; invalid → 0. */
pub const HOST_SHARED_ADD: AbiFn = AbiFn {
    symbol: "draconic_rt_host_shared_add",
    ret: "i32",
    params: "i32, i32, i32",
};
/* C06: sharedCompareExchange — old i32; invalid → 0. */
pub const HOST_SHARED_CMPXCHG: AbiFn = AbiFn {
    symbol: "draconic_rt_host_shared_cmpxchg",
    ret: "i32",
    params: "i32, i32, i32, i32",
};
/* C06: sharedWait — 0 woken / 1 not-equal / 2 timed-out / -1 invalid. */
pub const HOST_SHARED_WAIT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_shared_wait",
    ret: "i32",
    params: "i32, i32, i32, double",
};
/* C06: sharedNotify(handle, index) — waiter count, or -1 invalid. */
pub const HOST_SHARED_NOTIFY: AbiFn = AbiFn {
    symbol: "draconic_rt_host_shared_notify",
    ret: "i32",
    params: "i32, i32",
};
/* C03.02: Runtime-internal mutex (not a user Host API; no shared JS heap lock).
make → handle >= 1 or -1; lock/unlock → 0 success, -1 invalid. */
pub const HOST_INTERNAL_MUTEX_MAKE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_internal_mutex_make",
    ret: "i32",
    params: "",
};
pub const HOST_INTERNAL_MUTEX_LOCK: AbiFn = AbiFn {
    symbol: "draconic_rt_host_internal_mutex_lock",
    ret: "i32",
    params: "i32",
};
pub const HOST_INTERNAL_MUTEX_UNLOCK: AbiFn = AbiFn {
    symbol: "draconic_rt_host_internal_mutex_unlock",
    ret: "i32",
    params: "i32",
};
/* C05.01: makeCancelToken — Abort-like handle >= 1, or -1 on failure. */
pub const HOST_CANCEL_MAKE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_cancel_make",
    ret: "i32",
    params: "",
};
/* C05.01: abort token. 0 success (sticky/idempotent), -1 invalid. */
pub const HOST_CANCEL_ABORT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_cancel_abort",
    ret: "i32",
    params: "i32",
};
/* C05.01: aborted? 1 yes / 0 no / -1 invalid. */
pub const HOST_CANCEL_ABORTED: AbiFn = AbiFn {
    symbol: "draconic_rt_host_cancel_aborted",
    ret: "i32",
    params: "i32",
};
/* C05.01: link child to parent; parent abort propagates. 0 ok, -1 invalid. */
pub const HOST_CANCEL_LINK: AbiFn = AbiFn {
    symbol: "draconic_rt_host_cancel_link",
    ret: "i32",
    params: "i32, i32",
};
/* C05.02: withTimeout(ms) — token that auto-aborts after ms. Handle >= 1 or -1. */
pub const HOST_CANCEL_TIMEOUT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_cancel_timeout",
    ret: "i32",
    params: "double",
};
/* C05.02: clearWithTimeout(token) — cancel pending timer. 0 ok, -1 invalid. */
pub const HOST_CANCEL_CLEAR_TIMEOUT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_cancel_clear_timeout",
    ret: "i32",
    params: "i32",
};
/* C02.02: channelSend plain object (structured clone); 0 success, -1 reject. */
pub const HOST_CHANNEL_SEND_OBJ: AbiFn = AbiFn {
    symbol: "draconic_rt_host_channel_send_obj",
    ret: "i32",
    params: "i32, ptr",
};
/* C02.02: channelRecv object into out ptr; 0 success, -1 fail. */
pub const HOST_CHANNEL_RECV_OBJ: AbiFn = AbiFn {
    symbol: "draconic_rt_host_channel_recv_obj",
    ret: "i32",
    params: "i32, ptr",
};
/// Declares for H15.03 async process wait + Promise then + job drain.
pub const HOST_PROCESS_ASYNC_DECLARES: &[AbiFn] = &[
    GC_INIT,
    JOB_DRAIN,
    PROMISE_THEN,
    PRINT_I64,
    PRINT_STR,
    PRINT_BOOL,
    HOST_PROCESS_SPAWN,
    HOST_PROCESS_WAIT_ASYNC,
    HOST_PROCESS_CLOSE,
];

