//! Runtime C ABI surface for consumers (LLVM backend declare/call shapes).
//!
//! Symbol names match `draconic_rt.h` / `draconic_rt.c`. LLVM-text parameter and
//! return types are the shapes the backend currently emits (opaque `ptr`,
//! `i64` for size_t-ish integers, etc.).

/// One Runtime C ABI function with the LLVM-text shape the backend emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AbiFn {
    /// C / object-file symbol name (no leading `@`).
    pub symbol: &'static str,
    /// LLVM return type (`void`, `ptr`, `i64`, …).
    pub ret: &'static str,
    /// LLVM parameter type list (`""`, `"i64"`, `"ptr, ptr"`, …).
    pub params: &'static str,
}

impl AbiFn {
    /// `declare <ret> @<symbol>(<params>)`
    pub fn declare(self) -> String {
        format!("declare {} @{}({})", self.ret, self.symbol, self.params)
    }

    /// `@<symbol>` for operand positions that need the at-name.
    pub fn llvm_ref(self) -> String {
        format!("@{}", self.symbol)
    }

    /// `call <ret> @<symbol>(<args>)` — `args` is the typed LLVM arg list (may be empty).
    pub fn call(self, args: &str) -> String {
        if args.is_empty() {
            format!("call {} @{}()", self.ret, self.symbol)
        } else {
            format!("call {} @{}({})", self.ret, self.symbol, args)
        }
    }

    /// `<dest> = call <ret> @<symbol>(<args>)`
    pub fn call_to(self, dest: &str, args: &str) -> String {
        if args.is_empty() {
            format!("{dest} = call {} @{}()", self.ret, self.symbol)
        } else {
            format!("{dest} = call {} @{}({})", self.ret, self.symbol, args)
        }
    }
}

/// Join declare lines for a set of ABI functions (one per line, no trailing newline).
pub fn llvm_declares(fns: &[AbiFn]) -> String {
    fns.iter()
        .map(|f| f.declare())
        .collect::<Vec<_>>()
        .join("\n")
}

// --- Minimal std I/O ---

pub const HELLO: AbiFn = AbiFn {
    symbol: "draconic_rt_hello",
    ret: "void",
    params: "",
};
pub const PRINT_I64: AbiFn = AbiFn {
    symbol: "draconic_rt_print_i64",
    ret: "void",
    params: "i64",
};
pub const PRINT_U64: AbiFn = AbiFn {
    symbol: "draconic_rt_print_u64",
    ret: "void",
    params: "i64",
};
pub const PRINT_F64: AbiFn = AbiFn {
    symbol: "draconic_rt_print_f64",
    ret: "void",
    params: "double",
};
pub const PRINT_BOOL: AbiFn = AbiFn {
    symbol: "draconic_rt_print_bool",
    ret: "void",
    params: "i8",
};
pub const PRINT_STR: AbiFn = AbiFn {
    symbol: "draconic_rt_print_str",
    ret: "void",
    params: "ptr",
};

// --- C-string helpers (N08.02.08 for-in/of + string concat observations) ---

pub const CSTR_LEN: AbiFn = AbiFn {
    symbol: "draconic_rt_cstr_len",
    ret: "i64",
    params: "ptr",
};
pub const CSTR_CONCAT: AbiFn = AbiFn {
    symbol: "draconic_rt_cstr_concat",
    ret: "ptr",
    params: "ptr, ptr",
};
pub const CSTR_FROM_U64: AbiFn = AbiFn {
    symbol: "draconic_rt_cstr_from_u64",
    ret: "ptr",
    params: "i64",
};
pub const CSTR_FROM_CODE_UNIT: AbiFn = AbiFn {
    symbol: "draconic_rt_cstr_from_code_unit",
    ret: "ptr",
    params: "ptr, i64",
};
pub const PRINT_BYTES: AbiFn = AbiFn {
    symbol: "draconic_rt_print_bytes",
    ret: "void",
    params: "ptr, i64",
};
pub const CSTR_CONCAT_N: AbiFn = AbiFn {
    symbol: "draconic_rt_cstr_concat_n",
    ret: "ptr",
    params: "ptr, i64, ptr, i64, ptr",
};
pub const CSTR_FROM_CODE_UNIT_N: AbiFn = AbiFn {
    symbol: "draconic_rt_cstr_from_code_unit_n",
    ret: "ptr",
    params: "ptr, i64, i64, ptr",
};
pub const CSTR_EQ_N: AbiFn = AbiFn {
    symbol: "draconic_rt_cstr_eq_n",
    ret: "i32",
    params: "ptr, i64, ptr, i64",
};
pub const UTF16_LEN: AbiFn = AbiFn {
    symbol: "draconic_rt_utf16_len",
    ret: "i64",
    params: "ptr, i64",
};

// --- GC ---

pub const GC_INIT: AbiFn = AbiFn {
    symbol: "draconic_rt_gc_init",
    ret: "void",
    params: "",
};
pub const GC_SHUTDOWN: AbiFn = AbiFn {
    symbol: "draconic_rt_gc_shutdown",
    ret: "void",
    params: "",
};
pub const ALLOC_STRING: AbiFn = AbiFn {
    symbol: "draconic_rt_alloc_string",
    ret: "ptr",
    params: "ptr, i64",
};
pub const ALLOC_OBJECT: AbiFn = AbiFn {
    symbol: "draconic_rt_alloc_object",
    ret: "ptr",
    params: "",
};
pub const GC_ROOT_PUSH: AbiFn = AbiFn {
    symbol: "draconic_rt_gc_root_push",
    ret: "void",
    params: "ptr",
};
pub const GC_ROOT_POP: AbiFn = AbiFn {
    symbol: "draconic_rt_gc_root_pop",
    ret: "void",
    params: "",
};
pub const GC_COLLECT: AbiFn = AbiFn {
    symbol: "draconic_rt_gc_collect",
    ret: "void",
    params: "",
};
pub const GC_LIVE_COUNT: AbiFn = AbiFn {
    symbol: "draconic_rt_gc_live_count",
    ret: "i64",
    params: "",
};
pub const GC_SET_ALLOC_THRESHOLD: AbiFn = AbiFn {
    symbol: "draconic_rt_gc_set_alloc_threshold",
    ret: "void",
    params: "i64",
};
pub const GC_ALLOC_THRESHOLD: AbiFn = AbiFn {
    symbol: "draconic_rt_gc_alloc_threshold",
    ret: "i64",
    params: "",
};
pub const STRING_DATA: AbiFn = AbiFn {
    symbol: "draconic_rt_string_data",
    ret: "ptr",
    params: "ptr",
};
pub const STRING_LEN: AbiFn = AbiFn {
    symbol: "draconic_rt_string_len",
    ret: "i64",
    params: "ptr",
};
pub const IS_STRING: AbiFn = AbiFn {
    symbol: "draconic_rt_is_string",
    ret: "i32",
    params: "ptr",
};
pub const IS_OBJECT: AbiFn = AbiFn {
    symbol: "draconic_rt_is_object",
    ret: "i32",
    params: "ptr",
};

// --- Job queue ---

pub const JOB_ENQUEUE: AbiFn = AbiFn {
    symbol: "draconic_rt_job_enqueue",
    ret: "void",
    params: "ptr, ptr",
};
pub const JOB_DRAIN: AbiFn = AbiFn {
    symbol: "draconic_rt_job_drain",
    ret: "void",
    params: "",
};
pub const JOB_PENDING: AbiFn = AbiFn {
    symbol: "draconic_rt_job_pending",
    ret: "i64",
    params: "",
};

// --- Host timers (H05.03–H05.04) ---

pub const TIMER_SET: AbiFn = AbiFn {
    symbol: "draconic_rt_timer_set",
    ret: "i64",
    params: "ptr, ptr, double",
};
pub const TIMER_SET_INTERVAL: AbiFn = AbiFn {
    symbol: "draconic_rt_timer_set_interval",
    ret: "i64",
    params: "ptr, ptr, double",
};
pub const TIMER_CLEAR: AbiFn = AbiFn {
    symbol: "draconic_rt_timer_clear",
    ret: "void",
    params: "i64",
};

// --- Promise / array / object ---

pub const PROMISE_NEW: AbiFn = AbiFn {
    symbol: "draconic_rt_promise_new",
    ret: "ptr",
    params: "",
};
pub const IS_PROMISE: AbiFn = AbiFn {
    symbol: "draconic_rt_is_promise",
    ret: "i32",
    params: "ptr",
};
pub const PROMISE_STATE: AbiFn = AbiFn {
    symbol: "draconic_rt_promise_state",
    ret: "i32",
    params: "ptr",
};
pub const PROMISE_RESULT: AbiFn = AbiFn {
    symbol: "draconic_rt_promise_result",
    ret: "ptr",
    params: "ptr",
};
pub const PROMISE_RESOLVE: AbiFn = AbiFn {
    symbol: "draconic_rt_promise_resolve",
    ret: "void",
    params: "ptr, ptr",
};
pub const PROMISE_REJECT: AbiFn = AbiFn {
    symbol: "draconic_rt_promise_reject",
    ret: "void",
    params: "ptr, ptr",
};
pub const PROMISE_THEN: AbiFn = AbiFn {
    symbol: "draconic_rt_promise_then",
    ret: "ptr",
    params: "ptr, ptr, ptr, ptr, ptr",
};
pub const PROMISE_CONSTRUCT: AbiFn = AbiFn {
    symbol: "draconic_rt_promise_construct",
    ret: "ptr",
    params: "ptr, ptr",
};
pub const PROMISE_FINALLY: AbiFn = AbiFn {
    symbol: "draconic_rt_promise_finally",
    ret: "ptr",
    params: "ptr, ptr, ptr",
};
pub const ARRAY_NEW: AbiFn = AbiFn {
    symbol: "draconic_rt_array_new",
    ret: "ptr",
    params: "i64",
};
pub const IS_ARRAY: AbiFn = AbiFn {
    symbol: "draconic_rt_is_array",
    ret: "i32",
    params: "ptr",
};
pub const ARRAY_LEN: AbiFn = AbiFn {
    symbol: "draconic_rt_array_len",
    ret: "i64",
    params: "ptr",
};
pub const ARRAY_GET: AbiFn = AbiFn {
    symbol: "draconic_rt_array_get",
    ret: "ptr",
    params: "ptr, i64",
};
pub const ARRAY_SET: AbiFn = AbiFn {
    symbol: "draconic_rt_array_set",
    ret: "void",
    params: "ptr, i64, ptr",
};
pub const ARRAY_SPREAD_ARRAY: AbiFn = AbiFn {
    symbol: "draconic_rt_array_spread_array",
    ret: "void",
    params: "ptr, ptr",
};
pub const ARRAY_SPREAD_CSTR: AbiFn = AbiFn {
    symbol: "draconic_rt_array_spread_cstr",
    ret: "void",
    params: "ptr, ptr",
};
pub const PROMISE_ALL: AbiFn = AbiFn {
    symbol: "draconic_rt_promise_all",
    ret: "ptr",
    params: "ptr",
};
pub const PROMISE_RACE: AbiFn = AbiFn {
    symbol: "draconic_rt_promise_race",
    ret: "ptr",
    params: "ptr",
};
pub const OBJECT_GET: AbiFn = AbiFn {
    symbol: "draconic_rt_object_get",
    ret: "ptr",
    params: "ptr, ptr",
};
pub const OBJECT_SET: AbiFn = AbiFn {
    symbol: "draconic_rt_object_set",
    ret: "void",
    params: "ptr, ptr, ptr",
};
/// N08.09.02: get/set own props keyed by Symbol id (i64).
pub const OBJECT_REST: AbiFn = AbiFn {
    symbol: "draconic_rt_object_rest",
    ret: "ptr",
    params: "ptr, ptr",
};
pub const OBJECT_GET_BY_SYMBOL: AbiFn = AbiFn {
    symbol: "draconic_rt_object_get_symbol",
    ret: "ptr",
    params: "ptr, i64",
};
pub const OBJECT_SET_BY_SYMBOL: AbiFn = AbiFn {
    symbol: "draconic_rt_object_set_symbol",
    ret: "void",
    params: "ptr, i64, ptr",
};
pub const OBJECT_SET_PROTO: AbiFn = AbiFn {
    symbol: "draconic_rt_object_set_proto",
    ret: "void",
    params: "ptr, ptr",
};
pub const OBJECT_GET_PROTO: AbiFn = AbiFn {
    symbol: "draconic_rt_object_get_proto",
    ret: "ptr",
    params: "ptr",
};
/// N08.16.19: shallow copy own string props (object rest).
pub const OBJECT_COPY_OWN: AbiFn = AbiFn {
    symbol: "draconic_rt_object_copy_own",
    ret: "void",
    params: "ptr, ptr",
};
/// N08.16.19: delete own string prop (object rest exclusions).
pub const OBJECT_DELETE: AbiFn = AbiFn {
    symbol: "draconic_rt_object_delete",
    ret: "void",
    params: "ptr, ptr",
};
/// N08.16.28: CopyDataProperties for object spread (`{...src}`).
pub const OBJECT_SPREAD: AbiFn = AbiFn {
    symbol: "draconic_rt_object_spread",
    ret: "void",
    params: "ptr, ptr",
};
pub const PROMISE_ALL_SETTLED: AbiFn = AbiFn {
    symbol: "draconic_rt_promise_all_settled",
    ret: "ptr",
    params: "ptr",
};
pub const PROMISE_ANY: AbiFn = AbiFn {
    symbol: "draconic_rt_promise_any",
    ret: "ptr",
    params: "ptr",
};
pub const PROMISE_AWAIT: AbiFn = AbiFn {
    symbol: "draconic_rt_promise_await",
    ret: "ptr",
    params: "ptr",
};

