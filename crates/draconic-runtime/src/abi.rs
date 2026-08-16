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
pub const STRING_DATA_SYMBOL: &str = STRING_DATA.symbol;
pub const STRING_LEN_SYMBOL: &str = STRING_LEN.symbol;
pub const IS_STRING_SYMBOL: &str = IS_STRING.symbol;
pub const IS_OBJECT_SYMBOL: &str = IS_OBJECT.symbol;
pub const JOB_ENQUEUE_SYMBOL: &str = JOB_ENQUEUE.symbol;
pub const JOB_DRAIN_SYMBOL: &str = JOB_DRAIN.symbol;
pub const JOB_PENDING_SYMBOL: &str = JOB_PENDING.symbol;
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
pub const OBJECT_SET_PROTO_SYMBOL: &str = OBJECT_SET_PROTO.symbol;
pub const OBJECT_GET_PROTO_SYMBOL: &str = OBJECT_GET_PROTO.symbol;
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
    STRING_DATA_SYMBOL,
    STRING_LEN_SYMBOL,
    IS_STRING_SYMBOL,
    IS_OBJECT_SYMBOL,
];

/// Job queue ABI symbols (N06.01).
pub const JOB_QUEUE_SYMBOLS: &[&str] = &[JOB_ENQUEUE_SYMBOL, JOB_DRAIN_SYMBOL, JOB_PENDING_SYMBOL];

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
    PRINT_F64,
    PRINT_BOOL,
    PRINT_STR,
    CSTR_LEN,
    CSTR_CONCAT,
    CSTR_FROM_U64,
    CSTR_FROM_CODE_UNIT,
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
