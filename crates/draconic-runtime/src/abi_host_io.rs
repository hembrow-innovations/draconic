use super::{AbiFn, GC_INIT, JOB_DRAIN, PRINT_BOOL, PRINT_I64, PRINT_STR};

/* H14.01 / H14.02: signal watch / ignore / restore / raise / poll (native). */
pub const HOST_SIGNAL_WATCH: AbiFn = AbiFn {
    symbol: "draconic_rt_host_signal_watch",
    ret: "i32",
    params: "i32, ptr, ptr",
};
pub const HOST_SIGNAL_IGNORE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_signal_ignore",
    ret: "i32",
    params: "i32",
};
pub const HOST_SIGNAL_RESTORE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_signal_restore",
    ret: "i32",
    params: "i32",
};
pub const HOST_SIGNAL_RAISE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_signal_raise",
    ret: "i32",
    params: "i32",
};
pub const HOST_SIGNAL_POLL: AbiFn = AbiFn {
    symbol: "draconic_rt_host_signal_poll",
    ret: "i32",
    params: "",
};

/// Declares for H14 signal native emit.
pub const HOST_SIGNAL_DECLARES: &[AbiFn] = &[
    GC_INIT,
    PRINT_I64,
    PRINT_BOOL,
    PRINT_STR,
    JOB_DRAIN,
    HOST_SIGNAL_WATCH,
    HOST_SIGNAL_IGNORE,
    HOST_SIGNAL_RESTORE,
    HOST_SIGNAL_RAISE,
];

/* H05.01: wall clock ms since Unix epoch (double / JS Number). */
pub const HOST_NOW_MS: AbiFn = AbiFn {
    symbol: "draconic_rt_host_now_ms",
    ret: "double",
    params: "",
};

/* H05.02: monotonic clock ms for durations (double / JS Number). */
pub const HOST_MONOTONIC_MS: AbiFn = AbiFn {
    symbol: "draconic_rt_host_monotonic_ms",
    ret: "double",
    params: "",
};

/* H02.01: stdout write (raw bytes; no automatic newline). */
pub const HOST_STDOUT_WRITE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_stdout_write",
    ret: "i32",
    params: "ptr, i64",
};

/* H02.02: stderr write (raw bytes; no automatic newline). */
pub const HOST_STDERR_WRITE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_stderr_write",
    ret: "i32",
    params: "ptr, i64",
};

/* H02.03: stdin read line → malloc'd C string or null (EOF). */
pub const HOST_STDIN_READ_LINE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_stdin_read_line",
    ret: "ptr",
    params: "",
};

/* H02.03: stdin read up to max bytes → out_data/out_len (malloc'd). */
pub const HOST_STDIN_READ_BYTES: AbiFn = AbiFn {
    symbol: "draconic_rt_host_stdin_read_bytes",
    ret: "i32",
    params: "i64, ptr, ptr",
};

/* H03.01–H03.02: path helpers (malloc'd C string; free with path_free). */
pub const HOST_PATH_NORMALIZE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_path_normalize",
    ret: "ptr",
    params: "ptr",
};
pub const HOST_PATH_JOIN: AbiFn = AbiFn {
    symbol: "draconic_rt_host_path_join",
    ret: "ptr",
    params: "i64, ptr",
};
pub const HOST_PATH_DIRNAME: AbiFn = AbiFn {
    symbol: "draconic_rt_host_path_dirname",
    ret: "ptr",
    params: "ptr",
};
pub const HOST_PATH_BASENAME: AbiFn = AbiFn {
    symbol: "draconic_rt_host_path_basename",
    ret: "ptr",
    params: "ptr",
};
pub const HOST_PATH_EXTNAME: AbiFn = AbiFn {
    symbol: "draconic_rt_host_path_extname",
    ret: "ptr",
    params: "ptr",
};
pub const HOST_PATH_IS_ABSOLUTE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_path_is_absolute",
    ret: "i32",
    params: "ptr",
};
/* H03.03: path.resolve (malloc'd C string; free with path_free). */
pub const HOST_PATH_RESOLVE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_path_resolve",
    ret: "ptr",
    params: "i64, ptr",
};

/* H04.01: whole-file read (bytes + UTF-8 text). */
pub const HOST_FS_READ_FILE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_read_file",
    ret: "i32",
    params: "ptr, ptr, ptr",
};
pub const HOST_FS_READ_TEXT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_read_text",
    ret: "i32",
    params: "ptr, ptr",
};
/* H04.02: whole-file write / append (bytes + UTF-8 text). */
pub const HOST_FS_WRITE_FILE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_write_file",
    ret: "i32",
    params: "ptr, ptr, i64",
};
pub const HOST_FS_APPEND_FILE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_append_file",
    ret: "i32",
    params: "ptr, ptr, i64",
};
pub const HOST_FS_WRITE_TEXT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_write_text",
    ret: "i32",
    params: "ptr, ptr",
};
pub const HOST_FS_APPEND_TEXT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_append_text",
    ret: "i32",
    params: "ptr, ptr",
};
/* H04.03: exists / stat. */
pub const HOST_FS_EXISTS: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_exists",
    ret: "i32",
    params: "ptr",
};
pub const HOST_FS_STAT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_stat",
    ret: "i32",
    params: "ptr, ptr, ptr, ptr, ptr",
};
/* H04.04: mkdir / readdir / rmdir / removeFile. */
pub const HOST_FS_MKDIR: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_mkdir",
    ret: "i32",
    params: "ptr",
};
pub const HOST_FS_MKDIR_ALL: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_mkdir_all",
    ret: "i32",
    params: "ptr",
};
pub const HOST_FS_READDIR: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_readdir",
    ret: "i32",
    params: "ptr, ptr, ptr",
};
pub const HOST_FS_RMDIR: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_rmdir",
    ret: "i32",
    params: "ptr",
};
pub const HOST_FS_REMOVE_FILE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_remove_file",
    ret: "i32",
    params: "ptr",
};
/* H04.05: renameFile / copyFile. */
pub const HOST_FS_RENAME_FILE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_rename_file",
    ret: "i32",
    params: "ptr, ptr",
};
pub const HOST_FS_COPY_FILE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_copy_file",
    ret: "i32",
    params: "ptr, ptr",
};
/* H04.06: open handle open/read/write/seek (close via HOST_HANDLE_CLOSE). */
pub const HOST_FS_OPEN: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_open",
    ret: "i32",
    params: "ptr, ptr, ptr",
};
pub const HOST_FS_HANDLE_READ: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_handle_read",
    ret: "i32",
    params: "i64, i64, ptr, ptr",
};
pub const HOST_FS_HANDLE_WRITE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_handle_write",
    ret: "i32",
    params: "i64, ptr, i64",
};
pub const HOST_FS_HANDLE_SEEK: AbiFn = AbiFn {
    symbol: "draconic_rt_host_fs_handle_seek",
    ret: "i32",
    params: "i64, i64, i32, ptr",
};