// --- JS Symbol (N08.09.01) ---

pub const SYMBOL_NEW: AbiFn = AbiFn {
    symbol: "draconic_rt_symbol_new",
    ret: "i64",
    params: "",
};
pub const SYMBOL_FOR: AbiFn = AbiFn {
    symbol: "draconic_rt_symbol_for",
    ret: "i64",
    params: "ptr, i64",
};
pub const SYMBOL_KEY_FOR: AbiFn = AbiFn {
    symbol: "draconic_rt_symbol_key_for",
    ret: "ptr",
    params: "i64, ptr",
};

/// Declares used by the ES values / Symbol observation emitter (N08.09.01–N08.09.02).
pub const ES_VALUES_DECLARES: &[AbiFn] = &[
    PRINT_BOOL,
    PRINT_BYTES,
    PRINT_F64,
    GC_INIT,
    ALLOC_OBJECT,
    OBJECT_GET,
    OBJECT_SET,
    OBJECT_GET_BY_SYMBOL,
    OBJECT_SET_BY_SYMBOL,
    SYMBOL_NEW,
    SYMBOL_FOR,
    SYMBOL_KEY_FOR,
];

// --- Symbol-name aliases (single source: AbiFn.symbol) ---

pub const HELLO_SYMBOL: &str = HELLO.symbol;
pub const PRINT_I64_SYMBOL: &str = PRINT_I64.symbol;
pub const PRINT_U64_SYMBOL: &str = PRINT_U64.symbol;
pub const PRINT_F64_SYMBOL: &str = PRINT_F64.symbol;
pub const PRINT_BOOL_SYMBOL: &str = PRINT_BOOL.symbol;
pub const PRINT_STR_SYMBOL: &str = PRINT_STR.symbol;
pub const GC_INIT_SYMBOL: &str = GC_INIT.symbol;
pub const GC_SHUTDOWN_SYMBOL: &str = GC_SHUTDOWN.symbol;
pub const ALLOC_STRING_SYMBOL: &str = ALLOC_STRING.symbol;
pub const ALLOC_OBJECT_SYMBOL: &str = ALLOC_OBJECT.symbol;
pub const GC_ROOT_PUSH_SYMBOL: &str = GC_ROOT_PUSH.symbol;
pub const GC_ROOT_POP_SYMBOL: &str = GC_ROOT_POP.symbol;
pub const GC_COLLECT_SYMBOL: &str = GC_COLLECT.symbol;
pub const GC_LIVE_COUNT_SYMBOL: &str = GC_LIVE_COUNT.symbol;
pub const GC_SET_ALLOC_THRESHOLD_SYMBOL: &str = GC_SET_ALLOC_THRESHOLD.symbol;
pub const GC_ALLOC_THRESHOLD_SYMBOL: &str = GC_ALLOC_THRESHOLD.symbol;
pub const STRING_DATA_SYMBOL: &str = STRING_DATA.symbol;
pub const STRING_LEN_SYMBOL: &str = STRING_LEN.symbol;
pub const IS_STRING_SYMBOL: &str = IS_STRING.symbol;
pub const IS_OBJECT_SYMBOL: &str = IS_OBJECT.symbol;
pub const JOB_ENQUEUE_SYMBOL: &str = JOB_ENQUEUE.symbol;
pub const JOB_DRAIN_SYMBOL: &str = JOB_DRAIN.symbol;
pub const JOB_PENDING_SYMBOL: &str = JOB_PENDING.symbol;
pub const TIMER_SET_SYMBOL: &str = TIMER_SET.symbol;
pub const TIMER_SET_INTERVAL_SYMBOL: &str = TIMER_SET_INTERVAL.symbol;
pub const TIMER_CLEAR_SYMBOL: &str = TIMER_CLEAR.symbol;
pub const PROMISE_NEW_SYMBOL: &str = PROMISE_NEW.symbol;
pub const IS_PROMISE_SYMBOL: &str = IS_PROMISE.symbol;
pub const PROMISE_STATE_SYMBOL: &str = PROMISE_STATE.symbol;
pub const PROMISE_RESULT_SYMBOL: &str = PROMISE_RESULT.symbol;
pub const PROMISE_RESOLVE_SYMBOL: &str = PROMISE_RESOLVE.symbol;
pub const PROMISE_REJECT_SYMBOL: &str = PROMISE_REJECT.symbol;
pub const PROMISE_THEN_SYMBOL: &str = PROMISE_THEN.symbol;
pub const PROMISE_CONSTRUCT_SYMBOL: &str = PROMISE_CONSTRUCT.symbol;
pub const PROMISE_FINALLY_SYMBOL: &str = PROMISE_FINALLY.symbol;
pub const ARRAY_NEW_SYMBOL: &str = ARRAY_NEW.symbol;
pub const IS_ARRAY_SYMBOL: &str = IS_ARRAY.symbol;
pub const ARRAY_LEN_SYMBOL: &str = ARRAY_LEN.symbol;
pub const ARRAY_GET_SYMBOL: &str = ARRAY_GET.symbol;
pub const ARRAY_SET_SYMBOL: &str = ARRAY_SET.symbol;
pub const ARRAY_SPREAD_ARRAY_SYMBOL: &str = ARRAY_SPREAD_ARRAY.symbol;
pub const ARRAY_SPREAD_CSTR_SYMBOL: &str = ARRAY_SPREAD_CSTR.symbol;
pub const PROMISE_ALL_SYMBOL: &str = PROMISE_ALL.symbol;
pub const PROMISE_RACE_SYMBOL: &str = PROMISE_RACE.symbol;
pub const OBJECT_GET_SYMBOL: &str = OBJECT_GET.symbol;
pub const OBJECT_SET_SYMBOL: &str = OBJECT_SET.symbol;
pub const OBJECT_GET_BY_SYMBOL_SYMBOL: &str = OBJECT_GET_BY_SYMBOL.symbol;
pub const OBJECT_SET_BY_SYMBOL_SYMBOL: &str = OBJECT_SET_BY_SYMBOL.symbol;
pub const OBJECT_SET_PROTO_SYMBOL: &str = OBJECT_SET_PROTO.symbol;
pub const OBJECT_GET_PROTO_SYMBOL: &str = OBJECT_GET_PROTO.symbol;
pub const OBJECT_SPREAD_SYMBOL: &str = OBJECT_SPREAD.symbol;
pub const PROMISE_ALL_SETTLED_SYMBOL: &str = PROMISE_ALL_SETTLED.symbol;
pub const PROMISE_ANY_SYMBOL: &str = PROMISE_ANY.symbol;
pub const PROMISE_AWAIT_SYMBOL: &str = PROMISE_AWAIT.symbol;

/// Minimal std I/O + GC ABI symbols (N05 Runtime surface).
pub const MINIMAL_STD_AND_GC_SYMBOLS: &[&str] = &[
    HELLO_SYMBOL,
    PRINT_I64_SYMBOL,
    PRINT_U64_SYMBOL,
    PRINT_F64_SYMBOL,
    PRINT_BOOL_SYMBOL,
    PRINT_STR_SYMBOL,
    GC_INIT_SYMBOL,
    GC_SHUTDOWN_SYMBOL,
    ALLOC_STRING_SYMBOL,
    ALLOC_OBJECT_SYMBOL,
    GC_ROOT_PUSH_SYMBOL,
    GC_ROOT_POP_SYMBOL,
    GC_COLLECT_SYMBOL,
    GC_LIVE_COUNT_SYMBOL,
    GC_SET_ALLOC_THRESHOLD_SYMBOL,
    GC_ALLOC_THRESHOLD_SYMBOL,
    STRING_DATA_SYMBOL,
    STRING_LEN_SYMBOL,
    IS_STRING_SYMBOL,
    IS_OBJECT_SYMBOL,
];

/// Job queue ABI symbols (N06.01).
pub const JOB_QUEUE_SYMBOLS: &[&str] = &[JOB_ENQUEUE_SYMBOL, JOB_DRAIN_SYMBOL, JOB_PENDING_SYMBOL];

/// Host timer ABI symbols (H05.03–H05.04).
pub const TIMER_SYMBOLS: &[&str] = &[
    TIMER_SET_SYMBOL,
    TIMER_SET_INTERVAL_SYMBOL,
    TIMER_CLEAR_SYMBOL,
];

/// Declares for H05.03–H05.04 timer native emit.
pub const HOST_TIMER_DECLARES: &[AbiFn] = &[
    GC_INIT,
    PRINT_I64,
    PRINT_BOOL,
    PRINT_STR,
    JOB_DRAIN,
    TIMER_SET,
    TIMER_SET_INTERVAL,
    TIMER_CLEAR,
];

/// Promise ABI symbols (N06.02–N06.10).
pub const PROMISE_SYMBOLS: &[&str] = &[
    PROMISE_NEW_SYMBOL,
    IS_PROMISE_SYMBOL,
    PROMISE_STATE_SYMBOL,
    PROMISE_RESULT_SYMBOL,
    PROMISE_RESOLVE_SYMBOL,
    PROMISE_REJECT_SYMBOL,
    PROMISE_THEN_SYMBOL,
    PROMISE_CONSTRUCT_SYMBOL,
    PROMISE_FINALLY_SYMBOL,
    ARRAY_NEW_SYMBOL,
    IS_ARRAY_SYMBOL,
    ARRAY_LEN_SYMBOL,
    ARRAY_GET_SYMBOL,
    ARRAY_SET_SYMBOL,
    ARRAY_SPREAD_ARRAY_SYMBOL,
    ARRAY_SPREAD_CSTR_SYMBOL,
    PROMISE_ALL_SYMBOL,
    PROMISE_RACE_SYMBOL,
    OBJECT_GET_SYMBOL,
    OBJECT_SET_SYMBOL,
    OBJECT_SET_PROTO_SYMBOL,
    OBJECT_GET_PROTO_SYMBOL,
    OBJECT_SPREAD_SYMBOL,
    PROMISE_ALL_SETTLED_SYMBOL,
    PROMISE_ANY_SYMBOL,
    PROMISE_AWAIT_SYMBOL,
];

/// Declares used by the native scalar/layout emitter.
pub const NATIVE_INT_DECLARES: &[AbiFn] = &[PRINT_I64, PRINT_U64, PRINT_F64, PRINT_BOOL];

/// Declares used by the eval/Function observation emitter.
pub const ES_EVAL_DECLARES: &[AbiFn] = &[GC_INIT, PRINT_I64, PRINT_BOOL, PRINT_STR];

/// Declares used by the ES expression observation emitter (N08.01.* / N08.02.*).
pub const ES_EXPR_DECLARES: &[AbiFn] = &[
    PRINT_I64,
    PRINT_F64,
    PRINT_BOOL,
    PRINT_STR,
    PRINT_BYTES,
    CSTR_LEN,
    CSTR_CONCAT,
    CSTR_CONCAT_N,
    CSTR_FROM_U64,
    CSTR_FROM_CODE_UNIT,
    CSTR_FROM_CODE_UNIT_N,
    CSTR_EQ_N,
    UTF16_LEN,
];

/// Declares used by the Promise/async emitter.
pub const ES_PROMISE_DECLARES: &[AbiFn] = &[
    GC_INIT,
    PRINT_I64,
    PRINT_STR,
    JOB_DRAIN,
    PROMISE_NEW,
    PROMISE_RESOLVE,
    PROMISE_REJECT,
    PROMISE_CONSTRUCT,
    PROMISE_THEN,
    PROMISE_FINALLY,
    ARRAY_NEW,
    ARRAY_SET,
    ARRAY_GET,
    ARRAY_LEN,
    PROMISE_ALL,
    PROMISE_RACE,
    PROMISE_ALL_SETTLED,
    PROMISE_ANY,
    PROMISE_AWAIT,
    OBJECT_GET,
];


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
/* H11.01 / H11.02: TLS client/server wrap + read/write. */
pub const HOST_TLS_CLIENT_WRAP: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tls_client_wrap",
    ret: "i32",
    params: "i64, ptr, i32, ptr",
};
pub const HOST_TLS_SERVER_WRAP: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tls_server_wrap",
    ret: "i32",
    params: "i64, ptr, ptr, ptr",
};
pub const HOST_TLS_READ: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tls_read",
    ret: "i32",
    params: "i64, i64, ptr, ptr",
};
pub const HOST_TLS_WRITE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tls_write",
    ret: "i32",
    params: "i64, ptr, i64",
};
/* H06.01–H06.04: TCP listen/accept/connect/peer/read/write/shutdown. */
pub const HOST_TCP_LISTEN: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_listen",
    ret: "i32",
    params: "i32, i32, ptr",
};
pub const HOST_TCP_LOCAL_PORT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_local_port",
    ret: "i32",
    params: "i64, ptr",
};
pub const HOST_TCP_ACCEPT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_accept",
    ret: "i32",
    params: "i64, ptr",
};
pub const HOST_TCP_CONNECT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_connect",
    ret: "i32",
    params: "ptr, i32, ptr",
};
pub const HOST_TCP_PEER_PORT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_peer_port",
    ret: "i32",
    params: "i64, ptr",
};
pub const HOST_TCP_PEER_ADDRESS: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_peer_address",
    ret: "i32",
    params: "i64, ptr",
};
pub const HOST_TCP_READ: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_read",
    ret: "i32",
    params: "i64, i64, ptr, ptr",
};
pub const HOST_TCP_WRITE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_write",
    ret: "i32",
    params: "i64, ptr, i64",
};
pub const HOST_TCP_SHUTDOWN: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_shutdown",
    ret: "i32",
    params: "i64, i32",
};
/* H07.01: non-blocking readiness + job-queue completion. */
pub const HOST_TCP_SET_NONBLOCKING: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_set_nonblocking",
    ret: "i32",
    params: "i64, i32",
};
pub const HOST_IO_WAIT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_io_wait",
    ret: "i32",
    params: "i64, i32, ptr, ptr, ptr",
};
pub const HOST_IO_CANCEL: AbiFn = AbiFn {
    symbol: "draconic_rt_host_io_cancel",
    ret: "void",
    params: "i64",
};
pub const HOST_IO_PENDING: AbiFn = AbiFn {
    symbol: "draconic_rt_host_io_pending",
    ret: "i32",
    params: "",
};
pub const HOST_IO_POLL: AbiFn = AbiFn {
    symbol: "draconic_rt_host_io_poll",
    ret: "i32",
    params: "double",
};
/* H08.01: UDP bind/sendto/recvfrom. */
pub const HOST_UDP_BIND: AbiFn = AbiFn {
    symbol: "draconic_rt_host_udp_bind",
    ret: "i32",
    params: "i32, ptr",
};
pub const HOST_UDP_LOCAL_PORT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_udp_local_port",
    ret: "i32",
    params: "i64, ptr",
};
pub const HOST_UDP_SENDTO: AbiFn = AbiFn {
    symbol: "draconic_rt_host_udp_sendto",
    ret: "i32",
    params: "i64, ptr, i64, ptr, i32",
};
pub const HOST_UDP_RECVFROM: AbiFn = AbiFn {
    symbol: "draconic_rt_host_udp_recvfrom",
    ret: "i32",
    params: "i64, i64, ptr, ptr, ptr, ptr",
};
/* H09.01: DNS lookup hostname → IPv4 address strings. */
pub const HOST_DNS_LOOKUP: AbiFn = AbiFn {
    symbol: "draconic_rt_host_dns_lookup",
    ret: "i32",
    params: "ptr, ptr, ptr",
};
/* H10.01: HTTP/1.1 request parse (method/path/version/body + header lookup). */
pub const HOST_HTTP_PARSE_REQUEST: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http_parse_request",
    ret: "i32",
    params: "ptr, i64, ptr, ptr, ptr, ptr",
};
pub const HOST_HTTP_REQUEST_HEADER: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http_request_header",
    ret: "i32",
    params: "ptr, i64, ptr, ptr",
};
/* H10.02: HTTP/1.1 response write (status + reason + headers + body → message). */
pub const HOST_HTTP_WRITE_RESPONSE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http_write_response",
    ret: "i32",
    params: "i32, ptr, ptr, ptr, i64, ptr",
};
/* H17.03: one-shot static file serve on TCP connection under docroot. */
pub const HOST_HTTP_SERVE_STATIC: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http_serve_static",
    ret: "i32",
    params: "i64, ptr",
};
/* H10.05: HTTP/1.1 client — write request + parse response on connected TCP. */
pub const HOST_HTTP_WRITE_REQUEST: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http_write_request",
    ret: "i32",
    params: "ptr, ptr, ptr, ptr, i64, ptr",
};
pub const HOST_HTTP_PARSE_RESPONSE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http_parse_response",
    ret: "i32",
    params: "ptr, i64, ptr, ptr, ptr, ptr",
};
pub const HOST_HTTP_RESPONSE_HEADER: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http_response_header",
    ret: "i32",
    params: "ptr, i64, ptr, ptr",
};
/* H12.01: WebSocket server opening handshake response (RFC 6455). */
pub const HOST_WS_HANDSHAKE_RESPONSE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_handshake_response",
    ret: "i32",
    params: "ptr, ptr",
};
/* H12.02: WebSocket frames (RFC 6455 §5). */
pub const HOST_WS_ENCODE_TEXT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_encode_text",
    ret: "i32",
    params: "ptr, ptr, ptr",
};
pub const HOST_WS_ENCODE_BINARY: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_encode_binary",
    ret: "i32",
    params: "ptr, i64, ptr, ptr",
};
pub const HOST_WS_ENCODE_CLOSE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_encode_close",
    ret: "i32",
    params: "i32, ptr, ptr, ptr",
};
pub const HOST_WS_ENCODE_PING: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_encode_ping",
    ret: "i32",
    params: "ptr, ptr, ptr",
};
pub const HOST_WS_ENCODE_PONG: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_encode_pong",
    ret: "i32",
    params: "ptr, ptr, ptr",
};
pub const HOST_WS_DECODE_FRAME: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_decode_frame",
    ret: "i32",
    params: "ptr, i64, ptr, ptr, ptr, ptr, ptr",
};
/* H12.03: WebSocket client dial (handshake request, Accept check, masked text). */
pub const HOST_WS_CLIENT_HANDSHAKE_REQUEST: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_client_handshake_request",
    ret: "i32",
    params: "ptr, ptr, ptr, ptr",
};
pub const HOST_WS_CLIENT_CHECK_ACCEPT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_client_check_accept",
    ret: "i32",
    params: "ptr, i64, ptr",
};
pub const HOST_WS_ENCODE_TEXT_CLIENT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_encode_text_client",
    ret: "i32",
    params: "ptr, ptr, ptr",
};
/* H13.01: HTTP/2 preface + single-stream request/response. */
pub const HOST_HTTP2_CLIENT_PREFACE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_client_preface",
    ret: "i32",
    params: "ptr, ptr",
};
pub const HOST_HTTP2_SERVER_PREFACE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_server_preface",
    ret: "i32",
    params: "ptr, ptr",
};
pub const HOST_HTTP2_SETTINGS_ACK: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_settings_ack",
    ret: "i32",
    params: "ptr, ptr",
};
pub const HOST_HTTP2_ENCODE_REQUEST: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_encode_request",
    ret: "i32",
    params: "ptr, ptr, ptr, i64, ptr, ptr",
};
pub const HOST_HTTP2_ENCODE_RESPONSE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_encode_response",
    ret: "i32",
    params: "i32, ptr, i64, ptr, ptr",
};
pub const HOST_HTTP2_PARSE_REQUEST: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_parse_request",
    ret: "i32",
    params: "ptr, i64, ptr, ptr, ptr, ptr, ptr",
};
pub const HOST_HTTP2_PARSE_RESPONSE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_parse_response",
    ret: "i32",
    params: "ptr, i64, ptr, ptr, ptr, ptr",
};
pub const HOST_HTTP2_CLIENT_OPEN: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_client_open",
    ret: "i32",
    params: "ptr, ptr, ptr, i64, ptr, ptr",
};
pub const HOST_HTTP2_SERVER_REPLY: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_server_reply",
    ret: "i32",
    params: "i32, ptr, i64, ptr, ptr",
};
/* H07.02: async TCP → Promise. */
pub const HOST_TCP_ACCEPT_ASYNC: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_accept_async",
    ret: "ptr",
    params: "i64",
};
pub const HOST_TCP_CONNECT_ASYNC: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_connect_async",
    ret: "ptr",
    params: "ptr, i32",
};
pub const HOST_TCP_READ_ASYNC: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_read_async",
    ret: "ptr",
    params: "i64, i64",
};
pub const HOST_TCP_WRITE_ASYNC: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_write_async",
    ret: "ptr",
    params: "i64, ptr, i64",
};

/// Declares for H07.02 async TCP + Promise then + job drain.
pub const HOST_TCP_ASYNC_DECLARES: &[AbiFn] = &[
    GC_INIT,
    JOB_DRAIN,
    PROMISE_THEN,
    PRINT_I64,
    PRINT_STR,
    PRINT_BOOL,
    HOST_HANDLE_CLOSE,
    HOST_TCP_LISTEN,
    HOST_TCP_LOCAL_PORT,
    HOST_TCP_ACCEPT,
    HOST_TCP_CONNECT,
    HOST_TCP_ACCEPT_ASYNC,
    HOST_TCP_CONNECT_ASYNC,
    HOST_TCP_READ_ASYNC,
    HOST_TCP_WRITE_ASYNC,
];

pub const HOST_HANDLE_IS_VALID_SYMBOL: &str = HOST_HANDLE_IS_VALID.symbol;
pub const HOST_HANDLE_CLOSE_SYMBOL: &str = HOST_HANDLE_CLOSE.symbol;
pub const HOST_PATH_FROM_UTF8_SYMBOL: &str = HOST_PATH_FROM_UTF8.symbol;
pub const HOST_PATH_FREE_SYMBOL: &str = HOST_PATH_FREE.symbol;
pub const HOST_BYTES_FROM_RAW_SYMBOL: &str = HOST_BYTES_FROM_RAW.symbol;
pub const HOST_BYTES_VIEW_SYMBOL: &str = HOST_BYTES_VIEW.symbol;
pub const HOST_BYTES_ALLOC_SYMBOL: &str = HOST_BYTES_ALLOC.symbol;
pub const HOST_BYTES_STORAGE_FREE_SYMBOL: &str = HOST_BYTES_STORAGE_FREE.symbol;
pub const HOST_BYTES_COPY_IN_SYMBOL: &str = HOST_BYTES_COPY_IN.symbol;
pub const HOST_BYTES_COPY_OUT_SYMBOL: &str = HOST_BYTES_COPY_OUT.symbol;
pub const HOST_PROCESS_SET_ARGV_SYMBOL: &str = HOST_PROCESS_SET_ARGV.symbol;
pub const HOST_PROCESS_USER_ARGC_SYMBOL: &str = HOST_PROCESS_USER_ARGC.symbol;
pub const HOST_PROCESS_USER_ARG_SYMBOL: &str = HOST_PROCESS_USER_ARG.symbol;
pub const HOST_ENV_GET_SYMBOL: &str = HOST_ENV_GET.symbol;
pub const HOST_ENV_SET_SYMBOL: &str = HOST_ENV_SET.symbol;
pub const HOST_ENV_DELETE_SYMBOL: &str = HOST_ENV_DELETE.symbol;
pub const HOST_PROCESS_EXIT_SYMBOL: &str = HOST_PROCESS_EXIT.symbol;
pub const HOST_PROCESS_SET_EXIT_CODE_SYMBOL: &str = HOST_PROCESS_SET_EXIT_CODE.symbol;
pub const HOST_PROCESS_GET_EXIT_CODE_SYMBOL: &str = HOST_PROCESS_GET_EXIT_CODE.symbol;
pub const HOST_PROCESS_PID_SYMBOL: &str = HOST_PROCESS_PID.symbol;
pub const HOST_PROCESS_PPID_SYMBOL: &str = HOST_PROCESS_PPID.symbol;
pub const HOST_CWD_SYMBOL: &str = HOST_CWD.symbol;
pub const HOST_CHDIR_SYMBOL: &str = HOST_CHDIR.symbol;
pub const HOST_HOSTNAME_SYMBOL: &str = HOST_HOSTNAME.symbol;
pub const HOST_OS_TYPE_SYMBOL: &str = HOST_OS_TYPE.symbol;
pub const HOST_OS_ARCH_SYMBOL: &str = HOST_OS_ARCH.symbol;
pub const HOST_SIGNAL_WATCH_SYMBOL: &str = HOST_SIGNAL_WATCH.symbol;
pub const HOST_SIGNAL_RAISE_SYMBOL: &str = HOST_SIGNAL_RAISE.symbol;
pub const HOST_SIGNAL_POLL_SYMBOL: &str = HOST_SIGNAL_POLL.symbol;
pub const HOST_NOW_MS_SYMBOL: &str = HOST_NOW_MS.symbol;
pub const HOST_MONOTONIC_MS_SYMBOL: &str = HOST_MONOTONIC_MS.symbol;
pub const HOST_STDOUT_WRITE_SYMBOL: &str = HOST_STDOUT_WRITE.symbol;
pub const HOST_STDERR_WRITE_SYMBOL: &str = HOST_STDERR_WRITE.symbol;
pub const HOST_STDIN_READ_LINE_SYMBOL: &str = HOST_STDIN_READ_LINE.symbol;
pub const HOST_STDIN_READ_BYTES_SYMBOL: &str = HOST_STDIN_READ_BYTES.symbol;
pub const HOST_PATH_NORMALIZE_SYMBOL: &str = HOST_PATH_NORMALIZE.symbol;
pub const HOST_PATH_JOIN_SYMBOL: &str = HOST_PATH_JOIN.symbol;
pub const HOST_PATH_DIRNAME_SYMBOL: &str = HOST_PATH_DIRNAME.symbol;
pub const HOST_PATH_BASENAME_SYMBOL: &str = HOST_PATH_BASENAME.symbol;
pub const HOST_PATH_EXTNAME_SYMBOL: &str = HOST_PATH_EXTNAME.symbol;
pub const HOST_PATH_IS_ABSOLUTE_SYMBOL: &str = HOST_PATH_IS_ABSOLUTE.symbol;
pub const HOST_PATH_RESOLVE_SYMBOL: &str = HOST_PATH_RESOLVE.symbol;
pub const HOST_FS_READ_FILE_SYMBOL: &str = HOST_FS_READ_FILE.symbol;
pub const HOST_FS_READ_TEXT_SYMBOL: &str = HOST_FS_READ_TEXT.symbol;
pub const HOST_FS_WRITE_FILE_SYMBOL: &str = HOST_FS_WRITE_FILE.symbol;
pub const HOST_FS_APPEND_FILE_SYMBOL: &str = HOST_FS_APPEND_FILE.symbol;
pub const HOST_FS_WRITE_TEXT_SYMBOL: &str = HOST_FS_WRITE_TEXT.symbol;
pub const HOST_FS_APPEND_TEXT_SYMBOL: &str = HOST_FS_APPEND_TEXT.symbol;
pub const HOST_FS_EXISTS_SYMBOL: &str = HOST_FS_EXISTS.symbol;
pub const HOST_FS_STAT_SYMBOL: &str = HOST_FS_STAT.symbol;
pub const HOST_FS_MKDIR_SYMBOL: &str = HOST_FS_MKDIR.symbol;
pub const HOST_FS_MKDIR_ALL_SYMBOL: &str = HOST_FS_MKDIR_ALL.symbol;
pub const HOST_FS_READDIR_SYMBOL: &str = HOST_FS_READDIR.symbol;
pub const HOST_FS_RMDIR_SYMBOL: &str = HOST_FS_RMDIR.symbol;
pub const HOST_FS_REMOVE_FILE_SYMBOL: &str = HOST_FS_REMOVE_FILE.symbol;
pub const HOST_FS_RENAME_FILE_SYMBOL: &str = HOST_FS_RENAME_FILE.symbol;
pub const HOST_FS_COPY_FILE_SYMBOL: &str = HOST_FS_COPY_FILE.symbol;
pub const HOST_FS_OPEN_SYMBOL: &str = HOST_FS_OPEN.symbol;
pub const HOST_FS_HANDLE_READ_SYMBOL: &str = HOST_FS_HANDLE_READ.symbol;
pub const HOST_FS_HANDLE_WRITE_SYMBOL: &str = HOST_FS_HANDLE_WRITE.symbol;
pub const HOST_FS_HANDLE_SEEK_SYMBOL: &str = HOST_FS_HANDLE_SEEK.symbol;
pub const HOST_TCP_LISTEN_SYMBOL: &str = HOST_TCP_LISTEN.symbol;
pub const HOST_TCP_LOCAL_PORT_SYMBOL: &str = HOST_TCP_LOCAL_PORT.symbol;
pub const HOST_TCP_ACCEPT_SYMBOL: &str = HOST_TCP_ACCEPT.symbol;
pub const HOST_TCP_CONNECT_SYMBOL: &str = HOST_TCP_CONNECT.symbol;
pub const HOST_TCP_PEER_PORT_SYMBOL: &str = HOST_TCP_PEER_PORT.symbol;
pub const HOST_TCP_PEER_ADDRESS_SYMBOL: &str = HOST_TCP_PEER_ADDRESS.symbol;
pub const HOST_TCP_READ_SYMBOL: &str = HOST_TCP_READ.symbol;
pub const HOST_TCP_WRITE_SYMBOL: &str = HOST_TCP_WRITE.symbol;
pub const HOST_TCP_SHUTDOWN_SYMBOL: &str = HOST_TCP_SHUTDOWN.symbol;
pub const HOST_TCP_SET_NONBLOCKING_SYMBOL: &str = HOST_TCP_SET_NONBLOCKING.symbol;
pub const HOST_IO_WAIT_SYMBOL: &str = HOST_IO_WAIT.symbol;
pub const HOST_IO_CANCEL_SYMBOL: &str = HOST_IO_CANCEL.symbol;
pub const HOST_IO_PENDING_SYMBOL: &str = HOST_IO_PENDING.symbol;
pub const HOST_IO_POLL_SYMBOL: &str = HOST_IO_POLL.symbol;
pub const HOST_TCP_ACCEPT_ASYNC_SYMBOL: &str = HOST_TCP_ACCEPT_ASYNC.symbol;
pub const HOST_TCP_CONNECT_ASYNC_SYMBOL: &str = HOST_TCP_CONNECT_ASYNC.symbol;
pub const HOST_TCP_READ_ASYNC_SYMBOL: &str = HOST_TCP_READ_ASYNC.symbol;
pub const HOST_TCP_WRITE_ASYNC_SYMBOL: &str = HOST_TCP_WRITE_ASYNC.symbol;
pub const HOST_UDP_BIND_SYMBOL: &str = HOST_UDP_BIND.symbol;
pub const HOST_UDP_LOCAL_PORT_SYMBOL: &str = HOST_UDP_LOCAL_PORT.symbol;
pub const HOST_UDP_SENDTO_SYMBOL: &str = HOST_UDP_SENDTO.symbol;
pub const HOST_UDP_RECVFROM_SYMBOL: &str = HOST_UDP_RECVFROM.symbol;
pub const HOST_DNS_LOOKUP_SYMBOL: &str = HOST_DNS_LOOKUP.symbol;
pub const HOST_HTTP_PARSE_REQUEST_SYMBOL: &str = HOST_HTTP_PARSE_REQUEST.symbol;
pub const HOST_HTTP_REQUEST_HEADER_SYMBOL: &str = HOST_HTTP_REQUEST_HEADER.symbol;
pub const HOST_HTTP_WRITE_RESPONSE_SYMBOL: &str = HOST_HTTP_WRITE_RESPONSE.symbol;
pub const HOST_HTTP_SERVE_STATIC_SYMBOL: &str = HOST_HTTP_SERVE_STATIC.symbol;
pub const HOST_HTTP_WRITE_REQUEST_SYMBOL: &str = HOST_HTTP_WRITE_REQUEST.symbol;
pub const HOST_HTTP_PARSE_RESPONSE_SYMBOL: &str = HOST_HTTP_PARSE_RESPONSE.symbol;
pub const HOST_HTTP_RESPONSE_HEADER_SYMBOL: &str = HOST_HTTP_RESPONSE_HEADER.symbol;
pub const HOST_WORKER_SPAWN_SYMBOL: &str = HOST_WORKER_SPAWN.symbol;
pub const HOST_WORKER_JOIN_SYMBOL: &str = HOST_WORKER_JOIN.symbol;
pub const HOST_WORKER_TERMINATE_SYMBOL: &str = HOST_WORKER_TERMINATE.symbol;
pub const HOST_WORKER_OS_THREAD_SYMBOL: &str = HOST_WORKER_OS_THREAD.symbol;
pub const HOST_CHANNEL_MAKE_SYMBOL: &str = HOST_CHANNEL_MAKE.symbol;
pub const HOST_CHANNEL_SEND_F64_SYMBOL: &str = HOST_CHANNEL_SEND_F64.symbol;
pub const HOST_CHANNEL_SEND_STR_SYMBOL: &str = HOST_CHANNEL_SEND_STR.symbol;
pub const HOST_CHANNEL_SEND_BOOL_SYMBOL: &str = HOST_CHANNEL_SEND_BOOL.symbol;
pub const HOST_CHANNEL_RECV_F64_SYMBOL: &str = HOST_CHANNEL_RECV_F64.symbol;
pub const HOST_CHANNEL_RECV_STR_SYMBOL: &str = HOST_CHANNEL_RECV_STR.symbol;
pub const HOST_CHANNEL_RECV_BOOL_SYMBOL: &str = HOST_CHANNEL_RECV_BOOL.symbol;
pub const HOST_CHANNEL_SEND_OBJ_SYMBOL: &str = HOST_CHANNEL_SEND_OBJ.symbol;
pub const HOST_CHANNEL_RECV_OBJ_SYMBOL: &str = HOST_CHANNEL_RECV_OBJ.symbol;
pub const HOST_WS_HANDSHAKE_RESPONSE_SYMBOL: &str = HOST_WS_HANDSHAKE_RESPONSE.symbol;
pub const HOST_WS_ENCODE_TEXT_SYMBOL: &str = HOST_WS_ENCODE_TEXT.symbol;
pub const HOST_WS_ENCODE_BINARY_SYMBOL: &str = HOST_WS_ENCODE_BINARY.symbol;
pub const HOST_WS_ENCODE_CLOSE_SYMBOL: &str = HOST_WS_ENCODE_CLOSE.symbol;
pub const HOST_WS_ENCODE_PING_SYMBOL: &str = HOST_WS_ENCODE_PING.symbol;
pub const HOST_WS_ENCODE_PONG_SYMBOL: &str = HOST_WS_ENCODE_PONG.symbol;
pub const HOST_WS_DECODE_FRAME_SYMBOL: &str = HOST_WS_DECODE_FRAME.symbol;
pub const HOST_WS_CLIENT_HANDSHAKE_REQUEST_SYMBOL: &str = HOST_WS_CLIENT_HANDSHAKE_REQUEST.symbol;
pub const HOST_WS_CLIENT_CHECK_ACCEPT_SYMBOL: &str = HOST_WS_CLIENT_CHECK_ACCEPT.symbol;
pub const HOST_WS_ENCODE_TEXT_CLIENT_SYMBOL: &str = HOST_WS_ENCODE_TEXT_CLIENT.symbol;
pub const HOST_HTTP2_CLIENT_PREFACE_SYMBOL: &str = HOST_HTTP2_CLIENT_PREFACE.symbol;
pub const HOST_HTTP2_SERVER_PREFACE_SYMBOL: &str = HOST_HTTP2_SERVER_PREFACE.symbol;
pub const HOST_HTTP2_SETTINGS_ACK_SYMBOL: &str = HOST_HTTP2_SETTINGS_ACK.symbol;
pub const HOST_HTTP2_ENCODE_REQUEST_SYMBOL: &str = HOST_HTTP2_ENCODE_REQUEST.symbol;
pub const HOST_HTTP2_ENCODE_RESPONSE_SYMBOL: &str = HOST_HTTP2_ENCODE_RESPONSE.symbol;
pub const HOST_HTTP2_PARSE_REQUEST_SYMBOL: &str = HOST_HTTP2_PARSE_REQUEST.symbol;
pub const HOST_HTTP2_PARSE_RESPONSE_SYMBOL: &str = HOST_HTTP2_PARSE_RESPONSE.symbol;
pub const HOST_HTTP2_CLIENT_OPEN_SYMBOL: &str = HOST_HTTP2_CLIENT_OPEN.symbol;
pub const HOST_HTTP2_SERVER_REPLY_SYMBOL: &str = HOST_HTTP2_SERVER_REPLY.symbol;
pub const HOST_TLS_CLIENT_WRAP_SYMBOL: &str = HOST_TLS_CLIENT_WRAP.symbol;
pub const HOST_TLS_SERVER_WRAP_SYMBOL: &str = HOST_TLS_SERVER_WRAP.symbol;
pub const HOST_TLS_READ_SYMBOL: &str = HOST_TLS_READ.symbol;
pub const HOST_TLS_WRITE_SYMBOL: &str = HOST_TLS_WRITE.symbol;

/// Host Runtime ABI symbols (H00.02 scaffold + H00.03 bytes + H01–H11.01).
pub const HOST_SYMBOLS: &[&str] = &[
    HOST_HANDLE_IS_VALID_SYMBOL,
    HOST_HANDLE_CLOSE_SYMBOL,
    HOST_PATH_FROM_UTF8_SYMBOL,
    HOST_PATH_FREE_SYMBOL,
    HOST_BYTES_FROM_RAW_SYMBOL,
    HOST_BYTES_VIEW_SYMBOL,
    HOST_BYTES_ALLOC_SYMBOL,
    HOST_BYTES_STORAGE_FREE_SYMBOL,
    HOST_BYTES_COPY_IN_SYMBOL,
    HOST_BYTES_COPY_OUT_SYMBOL,
    HOST_PROCESS_SET_ARGV_SYMBOL,
    HOST_PROCESS_USER_ARGC_SYMBOL,
    HOST_PROCESS_USER_ARG_SYMBOL,
    HOST_ENV_GET_SYMBOL,
    HOST_ENV_SET_SYMBOL,
    HOST_ENV_DELETE_SYMBOL,
    HOST_PROCESS_EXIT_SYMBOL,
    HOST_PROCESS_SET_EXIT_CODE_SYMBOL,
    HOST_PROCESS_GET_EXIT_CODE_SYMBOL,
    HOST_PROCESS_PID_SYMBOL,
    HOST_PROCESS_PPID_SYMBOL,
    HOST_CWD_SYMBOL,
    HOST_CHDIR_SYMBOL,
    HOST_HOSTNAME_SYMBOL,
    HOST_OS_TYPE_SYMBOL,
    HOST_OS_ARCH_SYMBOL,
    HOST_SIGNAL_WATCH_SYMBOL,
    HOST_SIGNAL_RAISE_SYMBOL,
    HOST_SIGNAL_POLL_SYMBOL,
    HOST_NOW_MS_SYMBOL,
    HOST_MONOTONIC_MS_SYMBOL,
    HOST_STDOUT_WRITE_SYMBOL,
    HOST_STDERR_WRITE_SYMBOL,
    HOST_STDIN_READ_LINE_SYMBOL,
    HOST_STDIN_READ_BYTES_SYMBOL,
    HOST_PATH_NORMALIZE_SYMBOL,
    HOST_PATH_JOIN_SYMBOL,
    HOST_PATH_DIRNAME_SYMBOL,
    HOST_PATH_BASENAME_SYMBOL,
    HOST_PATH_EXTNAME_SYMBOL,
    HOST_PATH_IS_ABSOLUTE_SYMBOL,
    HOST_PATH_RESOLVE_SYMBOL,
    HOST_FS_READ_FILE_SYMBOL,
    HOST_FS_READ_TEXT_SYMBOL,
    HOST_FS_WRITE_FILE_SYMBOL,
    HOST_FS_APPEND_FILE_SYMBOL,
    HOST_FS_WRITE_TEXT_SYMBOL,
    HOST_FS_APPEND_TEXT_SYMBOL,
    HOST_FS_EXISTS_SYMBOL,
    HOST_FS_STAT_SYMBOL,
    HOST_FS_MKDIR_SYMBOL,
    HOST_FS_MKDIR_ALL_SYMBOL,
    HOST_FS_READDIR_SYMBOL,
    HOST_FS_RMDIR_SYMBOL,
    HOST_FS_REMOVE_FILE_SYMBOL,
    HOST_FS_RENAME_FILE_SYMBOL,
    HOST_FS_COPY_FILE_SYMBOL,
    HOST_FS_OPEN_SYMBOL,
    HOST_FS_HANDLE_READ_SYMBOL,
    HOST_FS_HANDLE_WRITE_SYMBOL,
    HOST_FS_HANDLE_SEEK_SYMBOL,
    HOST_TCP_LISTEN_SYMBOL,
    HOST_TCP_LOCAL_PORT_SYMBOL,
    HOST_TCP_ACCEPT_SYMBOL,
    HOST_TCP_CONNECT_SYMBOL,
    HOST_TCP_PEER_PORT_SYMBOL,
    HOST_TCP_PEER_ADDRESS_SYMBOL,
    HOST_TCP_READ_SYMBOL,
    HOST_TCP_WRITE_SYMBOL,
    HOST_TCP_SHUTDOWN_SYMBOL,
    HOST_TCP_SET_NONBLOCKING_SYMBOL,
    HOST_IO_WAIT_SYMBOL,
    HOST_IO_CANCEL_SYMBOL,
    HOST_IO_PENDING_SYMBOL,
    HOST_IO_POLL_SYMBOL,
    HOST_TCP_ACCEPT_ASYNC_SYMBOL,
    HOST_TCP_CONNECT_ASYNC_SYMBOL,
    HOST_TCP_READ_ASYNC_SYMBOL,
    HOST_TCP_WRITE_ASYNC_SYMBOL,
    HOST_UDP_BIND_SYMBOL,
    HOST_UDP_LOCAL_PORT_SYMBOL,
    HOST_UDP_SENDTO_SYMBOL,
    HOST_UDP_RECVFROM_SYMBOL,
    HOST_DNS_LOOKUP_SYMBOL,
    HOST_HTTP_PARSE_REQUEST_SYMBOL,
    HOST_HTTP_REQUEST_HEADER_SYMBOL,
    HOST_HTTP_WRITE_RESPONSE_SYMBOL,
    HOST_HTTP_SERVE_STATIC_SYMBOL,
    HOST_HTTP_WRITE_REQUEST_SYMBOL,
    HOST_WS_HANDSHAKE_RESPONSE_SYMBOL,
    HOST_WS_ENCODE_TEXT_SYMBOL,
    HOST_WS_ENCODE_BINARY_SYMBOL,
    HOST_WS_ENCODE_CLOSE_SYMBOL,
    HOST_WS_ENCODE_PING_SYMBOL,
    HOST_WS_ENCODE_PONG_SYMBOL,
    HOST_WS_DECODE_FRAME_SYMBOL,
    HOST_WS_CLIENT_HANDSHAKE_REQUEST_SYMBOL,
    HOST_WS_CLIENT_CHECK_ACCEPT_SYMBOL,
    HOST_WS_ENCODE_TEXT_CLIENT_SYMBOL,
    HOST_HTTP2_CLIENT_PREFACE_SYMBOL,
    HOST_HTTP2_SERVER_PREFACE_SYMBOL,
    HOST_HTTP2_SETTINGS_ACK_SYMBOL,
    HOST_HTTP2_ENCODE_REQUEST_SYMBOL,
    HOST_HTTP2_ENCODE_RESPONSE_SYMBOL,
    HOST_HTTP2_PARSE_REQUEST_SYMBOL,
    HOST_HTTP2_PARSE_RESPONSE_SYMBOL,
    HOST_HTTP2_CLIENT_OPEN_SYMBOL,
    HOST_HTTP2_SERVER_REPLY_SYMBOL,
    HOST_TLS_CLIENT_WRAP_SYMBOL,
    HOST_TLS_SERVER_WRAP_SYMBOL,
    HOST_TLS_READ_SYMBOL,
    HOST_TLS_WRITE_SYMBOL,
    HOST_HTTP_PARSE_RESPONSE_SYMBOL,
    HOST_HTTP_RESPONSE_HEADER_SYMBOL,
    HOST_WORKER_SPAWN_SYMBOL,
    HOST_WORKER_JOIN_SYMBOL,
    HOST_WORKER_TERMINATE_SYMBOL,
    HOST_WORKER_OS_THREAD_SYMBOL,
    HOST_CHANNEL_MAKE_SYMBOL,
    HOST_CHANNEL_SEND_F64_SYMBOL,
    HOST_CHANNEL_SEND_STR_SYMBOL,
    HOST_CHANNEL_SEND_BOOL_SYMBOL,
    HOST_CHANNEL_RECV_F64_SYMBOL,
    HOST_CHANNEL_RECV_STR_SYMBOL,
    HOST_CHANNEL_RECV_BOOL_SYMBOL,
    HOST_CHANNEL_SEND_OBJ_SYMBOL,
    HOST_CHANNEL_RECV_OBJ_SYMBOL,
];

/// Declares used when emitting host I/O calls (H01+).
pub const HOST_DECLARES: &[AbiFn] = &[
    HOST_HANDLE_IS_VALID,
    HOST_HANDLE_CLOSE,
    HOST_PATH_FROM_UTF8,
    HOST_PATH_FREE,
    HOST_BYTES_FROM_RAW,
    HOST_BYTES_VIEW,
    HOST_BYTES_ALLOC,
    HOST_BYTES_STORAGE_FREE,
    HOST_BYTES_COPY_IN,
    HOST_BYTES_COPY_OUT,
    HOST_PROCESS_SET_ARGV,
    HOST_PROCESS_USER_ARGC,
    HOST_PROCESS_USER_ARG,
    HOST_ENV_GET,
    HOST_ENV_SET,
    HOST_ENV_DELETE,
    HOST_PROCESS_EXIT,
    HOST_PROCESS_SET_EXIT_CODE,
    HOST_PROCESS_GET_EXIT_CODE,
    HOST_PROCESS_PID,
    HOST_PROCESS_PPID,
    HOST_CWD,
    HOST_CHDIR,
    HOST_HOSTNAME,
    HOST_OS_TYPE,
    HOST_OS_ARCH,
    HOST_NOW_MS,
    HOST_MONOTONIC_MS,
    HOST_STDOUT_WRITE,
    HOST_STDERR_WRITE,
    HOST_STDIN_READ_LINE,
    HOST_STDIN_READ_BYTES,
    HOST_PATH_NORMALIZE,
    HOST_PATH_JOIN,
    HOST_PATH_DIRNAME,
    HOST_PATH_BASENAME,
    HOST_PATH_EXTNAME,
    HOST_PATH_IS_ABSOLUTE,
    HOST_PATH_RESOLVE,
    HOST_FS_READ_FILE,
    HOST_FS_READ_TEXT,
    HOST_FS_WRITE_FILE,
    HOST_FS_APPEND_FILE,
    HOST_FS_WRITE_TEXT,
    HOST_FS_APPEND_TEXT,
    HOST_FS_EXISTS,
    HOST_FS_STAT,
    HOST_FS_MKDIR,
    HOST_FS_MKDIR_ALL,
    HOST_FS_READDIR,
    HOST_FS_RMDIR,
    HOST_FS_REMOVE_FILE,
    HOST_FS_RENAME_FILE,
    HOST_FS_COPY_FILE,
    HOST_FS_OPEN,
    HOST_FS_HANDLE_READ,
    HOST_FS_HANDLE_WRITE,
    HOST_FS_HANDLE_SEEK,
    HOST_TCP_LISTEN,
    HOST_TCP_LOCAL_PORT,
    HOST_TCP_ACCEPT,
    HOST_TCP_CONNECT,
    HOST_TCP_PEER_PORT,
    HOST_TCP_PEER_ADDRESS,
    HOST_TCP_READ,
    HOST_TCP_WRITE,
    HOST_TCP_SHUTDOWN,
    HOST_TCP_SET_NONBLOCKING,
    HOST_IO_WAIT,
    HOST_IO_CANCEL,
    HOST_IO_PENDING,
    HOST_IO_POLL,
    HOST_TCP_ACCEPT_ASYNC,
    HOST_TCP_CONNECT_ASYNC,
    HOST_TCP_READ_ASYNC,
    HOST_TCP_WRITE_ASYNC,
    HOST_UDP_BIND,
    HOST_UDP_LOCAL_PORT,
    HOST_UDP_SENDTO,
    HOST_UDP_RECVFROM,
    HOST_DNS_LOOKUP,
    HOST_HTTP_PARSE_REQUEST,
    HOST_HTTP_REQUEST_HEADER,
    HOST_HTTP_WRITE_RESPONSE,
    HOST_HTTP_SERVE_STATIC,
    HOST_HTTP_WRITE_REQUEST,
    HOST_HTTP_PARSE_RESPONSE,
    HOST_HTTP_RESPONSE_HEADER,
    HOST_WS_HANDSHAKE_RESPONSE,
    HOST_WS_ENCODE_TEXT,
    HOST_WS_ENCODE_BINARY,
    HOST_WS_ENCODE_CLOSE,
    HOST_WS_ENCODE_PING,
    HOST_WS_ENCODE_PONG,
    HOST_WS_DECODE_FRAME,
    HOST_WS_CLIENT_HANDSHAKE_REQUEST,
    HOST_WS_CLIENT_CHECK_ACCEPT,
    HOST_WS_ENCODE_TEXT_CLIENT,
    HOST_HTTP2_CLIENT_PREFACE,
    HOST_HTTP2_SERVER_PREFACE,
    HOST_HTTP2_SETTINGS_ACK,
    HOST_HTTP2_ENCODE_REQUEST,
    HOST_HTTP2_ENCODE_RESPONSE,
    HOST_HTTP2_PARSE_REQUEST,
    HOST_HTTP2_PARSE_RESPONSE,
    HOST_HTTP2_CLIENT_OPEN,
    HOST_HTTP2_SERVER_REPLY,
    HOST_TLS_CLIENT_WRAP,
    HOST_TLS_SERVER_WRAP,
    HOST_TLS_READ,
    HOST_TLS_WRITE,
    HOST_WORKER_SPAWN,
    HOST_WORKER_JOIN,
    HOST_WORKER_TERMINATE,
    HOST_WORKER_OS_THREAD,
    HOST_CHANNEL_MAKE,
    HOST_CHANNEL_SEND_F64,
    HOST_CHANNEL_SEND_STR,
    HOST_CHANNEL_SEND_BOOL,
    HOST_CHANNEL_RECV_F64,
    HOST_CHANNEL_RECV_STR,
    HOST_CHANNEL_RECV_BOOL,
    HOST_CHANNEL_SEND_OBJ,
    HOST_CHANNEL_RECV_OBJ,
];

/// JS polyfill for `processArgs()` (H01.01): user program args as string[].
///
/// Node bridge: `process.argv` without the executable; if `argv[1]` looks like a
/// script path (`.js`/`.mjs`/`.cjs`/`.drac`), skip it too (file run). Eval-style
/// (`node -e`) has no script slot — user args start at index 1.
pub fn process_args_js_polyfill() -> &'static str {
    r#"function processArgs() {
  var a = (typeof process !== "undefined" && process && process.argv) ? process.argv : [];
  if (!a || a.length <= 1) return [];
  var first = a[1];
  if (typeof first === "string" && /\.(m?js|cjs|drac)$/i.test(first)) {
    return a.slice(2).map(String);
  }
  return a.slice(1).map(String);
}
if (typeof globalThis !== "undefined") globalThis.processArgs = processArgs;
"#
}

/// JS polyfill for `envGet` / `envSet` / `envDelete` (H01.02).
///
/// Node bridge via `process.env`. Missing key → `undefined`. Values coerced to string.
pub fn process_env_js_polyfill() -> &'static str {
    r#"function envGet(key) {
  if (typeof process === "undefined" || !process || !process.env) return undefined;
  var v = process.env[String(key)];
  if (v === undefined || v === null) return undefined;
  return String(v);
}
function envSet(key, value) {
  if (typeof process === "undefined" || !process) return;
  if (!process.env) process.env = {};
  process.env[String(key)] = String(value);
}
function envDelete(key) {
  if (typeof process === "undefined" || !process || !process.env) return;
  delete process.env[String(key)];
}
if (typeof globalThis !== "undefined") {
  globalThis.envGet = envGet;
  globalThis.envSet = envSet;
  globalThis.envDelete = envDelete;
}
"#
}

/// JS polyfill for `exit` / `exitCode` / `setExitCode` (H01.03).
///
/// Node bridge via `process.exit` and `process.exitCode`. Bare `exit()` uses
/// the deferred code (default 0).
pub fn process_exit_js_polyfill() -> &'static str {
    r#"var __draconic_exitCode = 0;
function exitCode() {
  if (typeof process !== "undefined" && process && process.exitCode != null && process.exitCode !== undefined) {
    return Number(process.exitCode) | 0;
  }
  return __draconic_exitCode | 0;
}
function setExitCode(code) {
  var n = (code === undefined || code === null) ? 0 : (Number(code) | 0);
  __draconic_exitCode = n;
  if (typeof process !== "undefined" && process) process.exitCode = n;
}
function exit(code) {
  var n;
  if (arguments.length === 0 || code === undefined || code === null) {
    n = exitCode();
  } else {
    n = Number(code) | 0;
  }
  if (typeof process !== "undefined" && process && typeof process.exit === "function") {
    process.exit(n);
  }
  throw new Error("exit(" + n + ")");
}
if (typeof globalThis !== "undefined") {
  globalThis.exit = exit;
  globalThis.exitCode = exitCode;
  globalThis.setExitCode = setExitCode;
}
"#
}

/// JS polyfill for `pid` / `ppid` (H01.04).
///
/// Node bridge via `process.pid` and `process.ppid` (read-only numbers).
pub fn process_pid_js_polyfill() -> &'static str {
    r#"function pid() {
  if (typeof process !== "undefined" && process && process.pid != null) {
    return Number(process.pid) | 0;
  }
  return 0;
}
function ppid() {
  if (typeof process !== "undefined" && process && process.ppid != null) {
    return Number(process.ppid) | 0;
  }
  return 0;
}
if (typeof globalThis !== "undefined") {
  globalThis.pid = pid;
  globalThis.ppid = ppid;
}
"#
}

/// JS polyfill for `cwd` / `chdir` (H16.01).
///
/// Node bridge via `process.cwd` / `process.chdir`.
pub fn cwd_chdir_js_polyfill() -> &'static str {
    r#"function cwd() {
  if (typeof process !== "undefined" && process && typeof process.cwd === "function") {
    return process.cwd();
  }
  return "";
}
function chdir(path) {
  if (typeof process !== "undefined" && process && typeof process.chdir === "function") {
    process.chdir(String(path));
    return;
  }
  throw new Error("chdir unavailable");
}
if (typeof globalThis !== "undefined") {
  globalThis.cwd = cwd;
  globalThis.chdir = chdir;
}
"#
}

/// JS polyfill for `hostname` / `osType` / `osArch` (H16.02).
///
/// Node bridge via `os.hostname` / `os.platform` / `os.arch`.
pub fn hostname_os_js_polyfill() -> &'static str {
    r#"function hostname() {
  try {
    var os = require("os");
    if (os && typeof os.hostname === "function") return String(os.hostname());
  } catch (e) {}
  return "";
}
function osType() {
  try {
    var os = require("os");
    if (os && typeof os.platform === "function") return String(os.platform());
  } catch (e) {}
  return "";
}
function osArch() {
  try {
    var os = require("os");
    if (os && typeof os.arch === "function") return String(os.arch());
  } catch (e) {}
  return "";
}
if (typeof globalThis !== "undefined") {
  globalThis.hostname = hostname;
  globalThis.osType = osType;
  globalThis.osArch = osArch;
}
"#
}

/// JS polyfill for `tempDir` / `homeDir` (H16.03).
///
/// Node bridge via `os.tmpdir` / `os.homedir`.
pub fn temp_home_js_polyfill() -> &'static str {
    r#"function tempDir() {
  try {
    var os = require("os");
    if (os && typeof os.tmpdir === "function") return String(os.tmpdir());
  } catch (e) {}
  return "";
}
function homeDir() {
  try {
    var os = require("os");
    if (os && typeof os.homedir === "function") return String(os.homedir());
  } catch (e) {}
  return "";
}
if (typeof globalThis !== "undefined") {
  globalThis.tempDir = tempDir;
  globalThis.homeDir = homeDir;
}
"#
}

/// JS polyfill for `processRun` (H15.01).
///
/// Node bridge via `child_process.spawnSync`. argv[0] is the program; remaining
/// elements are args. Optional cwd (null/undefined → inherit). Optional env
/// object merges onto `process.env` (subset override). Returns exit status;
/// spawn failure → -1; killed by signal → 128.
pub fn process_run_js_polyfill() -> &'static str {
    r#"function processRun(argv, cwd, env) {
  var cp = require("child_process");
  var a = Array.isArray(argv) ? argv.map(function (x) { return String(x); }) : [];
  if (a.length < 1) return -1;
  var opts = { encoding: "utf8", stdio: ["ignore", "ignore", "ignore"] };
  if (cwd != null && cwd !== undefined) opts.cwd = String(cwd);
  if (env != null && env !== undefined && typeof env === "object") {
    var base = (typeof process !== "undefined" && process && process.env) ? process.env : {};
    var merged = {};
    for (var k in base) {
      if (Object.prototype.hasOwnProperty.call(base, k)) merged[k] = base[k];
    }
    for (var ek in env) {
      if (Object.prototype.hasOwnProperty.call(env, ek)) merged[ek] = String(env[ek]);
    }
    opts.env = merged;
  }
  var r = cp.spawnSync(a[0], a.slice(1), opts);
  if (!r || r.error) return -1;
  if (r.status != null && r.status !== undefined) return Number(r.status) | 0;
  if (r.signal) return 128;
  return -1;
}
if (typeof globalThis !== "undefined") {
  globalThis.processRun = processRun;
}
"#
}

/// JS polyfill for H15.02 process spawn + pipes (Node `spawnSync` deferred).
///
/// Handles are deferred until `processWait`: capture uses `spawnSync` with
/// `input`; kill runs a shell wrapper that spawns, SIGTERMs, and waits.
pub fn process_spawn_js_polyfill() -> &'static str {
    r#"(function () {
  var cp = require("child_process");
  var slots = Object.create(null);
  var nextId = 1;
  function mergeEnv(env) {
    var base = (typeof process !== "undefined" && process && process.env) ? process.env : {};
    var merged = {};
    for (var k in base) {
      if (Object.prototype.hasOwnProperty.call(base, k)) merged[k] = base[k];
    }
    if (env != null && env !== undefined && typeof env === "object") {
      for (var ek in env) {
        if (Object.prototype.hasOwnProperty.call(env, ek)) merged[ek] = String(env[ek]);
      }
    }
    return merged;
  }
  function shellQuote(s) {
    return "'" + String(s).replace(/'/g, "'\\''") + "'";
  }
  function processSpawn(argv, cwd, env) {
    var a = Array.isArray(argv) ? argv.map(function (x) { return String(x); }) : [];
    if (a.length < 1) return -1;
    var id = nextId++;
    slots[id] = {
      argv: a,
      cwd: cwd,
      env: env,
      stdin: null,
      stdinSet: false,
      kill: false,
      waited: false,
      exitCode: -1,
      stdout: "",
      stderr: ""
    };
    return id;
  }
  function processStdinWrite(h, text) {
    var s = slots[h | 0];
    if (!s || s.waited || s.stdinSet) return -1;
    s.stdin = text == null || text === undefined ? "" : String(text);
    s.stdinSet = true;
    return 0;
  }
  function processKill(h) {
    var s = slots[h | 0];
    if (!s || s.waited) return -1;
    s.kill = true;
    return 0;
  }
  function processWait(h) {
    var s = slots[h | 0];
    if (!s) return -1;
    if (s.waited) return s.exitCode;
    var opts = { encoding: "utf8", stdio: ["pipe", "pipe", "pipe"] };
    if (s.cwd != null && s.cwd !== undefined) opts.cwd = String(s.cwd);
    if (s.env != null && s.env !== undefined && typeof s.env === "object") {
      opts.env = mergeEnv(s.env);
    }
    var r;
    if (s.kill) {
      var parts = [];
      for (var i = 0; i < s.argv.length; i++) parts.push(shellQuote(s.argv[i]));
      var cmd = parts.join(" ") + " & pid=$!; kill -TERM $pid; wait $pid; exit $?";
      r = cp.spawnSync("/bin/sh", ["-c", cmd], opts);
    } else {
      opts.input = s.stdinSet ? s.stdin : "";
      r = cp.spawnSync(s.argv[0], s.argv.slice(1), opts);
    }
    if (!r || r.error) {
      s.exitCode = -1;
      s.stdout = "";
      s.stderr = "";
    } else {
      s.stdout = r.stdout == null ? "" : String(r.stdout);
      s.stderr = r.stderr == null ? "" : String(r.stderr);
      if (r.status != null && r.status !== undefined) s.exitCode = Number(r.status) | 0;
      else if (r.signal) s.exitCode = 128;
      else s.exitCode = -1;
    }
    s.waited = true;
    return s.exitCode;
  }
  function processStdout(h) {
    var s = slots[h | 0];
    if (!s || !s.waited) return "";
    return s.stdout == null ? "" : String(s.stdout);
  }
  function processStderr(h) {
    var s = slots[h | 0];
    if (!s || !s.waited) return "";
    return s.stderr == null ? "" : String(s.stderr);
  }
  function processClose(h) {
    var s = slots[h | 0];
    if (!s) return -1;
    delete slots[h | 0];
    return 0;
  }
  if (typeof globalThis !== "undefined") {
    globalThis.processSpawn = processSpawn;
    globalThis.processStdinWrite = processStdinWrite;
    globalThis.processWait = processWait;
    globalThis.processStdout = processStdout;
    globalThis.processStderr = processStderr;
    globalThis.processKill = processKill;
    globalThis.processClose = processClose;
  }
})();
"#
}

/// JS polyfill for `spawnWorker` (C01.01), `joinWorker` (C01.02),
/// and `terminateWorker` (C01.03). Optional second arg is a channel handle
/// (C02.04): the worker fn is called with that handle; queued values move
/// into the isolate and worker `channelSend` drains back on join.
///
/// Node `worker_threads`: eval bootstrap + SharedArrayBuffer handshake.
/// `unref` so the parent can exit without join. Join waits via `Atomics.wait`
/// and returns 0 on success or a negative code on invalid/already-joined handle.
/// Worker throw is stored as status 2 and surfaced as join result 1.
/// Terminate force-stops the worker thread; slot is not shared with the parent heap.
pub fn spawn_worker_js_polyfill() -> &'static str {
    r#"(function () {
  var nextId = 1;
  var slots = Object.create(null);
  function spawnWorker(entry, ch) {
    var fnSrc;
    if (typeof entry === "function") {
      fnSrc = "(" + Function.prototype.toString.call(entry) + ")";
    } else if (typeof entry === "string" && entry.length > 0) {
      fnSrc = "(function(){})";
    } else {
      return -1;
    }
    var sab = new SharedArrayBuffer(4);
    var ia = new Int32Array(sab);
    var channelId = 0;
    var chanInit = [];
    var port2 = null;
    var drainPort = null;
    if (arguments.length >= 2 && typeof ch === "number" && ch > 0) {
      var chans = globalThis.__draconicChannels;
      if (!chans || !chans.slots[ch]) return -1;
      try {
        var wt0 = require("worker_threads");
        var mc = new wt0.MessageChannel();
        drainPort = mc.port1;
        port2 = mc.port2;
      } catch (e0) {
        return -1;
      }
      channelId = ch;
      chanInit = chans.slots[ch].slice();
      chans.slots[ch].length = 0;
    }
    var chanBoot = "";
    if (channelId) {
      chanBoot =
        "var __q = (workerData.initial || []).slice();\n" +
        "var __port = workerData.port;\n" +
        "var __chid = workerData.channelId;\n" +
        "function channelSend(c, v) { if (c !== __chid) return -1; if (__port && typeof __port.postMessage === 'function') __port.postMessage(v); return 0; }\n" +
        "function channelRecv(c) { if (c !== __chid) return undefined; if (!__q.length) return undefined; return __q.shift(); }\n";
    }
    var callSrc = channelId ? (fnSrc + "(workerData.channelId);\n") : (fnSrc + "();\n");
    var bootstrap =
      "const { workerData } = require('worker_threads');\n" +
      chanBoot +
      "try {\n" +
      callSrc +
      "  Atomics.store(workerData.ia, 0, 1);\n" +
      "} catch (e) {\n" +
      "  Atomics.store(workerData.ia, 0, 2);\n" +
      "}\n" +
      "Atomics.notify(workerData.ia, 0, 1);\n";
    try {
      var wt = require("worker_threads");
      var opts = { eval: true, workerData: { ia: ia, channelId: channelId, initial: chanInit, port: port2 } };
      if (port2) opts.transferList = [port2];
      var w = new wt.Worker(bootstrap, opts);
      if (typeof w.unref === "function") w.unref();
      var id = nextId++;
      slots[id] = { ia: ia, worker: w, joined: false, drainPort: drainPort, channelId: channelId };
      return id;
    } catch (e) {
      return -1;
    }
  }
  function joinWorker(h) {
    var rec = slots[h];
    if (!rec || rec.joined) return -1;
    rec.joined = true;
    if (rec.worker && typeof rec.worker.ref === "function") rec.worker.ref();
    Atomics.wait(rec.ia, 0, 0);
    var st = Atomics.load(rec.ia, 0);
    if (rec.drainPort) {
      try {
        var rmp = require("worker_threads").receiveMessageOnPort;
        var chans2 = globalThis.__draconicChannels;
        var msg;
        while (typeof rmp === "function") {
          msg = rmp(rec.drainPort);
          if (!msg) break;
          if (chans2 && rec.channelId && chans2.slots[rec.channelId]) {
            chans2.slots[rec.channelId].push(msg.message);
          }
        }
      } catch (e1) {}
    }
    if (rec.worker && typeof rec.worker.unref === "function") rec.worker.unref();
    delete slots[h];
    if (st === 2) return 1;
    return 0;
  }
  function terminateWorker(h) {
    var rec = slots[h];
    if (!rec || rec.joined) return -1;
    rec.joined = true;
    try {
      if (rec.worker && typeof rec.worker.terminate === "function") rec.worker.terminate();
    } catch (e) {}
    delete slots[h];
    return 0;
  }
  if (typeof globalThis !== "undefined") {
    globalThis.spawnWorker = spawnWorker;
    globalThis.joinWorker = joinWorker;
    globalThis.terminateWorker = terminateWorker;
  }
})();
"#
}

/// JS polyfill for `makeChannel` / `channelSend` / `channelRecv` (C02.01–C02.03).
///
/// Same-isolate FIFO of numbers, strings, bools, and structured-cloned
/// plain objects. `makeChannel()` / `makeChannel(n<=0)` is unbounded;
/// `makeChannel(n)` with n > 0 bounds the buffer. Send on a full bounded
/// channel returns -2 without enqueueing. Shared object refs (cycles /
/// diamonds) and non-plain values are rejected. Send returns 0 on success
/// or -1 on invalid handle / reject.
pub fn channel_js_polyfill() -> &'static str {
    r#"(function () {
  var nextId = 1;
  var slots = Object.create(null);
  var caps = Object.create(null);
  var FAIL = {};
  function clonePlain(v, seen) {
    var t = typeof v;
    if (t === "number" || t === "string" || t === "boolean") return v;
    if (v === null || t !== "object") return FAIL;
    if (typeof Array.isArray === "function" && Array.isArray(v)) return FAIL;
    var i;
    for (i = 0; i < seen.length; i++) {
      if (seen[i] === v) return FAIL;
    }
    seen.push(v);
    var out = {};
    var keys = Object.keys(v);
    for (i = 0; i < keys.length; i++) {
      var c = clonePlain(v[keys[i]], seen);
      if (c === FAIL) return FAIL;
      out[keys[i]] = c;
    }
    return out;
  }
  function makeChannel(cap) {
    var id = nextId++;
    slots[id] = [];
    caps[id] = (typeof cap === "number" && cap > 0) ? cap : 0;
    return id;
  }
  function channelSend(ch, v) {
    var q = slots[ch];
    if (!q) return -1;
    var cap = caps[ch];
    if (cap > 0 && q.length >= cap) return -2;
    var t = typeof v;
    if (t === "number" || t === "string" || t === "boolean") {
      q.push(v);
      return 0;
    }
    var cloned = clonePlain(v, []);
    if (cloned === FAIL) return -1;
    q.push(cloned);
    return 0;
  }
  function channelRecv(ch) {
    var q = slots[ch];
    if (!q || q.length === 0) return undefined;
    return q.shift();
  }
  if (typeof globalThis !== "undefined") {
    globalThis.makeChannel = makeChannel;
    globalThis.channelSend = channelSend;
    globalThis.channelRecv = channelRecv;
    globalThis.__draconicChannels = { slots: slots, caps: caps };
  }
})();
"#
}

/// JS polyfill for `makeCancelToken` / `cancelTokenAbort` / `cancelTokenAborted` /
/// `cancelTokenLink` (C05.01) and `withTimeout` / `clearWithTimeout` (C05.02).
///
/// Abort is sticky and idempotent. `cancelTokenLink(child, parent)` makes a
/// parent abort propagate to the child (immediately if the parent is already
/// aborted). Invalid handles return -1.
///
/// `withTimeout(ms)` returns a token that auto-aborts after ms (H05 timer).
/// `clearWithTimeout(token)` cancels the pending timer (work won; settle
/// cleanly). Invalid handles return -1.
pub fn cancel_token_js_polyfill() -> &'static str {
    r#"(function () {
  var nextId = 1;
  var slots = Object.create(null);
  function makeCancelToken() {
    var id = nextId++;
    slots[id] = { aborted: 0, links: [], timer: null };
    return id;
  }
  function cancelTokenAbort(t) {
    var s = slots[t];
    if (!s) return -1;
    if (s.aborted) return 0;
    s.aborted = 1;
    if (s.timer != null) {
      clearTimeout(s.timer);
      s.timer = null;
    }
    var kids = s.links;
    var i;
    for (i = 0; i < kids.length; i++) {
      cancelTokenAbort(kids[i]);
    }
    return 0;
  }
  function cancelTokenAborted(t) {
    var s = slots[t];
    if (!s) return -1;
    return s.aborted ? 1 : 0;
  }
  function cancelTokenLink(child, parent) {
    var c = slots[child];
    var p = slots[parent];
    if (!c || !p) return -1;
    if (p.aborted) {
      cancelTokenAbort(child);
      return 0;
    }
    p.links.push(child);
    return 0;
  }
  function withTimeout(ms) {
    var tok = makeCancelToken();
    var s = slots[tok];
    s.timer = setTimeout(function () {
      s.timer = null;
      cancelTokenAbort(tok);
    }, ms);
    return tok;
  }
  function clearWithTimeout(t) {
    var s = slots[t];
    if (!s) return -1;
    if (s.timer != null) {
      clearTimeout(s.timer);
      s.timer = null;
    }
    return 0;
  }
  if (typeof globalThis !== "undefined") {
    globalThis.makeCancelToken = makeCancelToken;
    globalThis.cancelTokenAbort = cancelTokenAbort;
    globalThis.cancelTokenAborted = cancelTokenAborted;
    globalThis.cancelTokenLink = cancelTokenLink;
    globalThis.withTimeout = withTimeout;
    globalThis.clearWithTimeout = clearWithTimeout;
  }
})();
"#
}

/// JS polyfill for `nowMs()` (H05.01).
///
/// Node/browser wall clock via `Date.now()` (ms since Unix epoch).
pub fn now_ms_js_polyfill() -> &'static str {
    r#"function nowMs() {
  return Date.now();
}
if (typeof globalThis !== "undefined") {
  globalThis.nowMs = nowMs;
}
"#
}

/// JS polyfill for `monotonicMs()` (H05.02).
///
/// Prefer `performance.now()`; else Node `process.hrtime`; last resort wall clock.
pub fn monotonic_ms_js_polyfill() -> &'static str {
    r#"function monotonicMs() {
  if (typeof performance !== "undefined" && performance && typeof performance.now === "function") {
    return performance.now();
  }
  if (typeof process !== "undefined" && process && typeof process.hrtime === "function") {
    var t = process.hrtime();
    return t[0] * 1e3 + t[1] / 1e6;
  }
  return Date.now();
}
if (typeof globalThis !== "undefined") {
  globalThis.monotonicMs = monotonicMs;
}
"#
}

/// JS polyfill for `setTimeout` / `clearTimeout` (H05.03).
///
/// Bridges to the host event loop (Node/browser). Delay coerced with ToNumber;
/// missing/NaN/negative → 0.
pub fn set_timeout_js_polyfill() -> &'static str {
    r#"(function () {
  var _st = globalThis.setTimeout.bind(globalThis);
  var _ct = globalThis.clearTimeout.bind(globalThis);
  function setTimeout(fn, delay) {
    var d = delay == null ? 0 : +delay;
    if (!(d > 0)) d = 0;
    return _st(fn, d);
  }
  function clearTimeout(id) {
    return _ct(id);
  }
  globalThis.setTimeout = setTimeout;
  globalThis.clearTimeout = clearTimeout;
})();
"#
}

/// JS polyfill for `setInterval` / `clearInterval` (H05.04).
///
/// Bridges to the host event loop (Node/browser). Interval coerced with
/// ToNumber; missing/NaN/negative → 0.
pub fn set_interval_js_polyfill() -> &'static str {
    r#"(function () {
  var _si = globalThis.setInterval.bind(globalThis);
  var _ci = globalThis.clearInterval.bind(globalThis);
  function setInterval(fn, delay) {
    var d = delay == null ? 0 : +delay;
    if (!(d > 0)) d = 0;
    return _si(fn, d);
  }
  function clearInterval(id) {
    return _ci(id);
  }
  globalThis.setInterval = setInterval;
  globalThis.clearInterval = clearInterval;
})();
"#
}

/// JS polyfill for `stdoutWrite` (H02.01).
///
/// Node bridge via `process.stdout.write`. Accepts string (UTF-8) or `Uint8Array`
/// (raw bytes). No automatic newline — include `\n` in the string when needed.
pub fn stdout_write_js_polyfill() -> &'static str {
    r#"function stdoutWrite(data) {
  if (typeof process === "undefined" || !process || !process.stdout || typeof process.stdout.write !== "function") {
    return;
  }
  if (data == null) return;
  if (typeof data === "string") {
    process.stdout.write(data);
    return;
  }
  if (typeof Uint8Array !== "undefined" && data instanceof Uint8Array) {
    process.stdout.write(Buffer.from(data.buffer, data.byteOffset, data.byteLength));
    return;
  }
  if (typeof Buffer !== "undefined" && Buffer.isBuffer && Buffer.isBuffer(data)) {
    process.stdout.write(data);
    return;
  }
  process.stdout.write(String(data));
}
if (typeof globalThis !== "undefined") globalThis.stdoutWrite = stdoutWrite;
"#
}

/// JS polyfill for `stderrWrite` (H02.02).
///
/// Node bridge via `process.stderr.write`. Accepts string (UTF-8) or `Uint8Array`
/// (raw bytes). No automatic newline — include `\n` in the string when needed.
pub fn stderr_write_js_polyfill() -> &'static str {
    r#"function stderrWrite(data) {
  if (typeof process === "undefined" || !process || !process.stderr || typeof process.stderr.write !== "function") {
    return;
  }
  if (data == null) return;
  if (typeof data === "string") {
    process.stderr.write(data);
    return;
  }
  if (typeof Uint8Array !== "undefined" && data instanceof Uint8Array) {
    process.stderr.write(Buffer.from(data.buffer, data.byteOffset, data.byteLength));
    return;
  }
  if (typeof Buffer !== "undefined" && Buffer.isBuffer && Buffer.isBuffer(data)) {
    process.stderr.write(data);
    return;
  }
  process.stderr.write(String(data));
}
if (typeof globalThis !== "undefined") globalThis.stderrWrite = stderrWrite;
"#
}

/// JS polyfill for `stdinReadLine` / `stdinReadBytes` (H02.03).
///
/// Node bridge via `fs.readSync(0, …)` (blocking). Line strips trailing `\n` /
/// `\r\n`; EOF with no data → `null`. Bytes return a `Uint8Array` of actual
/// length (empty at EOF).
pub fn stdin_read_js_polyfill() -> &'static str {
    r#"function stdinReadLine() {
  var fs = require("fs");
  var chunks = [];
  var buf = Buffer.alloc(1);
  for (;;) {
    var n;
    try {
      n = fs.readSync(0, buf, 0, 1, null);
    } catch (e) {
      if (e && (e.code === "EOF" || e.code === "EAGAIN")) n = 0;
      else throw e;
    }
    if (n === 0) {
      if (chunks.length === 0) return null;
      break;
    }
    var c = buf[0];
    if (c === 10) break;
    chunks.push(c);
  }
  if (chunks.length > 0 && chunks[chunks.length - 1] === 13) chunks.pop();
  return Buffer.from(chunks).toString("utf8");
}
function stdinReadBytes(max) {
  var fs = require("fs");
  var m = Number(max);
  if (!(m > 0) || !isFinite(m)) return new Uint8Array(0);
  m = m >>> 0;
  if (m === 0) return new Uint8Array(0);
  var buf = Buffer.alloc(m);
  var n;
  try {
    n = fs.readSync(0, buf, 0, m, null);
  } catch (e) {
    if (e && (e.code === "EOF" || e.code === "EAGAIN")) n = 0;
    else throw e;
  }
  if (!n) return new Uint8Array(0);
  return new Uint8Array(buf.buffer, buf.byteOffset, n);
}
if (typeof globalThis !== "undefined") {
  globalThis.stdinReadLine = stdinReadLine;
  globalThis.stdinReadBytes = stdinReadBytes;
}
"#
}

/// JS polyfill for path helpers (H03.01–H03.03).
///
/// Pure string ops except `pathResolve` (uses cwd). POSIX-style `/` output;
/// input accepts `/` and `\`. Empty normalize/join → `"."`. Matches Node
/// `path.posix` for `/` inputs; `pathResolve` matches Node `path.resolve`.
pub fn path_js_polyfill() -> &'static str {
    r#"function pathNormalize(path) {
  var src = path == null ? "" : String(path);
  if (src.length === 0) return ".";
  function isSep(c) { return c === "/" || c === "\\"; }
  var isAbs = isSep(src.charAt(0));
  var trailing = isSep(src.charAt(src.length - 1));
  var segs = [];
  var i = 0;
  while (i < src.length) {
    while (i < src.length && isSep(src.charAt(i))) i++;
    if (i >= src.length) break;
    var start = i;
    while (i < src.length && !isSep(src.charAt(i))) i++;
    var seg = src.slice(start, i);
    if (seg === ".") continue;
    if (seg === "..") {
      if (segs.length > 0 && segs[segs.length - 1] !== "..") {
        segs.pop();
        continue;
      }
      if (!isAbs) segs.push("..");
      continue;
    }
    segs.push(seg);
  }
  var out = "";
  if (isAbs) out = "/";
  if (segs.length === 0) {
    if (!isAbs) out = ".";
  } else {
    out += segs.join("/");
    if (trailing) out += "/";
  }
  return out;
}
function pathJoin() {
  var parts = [];
  for (var i = 0; i < arguments.length; i++) {
    var p = arguments[i] == null ? "" : String(arguments[i]);
    if (p.length > 0) parts.push(p);
  }
  if (parts.length === 0) return ".";
  return pathNormalize(parts.join("/"));
}
function pathDirname(path) {
  var src = path == null ? "" : String(path);
  function isSep(c) { return c === "/" || c === "\\"; }
  if (src.length === 0) return ".";
  var end = src.length;
  while (end > 0 && isSep(src.charAt(end - 1))) end--;
  if (end === 0) return "/";
  var i = end;
  while (i > 0 && !isSep(src.charAt(i - 1))) i--;
  if (i === 0) return ".";
  var dend = i;
  while (dend > 0 && isSep(src.charAt(dend - 1))) dend--;
  if (dend === 0) return "/";
  return src.slice(0, dend).replace(/\\/g, "/");
}
function pathBasename(path) {
  var src = path == null ? "" : String(path);
  function isSep(c) { return c === "/" || c === "\\"; }
  if (src.length === 0) return "";
  var end = src.length;
  while (end > 0 && isSep(src.charAt(end - 1))) end--;
  if (end === 0) return "";
  var i = end;
  while (i > 0 && !isSep(src.charAt(i - 1))) i--;
  return src.slice(i, end).replace(/\\/g, "/");
}
function pathExtname(path) {
  var src = path == null ? "" : String(path);
  function isSep(c) { return c === "/" || c === "\\"; }
  if (src.length === 0) return "";
  var startDot = -1;
  var startPart = 0;
  var end = -1;
  var matchedSlash = true;
  var preDotState = 0;
  for (var i = src.length - 1; i >= 0; --i) {
    var c = src.charAt(i);
    if (isSep(c)) {
      if (!matchedSlash) {
        startPart = i + 1;
        break;
      }
      continue;
    }
    if (end === -1) {
      matchedSlash = false;
      end = i + 1;
    }
    if (c === ".") {
      if (startDot === -1) startDot = i;
      else if (preDotState !== 1) preDotState = 1;
    } else if (startDot !== -1) {
      preDotState = -1;
    }
  }
  if (startDot === -1 || end === -1 ||
      preDotState === 0 ||
      (preDotState === 1 && startDot === end - 1 && startDot === startPart + 1)) {
    return "";
  }
  return src.slice(startDot, end);
}
function pathIsAbsolute(path) {
  var src = path == null ? "" : String(path);
  if (src.length === 0) return false;
  var c = src.charAt(0);
  return c === "/" || c === "\\";
}
function pathResolve() {
  var resolvedPath = "";
  var resolvedAbsolute = false;
  for (var i = arguments.length - 1; i >= -1 && !resolvedAbsolute; i--) {
    var path;
    if (i >= 0) {
      path = arguments[i] == null ? "" : String(arguments[i]);
    } else if (typeof process !== "undefined" && process && typeof process.cwd === "function") {
      path = process.cwd();
    } else if (typeof cwd === "function") {
      path = String(cwd());
    } else {
      path = "/";
    }
    if (path.length === 0) continue;
    resolvedPath = path + "/" + resolvedPath;
    resolvedAbsolute = path.charAt(0) === "/" || path.charAt(0) === "\\";
  }
  resolvedPath = pathNormalize(resolvedPath);
  if (resolvedAbsolute) {
    if (resolvedPath.length > 1 && resolvedPath.charAt(resolvedPath.length - 1) === "/") {
      resolvedPath = resolvedPath.slice(0, -1);
    }
    if (resolvedPath.length === 0) return "/";
    if (resolvedPath.charAt(0) !== "/" && resolvedPath.charAt(0) !== "\\") {
      resolvedPath = "/" + resolvedPath;
    }
    return resolvedPath.replace(/\\/g, "/");
  }
  return resolvedPath.length > 0 ? resolvedPath : ".";
}
if (typeof globalThis !== "undefined") {
  globalThis.pathJoin = pathJoin;
  globalThis.pathNormalize = pathNormalize;
  globalThis.pathDirname = pathDirname;
  globalThis.pathBasename = pathBasename;
  globalThis.pathExtname = pathExtname;
  globalThis.pathIsAbsolute = pathIsAbsolute;
  globalThis.pathResolve = pathResolve;
}
"#
}

/// JS polyfill for host file APIs (H04.01–H04.05).
///
/// Node `fs` bridge. Missing path → throw `Error` with `.code === "ENOENT"`
/// and `.name === "HostError"`. `exists` returns boolean (no throw).
pub fn fs_read_js_polyfill() -> &'static str {
    r#"function __draconic_host_fs_err(code, path, cause) {
  var msg = code + ": " + (cause && cause.message ? cause.message : "file error");
  if (path != null) msg += ", open '" + String(path) + "'";
  var e = new Error(msg);
  e.name = "HostError";
  e.code = code;
  if (cause && cause.code) e.code = String(cause.code);
  throw e;
}
function __draconic_host_fs_catch(p, err) {
  if (err && (err.code === "ENOENT" || err.code === "ENOTDIR")) {
    __draconic_host_fs_err("ENOENT", p, err);
  }
  if (err && err.code === "EEXIST") {
    __draconic_host_fs_err("EEXIST", p, err);
  }
  if (err && (err.code === "EACCES" || err.code === "EPERM")) {
    __draconic_host_fs_err("EPERM", p, err);
  }
  __draconic_host_fs_err("EIO", p, err);
}
function readFileText(path) {
  var p = String(path);
  var fs = require("fs");
  try {
    return fs.readFileSync(p, "utf8");
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function readFileBytes(path) {
  var p = String(path);
  var fs = require("fs");
  try {
    var buf = fs.readFileSync(p);
    return new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength);
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function writeFileText(path, text) {
  var p = String(path);
  var fs = require("fs");
  try {
    fs.writeFileSync(p, text == null ? "" : String(text), "utf8");
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function appendFileText(path, text) {
  var p = String(path);
  var fs = require("fs");
  try {
    fs.appendFileSync(p, text == null ? "" : String(text), "utf8");
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function writeFileBytes(path, data) {
  var p = String(path);
  var fs = require("fs");
  try {
    var buf = Buffer.from(data == null ? [] : data);
    fs.writeFileSync(p, buf);
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function appendFileBytes(path, data) {
  var p = String(path);
  var fs = require("fs");
  try {
    var buf = Buffer.from(data == null ? [] : data);
    fs.appendFileSync(p, buf);
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function exists(path) {
  var p = String(path);
  if (!p) return false;
  var fs = require("fs");
  try {
    fs.accessSync(p);
    return true;
  } catch (err) {
    return false;
  }
}
function stat(path) {
  var p = String(path);
  var fs = require("fs");
  try {
    var st = fs.statSync(p);
    return {
      size: st.size,
      isFile: st.isFile(),
      isDir: st.isDirectory(),
      mtime: st.mtimeMs != null ? st.mtimeMs : (+st.mtime)
    };
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function mkdir(path) {
  var p = String(path);
  var fs = require("fs");
  try {
    fs.mkdirSync(p);
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function mkdirAll(path) {
  var p = String(path);
  var fs = require("fs");
  try {
    fs.mkdirSync(p, { recursive: true });
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function readdir(path) {
  var p = String(path);
  var fs = require("fs");
  try {
    return fs.readdirSync(p);
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function rmdir(path) {
  var p = String(path);
  var fs = require("fs");
  try {
    fs.rmdirSync(p);
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function removeFile(path) {
  var p = String(path);
  var fs = require("fs");
  try {
    fs.unlinkSync(p);
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function renameFile(from, to) {
  var a = String(from);
  var b = String(to);
  var fs = require("fs");
  try {
    fs.renameSync(a, b);
  } catch (err) {
    __draconic_host_fs_catch(a, err);
  }
}
function copyFile(from, to) {
  var a = String(from);
  var b = String(to);
  var fs = require("fs");
  try {
    fs.copyFileSync(a, b);
  } catch (err) {
    __draconic_host_fs_catch(a, err);
  }
}
if (typeof globalThis !== "undefined") {
  globalThis.readFileText = readFileText;
  globalThis.readFileBytes = readFileBytes;
  globalThis.writeFileText = writeFileText;
  globalThis.appendFileText = appendFileText;
  globalThis.writeFileBytes = writeFileBytes;
  globalThis.appendFileBytes = appendFileBytes;
  globalThis.exists = exists;
  globalThis.stat = stat;
  globalThis.mkdir = mkdir;
  globalThis.mkdirAll = mkdirAll;
  globalThis.readdir = readdir;
  globalThis.rmdir = rmdir;
  globalThis.removeFile = removeFile;
  globalThis.renameFile = renameFile;
  globalThis.copyFile = copyFile;
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declare_and_call_shapes() {
        assert_eq!(HELLO.declare(), "declare void @draconic_rt_hello()");
        assert_eq!(HELLO.call(""), "call void @draconic_rt_hello()");
        assert_eq!(
            PRINT_I64.call("i64 %v"),
            "call void @draconic_rt_print_i64(i64 %v)"
        );
        assert_eq!(
            PROMISE_THEN.call_to("%t", "ptr %p, ptr null, ptr null, ptr null, ptr null"),
            "%t = call ptr @draconic_rt_promise_then(ptr %p, ptr null, ptr null, ptr null, ptr null)"
        );
    }

    #[test]
    fn symbol_aliases_match_abi_fn() {
        assert_eq!(HELLO_SYMBOL, HELLO.symbol);
        assert_eq!(PROMISE_AWAIT_SYMBOL, PROMISE_AWAIT.symbol);
        assert!(MINIMAL_STD_AND_GC_SYMBOLS.contains(&HELLO_SYMBOL));
        assert!(PROMISE_SYMBOLS.contains(&PROMISE_NEW_SYMBOL));
    }

    #[test]
    fn emitter_declare_sets_are_stable() {
        assert!(llvm_declares(NATIVE_INT_DECLARES).contains(PRINT_I64_SYMBOL));
        assert!(llvm_declares(ES_EVAL_DECLARES).contains(GC_INIT_SYMBOL));
        assert!(llvm_declares(ES_EXPR_DECLARES).contains(PRINT_F64_SYMBOL));
        assert!(llvm_declares(ES_PROMISE_DECLARES).contains(PROMISE_THEN_SYMBOL));
    }
}
