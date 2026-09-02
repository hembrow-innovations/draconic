//! LLVM backend: IR → native (one lowerer; private adapters for supported subsets).

mod debug_info;
mod es_arrays;
mod es_param_dstr;
mod es_builtins;
mod es_call_spread;
mod es_class_expr_name;
mod es_classes;
mod es_static_private_fields;
mod es_static_private_methods;
mod es_coercion;
mod es_destructure_defaults;
mod base64;
mod hex;
mod sha256;
mod es_encoding;
mod es_logging;
mod es_testing;
mod es_eval;
mod es_exceptions;
mod es_expr;
mod es_functions;
mod es_generators;
mod es_instanceof;
mod es_legacy;
mod es_modules;
mod es_private_methods;
mod es_private_accessors;
mod es_optional_chain;
mod es_new_target;

mod es_static_blocks;
mod es_nullish;
mod es_object_destructure;
mod es_objects;

mod es_private_in;
mod es_async_methods;
mod es_promise;
mod es_proxies;
mod es_tagged_template;
mod es_to_primitive;
mod es_values;
mod es_var_for;
mod host_dns;
mod host_docs;
mod host_fs;
mod host_http;
mod host_http_server;
mod host_os;
mod host_path;
mod host_process;
mod host_subprocess;
mod host_process_async;
mod host_signals;
mod host_stdio;
mod host_tcp;
mod host_tcp_async;
mod host_udp;
mod host_ws;
mod host_ws_e2e;
mod host_http2;
mod host_time;
mod host_timers;
mod host_once;
mod host_atomics;
mod host_cancel;
mod host_workers;
mod host_worker_channels;
mod host_channels;
mod native_ints;
mod cross_compile;

pub use debug_info::SourceDebug;
pub use cross_compile::{
    compile_object_for_triple, cross_compile_matrix, host_cross_compile_pair, CrossCompilePair,
};

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use draconic_diagnostics::{codes, Diagnostic, Span};
use draconic_ir::Module;

use es_arrays::{emit_es_arrays, is_es_arrays_module};
use es_param_dstr::{emit_es_param_dstr, is_es_param_dstr_module};
use es_builtins::{emit_es_builtins, is_es_builtins_module};
use es_call_spread::{emit_es_call_spread, is_es_call_spread_module};
use es_class_expr_name::{emit_es_class_expr_name, is_es_class_expr_name_module};
use es_classes::{emit_es_classes, is_es_classes_module};
use es_static_private_fields::{
    emit_es_static_private_fields, is_es_static_private_fields_module,
};
use es_static_private_methods::{
    emit_es_static_private_methods, is_es_static_private_methods_module,
};
use es_coercion::{emit_es_coercion, is_es_coercion_module};
use es_destructure_defaults::{
    emit_es_destructure_defaults, is_es_destructure_defaults_module,
};
use es_encoding::{emit_es_encoding, is_es_encoding_module};
use es_logging::{emit_es_logging, is_es_logging_module};
use es_testing::{emit_es_testing, is_es_testing_module};
use es_eval::{emit_es_eval, is_es_eval_module};
use es_exceptions::{emit_es_exceptions, is_es_exceptions_module};
use es_expr::{emit_es_expr, is_es_expr_module};
use es_functions::{emit_es_functions, is_es_functions_module};
use es_generators::{emit_es_generators, is_es_generators_module};
use es_instanceof::{emit_es_instanceof, is_es_instanceof_module};
use es_legacy::{emit_es_legacy, is_es_legacy_module};
use es_modules::{emit_es_modules, is_es_modules_module};
use es_private_methods::{emit_es_private_methods, is_es_private_methods_module};
use es_private_accessors::{emit_es_private_accessors, is_es_private_accessors_module};
use es_optional_chain::{emit_es_optional_chain, is_es_optional_chain_module};
use es_new_target::{emit_es_new_target, is_es_new_target_module};

use es_static_blocks::{emit_es_static_blocks, is_es_static_blocks_module};
use es_nullish::{emit_es_nullish, is_es_nullish_module};
use es_object_destructure::{emit_es_object_destructure, is_es_object_destructure_module};
use es_objects::{emit_es_objects, is_es_objects_module};

use es_private_in::{emit_es_private_in, is_es_private_in_module};
use es_async_methods::{emit_es_async_methods, is_es_async_methods_module};
use es_promise::{emit_es_promise, is_es_promise_module};
use es_proxies::{emit_es_proxies, is_es_proxies_module};
use es_tagged_template::{emit_es_tagged_template, is_es_tagged_template_module};
use es_to_primitive::{emit_es_to_primitive, is_es_to_primitive_module};
use es_values::{emit_es_values, is_es_values_module};
use es_var_for::{emit_es_var_for, is_es_var_for_module};
use host_dns::{emit_host_dns, is_host_dns_module};
use host_docs::{emit_host_docs, is_host_docs_module};
use host_fs::{emit_host_fs, is_host_fs_module};
use host_http::{emit_host_http, is_host_http_module};
use host_http_server::{emit_host_http_server, is_host_http_server_module};
use host_os::{emit_host_os, is_host_os_module};
use host_path::{emit_host_path, is_host_path_module};
use host_process::{emit_host_process, is_host_process_module};
use host_subprocess::{emit_host_subprocess, is_host_subprocess_module};
use host_process_async::{emit_host_process_async, is_host_process_async_module};
use host_signals::{emit_host_signals, is_host_signal_module};
use host_stdio::{emit_host_stdio, is_host_stdio_module};
use host_tcp::{emit_host_tcp, is_host_tcp_module};
use host_tcp_async::{emit_host_tcp_async, is_host_tcp_async_module};
use host_udp::{emit_host_udp, is_host_udp_module};
use host_ws::{emit_host_ws, is_host_ws_module};
use host_ws_e2e::{emit_host_ws_e2e, is_host_ws_e2e_module};
use host_http2::{emit_host_http2, is_host_http2_module};
use host_time::{emit_host_time, is_host_time_module};
use host_timers::{emit_host_timers, is_host_timer_module};
use host_once::{emit_host_once, is_host_once_module};
use host_atomics::{emit_host_atomics, is_host_atomics_module};
use host_cancel::{emit_host_cancel, is_host_cancel_module};
use host_workers::{emit_host_workers, is_host_workers_module};
use host_worker_channels::{emit_host_worker_channels, is_host_worker_channels_module};
use host_channels::{emit_host_channels, is_host_channels_module};
use native_ints::{emit_native_ints, is_native_int_module};

/// Emit LLVM IR text for a shared IR module.
///
/// Selects a private adapter for a supported subset, otherwise returns a hard
/// diagnostic (no silent hello-stub success for arbitrary Programs):
///
/// - **Native scalars/layouts** (`i8`–`i64`, `u8`–`u64`, `f32`/`f64`, `bool`,
///   native structs/arrays/pointers) — N01–N03
/// - **Promise / async** (constructor basics through async/await and async
///   arrows) via Runtime Promise ABI — N06.03–N06.11
/// - **eval / Function** (constant-string fold via Embed) — N07.02–N07.04
/// - **ES expressions** (numeric arithmetic + comparison/equality + logical
///   `&&`/`||`/`!` + bitwise `&` `|` `^` `~` `<<` `>>` `>>>` + `**` +
///   conditional `?:` + simple/compound assignment + prefix/postfix `++`/`--` +
///   comma `,` + unary keywords `typeof`/`void`/`delete` over JS
///   numbers/BigInts/booleans/strings/undefined + Math + Number/NaN/Infinity) via Runtime prints — N08.01.01–N08.01.04.08 / N08.08.01–N08.08.06
/// - **Nullish / logical assignment** (`??` `??=` `&&=` `||=` with mixed
///   null/undefined/number/bool/string) via tagged slots — N08.01.04.09
/// - **`if` / `else`** (block or expression bodies; ToBoolean on number/boolean
///   tests) via Runtime prints — N08.02.01
/// - **`while`** (block or expression bodies; ToBoolean on number/boolean tests)
///   via Runtime prints — N08.02.02
/// - **`do` / `while`** (block or expression bodies; ToBoolean on number/boolean
///   tests) via Runtime prints — N08.02.03
/// - **`for`** (`for (init; test; update)`; `let` init; omitted clauses; block
///   bodies; ToBoolean on number/boolean tests) via Runtime prints — N08.02.04
/// - **`break` / `continue`** (unlabeled, in `while`/`do`/`for`) via Runtime
///   prints — N08.02.05
/// - **`switch` / `case` / `default`** (number discriminant; fall-through;
///   unlabeled `break`) via Runtime prints — N08.02.06
/// - **Labeled statements** + labeled `break` / `continue` via Runtime prints —
///   N08.02.07
/// - **`for-in` / `for-of`** over strings (`let`/assign binding; string concat)
///   via Runtime prints — N08.02.08
/// - **Function declaration/expression/arrow** + `return` + call (simple ident params,
///   defaults) + nested decls with free-variable capture + IIFE/named/higher-order via
///   Runtime prints — N08.03.01–N08.03.07
/// - **Object literals** + property access/assignment (string keys; nested objects;
///   number props) via Runtime GC/object ABI — N08.04.01–N08.04.02
/// - **Class declarations** (base + `extends`/`super()` + instance/static methods;
///   `super.m(…)`; `new`; prototype chain) via Runtime GC/object ABI — N08.05.01–N08.05.04
/// - **Array literals** + index access + `.length` + destructuring via Runtime
///   array ABI — N08.06.01–N08.06.06
/// - **String lit** + concat (incl. number ToString) + `.length` + index via
///   length-aware C-string ABI — N08.07.01
/// - **Untagged templates** — N08.07.02
/// - **Unicode escapes** (`\x`/`\u`/`\u{}`) + UTF-16 `.length` — N08.07.03
/// - **Tagged templates** `` tag`…` `` (quasi array + interps; method/call tags) — N08.07.04
/// - **Symbol basics** (`Symbol()` / `Symbol.for` / `Symbol.keyFor` / typeof / `===`) — N08.09.01
/// - **Symbol property keys** (computed/get/set; no string collision) — N08.09.02
/// - **Abstract equality & coercion** (`==`/`!=` mixed types; ToNumber/ToString/ToBoolean) — N08.09.03
/// - **ToPrimitive** (`valueOf`/`toString` hooks in `+` and `==`) — N08.09.04
/// - **Exceptions** (`throw` + bare `try`/`catch`; catch binding; nested; throw from fn) — N08.10.01
/// - **Linked ESM modules** (named/default/cyclic flatten; number/string observations) — N08.11
/// - **Generators** (function* + yield/yield* + return/throw + `.next()` + for-of) — N08.12.01–N08.12.08
/// - **Async generators** (`async function*` / methods + `.next().then` + `await` + `for await`) — N08.16.44
/// - **`for await` over arrays** (`let`/`const`/assign + break/continue) — N08.16.43.01
/// - **`for await` over `Symbol.asyncIterator` custom async iterables** — N08.16.43.02
/// - **Proxy basics** (`new Proxy`, empty-handler get, `get` trap) — N08.13.01
/// - **Proxy set** (empty-handler set pass-through; `set` trap; assign result) — N08.13.02
/// - **Proxy has/`in`** (empty-handler pass-through; `has` trap; plain `in`) — N08.13.03
/// - **Proxy deleteProperty/`delete`** (empty-handler pass-through; trap; plain `delete`) — N08.13.04
/// - **Proxy apply** (empty-handler call pass-through; `apply` trap; method `this`) — N08.13.05
/// - **Proxy construct** (empty-handler `new` pass-through; `construct` trap; ctor `this`) — N08.13.06
/// - **Reflect basics** + **ownKeys** + **getPrototypeOf/setPrototypeOf** + **defineProperty/getOwnPropertyDescriptor** — N08.13.07–N08.13.10
/// - **Global builtins basics** (`undefined`/`globalThis`/`Object`/`Function`/`Array`/`String`/`Boolean`) — N08.14.01
/// - **Error constructors** (`Error`/`TypeError`/…/`AggregateError`, `new`, `.name`/`.message`, throw+catch) — N08.14.02
/// - **Global functions** (`parseInt`/`parseFloat`/`isNaN`/`isFinite`) — N08.14.03
/// - **URI encode/decode** (`encodeURI`/`decodeURI`/`encodeURIComponent`/`decodeURIComponent`) — N08.14.04
/// - **JSON** (`JSON`/`JSON.parse`/`JSON.stringify` basics) — N08.14.05
/// - **Date** (`Date`/`Date.now`/`Date.UTC`/`new Date(ms)`/`.getTime`/`.valueOf`) — N08.14.06
/// - **RegExp** (`RegExp`/`new RegExp`/`.source`/`.flags`/`.test`/`.exec`) — N08.14.07
/// - **Map/Set** (`new Map`/`new Set`, `.set`/`.get`/`.has`/`.size`, `.add`/`.has`/`.size`) — N08.14.08
/// - **Legacy `with`** (Object Environment get/put; nested `with`) — N08.15
/// - **Annex B `escape`/`unescape`** — N08.16.01
/// - **Annex B `Object.prototype.__proto__`** — N08.16.02
/// - **Annex B `RegExp.prototype.compile`** — N08.16.05
/// - **Annex B `String.prototype.trimLeft`/`trimRight`** — N08.16.06
/// - **Annex B `Object.prototype` accessor legacy** (`__defineGetter__`/…) — N08.16.07
/// - **Annex B labelled function declarations** (`L: function f(){…}`) — N08.16.11
/// - **Annex B FunctionDeclarations in `if`** (`if (c) function f(){…}`) — N08.16.12
/// - **Annex B block-level function declarations** (`{ function f(){…} }`) — N08.16.13
/// - **`var` declarations** (hoist/redeclare/uninit) — N08.16.14
/// - **`var` in `for` heads** (for-in/of/classic + Annex B.3.5 init) — N08.16.15

/// - **Private accessors** (`get`/`set #x` instance+static) — N08.16.40
/// - **Empty program** — B08 Runtime hello demo only (`main` calls
///   `draconic_rt_hello`)
/// Emit LLVM IR text for a shared IR module (no DWARF).
pub fn emit_llvm_ir(module: &Module) -> Result<String, Diagnostic> {
    emit_llvm_ir_inner(module, None)
}

/// Emit LLVM IR with DWARF debug info mapping Draconic source lines (U07).
pub fn emit_llvm_ir_with_debug(
    module: &Module,
    debug: &SourceDebug,
) -> Result<String, Diagnostic> {
    emit_llvm_ir_inner(module, Some(debug))
}

fn emit_llvm_ir_inner(
    module: &Module,
    debug: Option<&SourceDebug>,
) -> Result<String, Diagnostic> {
    let ir = emit_llvm_ir_raw(module, debug)?;
    if let Some(dbg) = debug {
        Ok(debug_info::attach_debug_info(&ir, module, dbg))
    } else {
        Ok(ir)
    }
}

fn emit_llvm_ir_raw(
    module: &Module,
    debug: Option<&SourceDebug>,
) -> Result<String, Diagnostic> {
    if is_native_int_module(module) {
        return emit_native_ints(module, debug);
    }
    if is_host_process_module(module) {
        return emit_host_process(module);
    }
    if is_host_os_module(module) {
        return emit_host_os(module);
    }
    if is_host_process_async_module(module) {
        return emit_host_process_async(module);
    }
    if is_host_subprocess_module(module) {
        return emit_host_subprocess(module);
    }
    if is_host_signal_module(module) {
        return emit_host_signals(module);
    }
    if is_host_stdio_module(module) {
        return emit_host_stdio(module);
    }
    if is_host_path_module(module) {
        return emit_host_path(module);
    }
    if is_host_fs_module(module) {
        return emit_host_fs(module);
    }
    if is_host_docs_module(module) {
        return emit_host_docs(module);
    }
    if is_host_tcp_async_module(module) {
        return emit_host_tcp_async(module);
    }
    if is_host_udp_module(module) {
        return emit_host_udp(module);
    }
    if is_host_dns_module(module) {
        return emit_host_dns(module);
    }
    if is_host_ws_e2e_module(module) {
        return emit_host_ws_e2e(module);
    }
    if is_host_http2_module(module) {
        return emit_host_http2(module);
    }
    if is_host_http_server_module(module) {
        return emit_host_http_server(module);
    }
    if is_host_ws_module(module) {
        return emit_host_ws(module);
    }
    if is_host_http_module(module) {
        return emit_host_http(module);
    }
    if is_host_tcp_module(module) {
        return emit_host_tcp(module);
    }
    if is_host_time_module(module) {
        return emit_host_time(module);
    }
    if is_host_timer_module(module) {
        return emit_host_timers(module);
    }
    if is_host_atomics_module(module) {
        return emit_host_atomics(module);
    }
    if is_host_worker_channels_module(module) {
        return emit_host_worker_channels(module);
    }
    if is_host_once_module(module) {
        return emit_host_once(module);
    }
    if is_host_cancel_module(module) {
        return emit_host_cancel(module);
    }
    if is_host_workers_module(module) {
        return emit_host_workers(module);
    }
    if is_host_channels_module(module) {
        return emit_host_channels(module);
    }
    if is_es_async_methods_module(module) {
        return emit_es_async_methods(module);
    }
    if is_es_promise_module(module) {
        return emit_es_promise(module);
    }
    if is_es_eval_module(module) {
        return emit_es_eval(module);
    }
    if is_es_private_in_module(module) {
        return emit_es_private_in(module);
    }
    if is_es_proxies_module(module) {
        return emit_es_proxies(module);
    }
    if is_es_testing_module(module) {
        return emit_es_testing(module);
    }
    if is_es_logging_module(module) {
        return emit_es_logging(module);
    }
    if is_es_encoding_module(module) {
        return emit_es_encoding(module);
    }
    if is_es_new_target_module(module) {
        return emit_es_new_target(module);
    }
    if is_es_private_accessors_module(module) {
        return emit_es_private_accessors(module);
    }
    if is_es_instanceof_module(module) {
        return emit_es_instanceof(module);
    }
    if is_es_private_methods_module(module) {
        return emit_es_private_methods(module);
    }
    if is_es_generators_module(module) {
        return emit_es_generators(module);
    }
    if is_es_modules_module(module) {
        return emit_es_modules(module);
    }
    if is_es_exceptions_module(module) {
        return emit_es_exceptions(module);
    }
    if is_es_legacy_module(module) {
        return emit_es_legacy(module);
    }
    if is_es_optional_chain_module(module) {
        return emit_es_optional_chain(module);
    }
    if is_es_static_blocks_module(module) {
        return emit_es_static_blocks(module);
    }
    if is_es_nullish_module(module) {
        return emit_es_nullish(module);
    }
    if is_es_to_primitive_module(module) {
        return emit_es_to_primitive(module);
    }
    if is_es_coercion_module(module) {
        return emit_es_coercion(module);
    }
    if is_es_values_module(module) {
        return emit_es_values(module);
    }
    if is_es_call_spread_module(module) {
        return emit_es_call_spread(module);
    }
    if is_es_tagged_template_module(module) {
        return emit_es_tagged_template(module);
    }
    if is_es_param_dstr_module(module) {
        return emit_es_param_dstr(module);
    }
    if is_es_functions_module(module) {
        return emit_es_functions(module);
    }
    if is_es_var_for_module(module) {
        return emit_es_var_for(module);
    }
    if is_es_class_expr_name_module(module) {
        return emit_es_class_expr_name(module);
    }
    if is_es_static_private_fields_module(module) {
        return emit_es_static_private_fields(module);
    }
    if is_es_static_private_methods_module(module) {
        return emit_es_static_private_methods(module);
    }
    if is_es_classes_module(module) {
        return emit_es_classes(module);
    }
    if is_es_object_destructure_module(module) {
        return emit_es_object_destructure(module);
    }
    if is_es_destructure_defaults_module(module) {
        return emit_es_destructure_defaults(module);
    }
    if is_es_builtins_module(module) {
        return emit_es_builtins(module);
    }
    if is_es_objects_module(module) {
        return emit_es_objects(module);
    }
    if is_es_arrays_module(module) {
        return emit_es_arrays(module);
    }
    if is_es_expr_module(module) {
        return emit_es_expr(module);
    }
    if is_empty_program(module) {
        return Ok(emit_empty_hello());
    }
    let _ = debug;
    Err(unsupported_native_diagnostic())
}

fn is_empty_program(module: &Module) -> bool {
    module.body.is_empty()
}

fn unsupported_native_diagnostic() -> Diagnostic {
    Diagnostic::new(
        "native target: unsupported IR (no LLVM lowering for this program; \
            supported: native scalars/layouts, Promise/async subset, eval/Function fold, \
            ES expressions (arithmetic/comparison/logical/bitwise/pow/conditional/assign/compound-assign/update/comma/typeof/void/delete/nullish/logical-assign/if-else/while/do-while/for/for-in/for-of/break/continue/switch/labeled), ES function decl/expr/arrow/return/call (simple params+defaults+rest, nested+capture, IIFE/named/HOF), ES object lit + property access/assignment + method this, ES class decl (base ctor+methods), ES array lit + index/length, ES throw/try/catch, ES generators (function*/yield/next/for-of), ES Proxy basics/set/has/delete/apply/construct, ES global builtins basics + Error constructors, instanceof prototype-chain fold, linked ESM modules (named/default/namespace/cyclic), legacy with, empty hello)",
        Span::dummy(),
    )
}

/// B08 empty-program demo: link Runtime hello. Not used for non-empty unsupported IR.
fn emit_empty_hello() -> String {
    use draconic_runtime::abi::HELLO;
    format!(
        "; Draconic LLVM backend empty program (B08 hello)\n{}\n\ndefine i32 @main() {{\nentry:\n  {}\n  ret i32 0\n}}\n",
        HELLO.declare(),
        HELLO.call(""),
    )
}

/// Compile LLVM IR + Runtime C into a native executable via `clang`.
pub fn build_native_binary(llvm_ir: &str, out_bin: &Path) -> Result<(), Diagnostic> {
    build_native_binary_with_libs(llvm_ir, out_bin, &[], &[], false)
}

/// F04.01: same as [`build_native_binary`], plus extra `.a` archives on the link line.
pub fn build_native_binary_with_static_libs(
    llvm_ir: &str,
    out_bin: &Path,
    extra_static_libs: &[PathBuf],
) -> Result<(), Diagnostic> {
    build_native_binary_with_libs(llvm_ir, out_bin, extra_static_libs, &[], false)
}

/// F05.01: same as [`build_native_binary`], plus extra shared libraries on the link line.
pub fn build_native_binary_with_dynamic_libs(
    llvm_ir: &str,
    out_bin: &Path,
    extra_dynamic_libs: &[PathBuf],
) -> Result<(), Diagnostic> {
    build_native_binary_with_libs(llvm_ir, out_bin, &[], extra_dynamic_libs, false)
}

/// D05.02: same as [`build_native_binary_with_static_libs`], with optional LTO.
pub fn build_native_binary_with_lto(
    llvm_ir: &str,
    out_bin: &Path,
    extra_static_libs: &[PathBuf],
    lto: bool,
) -> Result<(), Diagnostic> {
    build_native_binary_with_libs(llvm_ir, out_bin, extra_static_libs, &[], lto)
}

fn build_native_binary_with_libs(
    llvm_ir: &str,
    out_bin: &Path,
    extra_static_libs: &[PathBuf],
    extra_dynamic_libs: &[PathBuf],
    lto: bool,
) -> Result<(), Diagnostic> {
    let clang = find_clang().ok_or_else(|| {
        Diagnostic::new(
            "clang not found (set CLANG or install a C toolchain)",
            Span::dummy(),
        )
    })?;

    let work = work_dir("draconic-llvm-build")?;
    let ll_path = work.join("program.ll");
    std::fs::write(&ll_path, llvm_ir)
        .map_err(|e| Diagnostic::new(format!("write LLVM IR failed: {e}"), Span::dummy()))?;

    let rt_lib = draconic_runtime::build_runtime_static_lib_with_lto(&work, lto).map_err(|e| {
        Diagnostic::new(
            format!("build runtime static lib failed: {e}"),
            Span::dummy(),
        )
    })?;

    if let Some(parent) = out_bin.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            Diagnostic::new(format!("create output dir failed: {e}"), Span::dummy())
        })?;
    }

    let want_debug = llvm_ir.contains("!llvm.dbg.cu");

    // Object step first so DWARF from IR metadata is materialized (U07). Direct
    // `clang file.ll -o bin` drops debug on Apple ld without a retained .o.
    let obj_path = work.join("program.o");
    let mut cc_obj = Command::new(&clang);
    cc_obj
        .arg("-c")
        .arg(&ll_path)
        .arg("-o")
        .arg(&obj_path)
        .arg("-Wno-override-module");
    if lto {
        cc_obj.arg("-flto").arg("-Os");
    }
    if want_debug {
        cc_obj.arg("-g");
    }
    let output = cc_obj
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Diagnostic::new(format!("spawn clang -c failed: {e}"), Span::dummy()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Diagnostic::new(
            format!("clang -c failed: {stderr}"),
            Span::dummy(),
        ));
    }

    let mut static_libs: Vec<&Path> = Vec::new();
    let mut dynamic_libs: Vec<&Path> = extra_dynamic_libs.iter().map(|p| p.as_path()).collect();
    for lib in extra_static_libs {
        if is_shared_lib(lib) {
            dynamic_libs.push(lib);
        } else {
            static_libs.push(lib);
        }
    }

    for lib in &static_libs {
        if !lib.is_file() {
            return Err(Diagnostic::new(
                format!("static lib not found: {}", lib.display()),
                Span::dummy(),
            ));
        }
    }
    for lib in &dynamic_libs {
        if !lib.is_file() {
            return Err(
                Diagnostic::new(
                    format!("dynamic lib not found: {}", lib.display()),
                    Span::dummy(),
                )
                .with_code(codes::MISSING_DYNAMIC_LIB),
            );
        }
    }

    let mut cc_link = Command::new(&clang);
    cc_link.arg(&obj_path);
    for lib in &static_libs {
        cc_link.arg(lib);
    }
    for lib in &dynamic_libs {
        cc_link.arg(lib);
        if let Some(parent) = lib.parent() {
            let parent = if parent.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                parent.to_path_buf()
            };
            let rpath = if parent.is_absolute() {
                parent
            } else {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(&parent)
            };
            cc_link.arg(format!("-Wl,-rpath,{}", rpath.display()));
        }
    }
    cc_link.arg(&rt_lib).arg("-o").arg(out_bin);
    if lto {
        cc_link.arg("-flto").arg("-Os");
        if cfg!(target_os = "macos") {
            cc_link.arg("-Wl,-dead_strip");
        } else {
            cc_link.arg("-Wl,--gc-sections");
        }
    }
    if want_debug {
        cc_link.arg("-g");
    }
    draconic_runtime::apply_runtime_link_flags(&mut cc_link);
    let output = cc_link
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Diagnostic::new(format!("spawn clang link failed: {e}"), Span::dummy()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Diagnostic::new(
            format!("clang link failed: {stderr}"),
            Span::dummy(),
        ));
    }

    // macOS: DWARF lives in a .dSYM companion; generate it when we emitted debug.
    if want_debug && cfg!(target_os = "macos") {
        let _ = Command::new("dsymutil")
            .arg(out_bin)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    Ok(())
}

pub(crate) fn find_clang() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CLANG") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    for candidate in [
        "clang",
        "/usr/bin/clang",
        "/opt/homebrew/opt/llvm@22/bin/clang",
        "/opt/homebrew/opt/llvm/bin/clang",
    ] {
        let ok = Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return Some(PathBuf::from(candidate));
        }
    }
    None
}

fn find_ar() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("AR") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    for candidate in ["ar", "/usr/bin/ar", "llvm-ar", "/opt/homebrew/opt/llvm/bin/llvm-ar"] {
        let ok = Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or_else(|_| {
                Command::new(candidate)
                    .arg("-V")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            });
        if ok {
            return Some(PathBuf::from(candidate));
        }
        let probe = Command::new(candidate)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if probe.is_ok() {
            return Some(PathBuf::from(candidate));
        }
    }
    None
}

/// Compile one C file into a static archive (`.a`). Used to feed extra libs to
/// [`build_native_binary_with_static_libs`] (F04.01).
pub fn build_c_static_lib(c_src: &Path, archive: &Path) -> Result<(), Diagnostic> {
    let clang = find_clang().ok_or_else(|| {
        Diagnostic::new(
            "clang not found (set CLANG or install a C toolchain)",
            Span::dummy(),
        )
    })?;
    let ar = find_ar().ok_or_else(|| {
        Diagnostic::new(
            "ar not found (set AR or install a C toolchain)",
            Span::dummy(),
        )
    })?;
    if !c_src.is_file() {
        return Err(Diagnostic::new(
            format!("C source not found: {}", c_src.display()),
            Span::dummy(),
        ));
    }
    if let Some(parent) = archive.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Diagnostic::new(format!("create archive dir failed: {e}"), Span::dummy())
            })?;
        }
    }
    let work = work_dir("draconic-c-static")?;
    let obj = work.join("lib.o");
    let compile = Command::new(&clang)
        .arg("-c")
        .arg(c_src)
        .arg("-o")
        .arg(&obj)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Diagnostic::new(format!("spawn clang -c failed: {e}"), Span::dummy()))?;
    if !compile.status.success() {
        let stderr = String::from_utf8_lossy(&compile.stderr);
        return Err(Diagnostic::new(
            format!("clang -c {} failed: {stderr}", c_src.display()),
            Span::dummy(),
        ));
    }
    let archive_out = Command::new(&ar)
        .arg("rcs")
        .arg(archive)
        .arg(&obj)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Diagnostic::new(format!("spawn ar failed: {e}"), Span::dummy()))?;
    if !archive_out.status.success() {
        let stderr = String::from_utf8_lossy(&archive_out.stderr);
        return Err(Diagnostic::new(
            format!("ar rcs failed: {stderr}"),
            Span::dummy(),
        ));
    }
    if !archive.is_file() {
        return Err(Diagnostic::new(
            format!("static lib missing after ar: {}", archive.display()),
            Span::dummy(),
        ));
    }
    Ok(())
}

fn is_shared_lib(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("so") | Some("dylib") | Some("dll")
    )
}

/// Host shared-library file name (`libfoo.dylib` / `libfoo.so` / `foo.dll`).
pub fn dynamic_lib_file_name(stem: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{stem}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{stem}.dylib")
    } else {
        format!("lib{stem}.so")
    }
}

/// F05.01: compile one C file into a shared library (`.so` / `.dylib` / `.dll`).
pub fn build_c_dynamic_lib(c_src: &Path, dylib: &Path) -> Result<(), Diagnostic> {
    let clang = find_clang().ok_or_else(|| {
        Diagnostic::new(
            "clang not found (set CLANG or install a C toolchain)",
            Span::dummy(),
        )
    })?;
    if !c_src.is_file() {
        return Err(Diagnostic::new(
            format!("C source not found: {}", c_src.display()),
            Span::dummy(),
        ));
    }
    if let Some(parent) = dylib.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Diagnostic::new(format!("create dylib dir failed: {e}"), Span::dummy())
            })?;
        }
    }
    let abs_dylib = if dylib.is_absolute() {
        dylib.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| Diagnostic::new(format!("cwd failed: {e}"), Span::dummy()))?
            .join(dylib)
    };
    let mut compile = Command::new(&clang);
    compile
        .arg("-shared")
        .arg("-fPIC")
        .arg("-o")
        .arg(dylib)
        .arg(c_src);
    if cfg!(target_os = "macos") {
        compile.arg("-install_name").arg(&abs_dylib);
    }
    let compile = compile
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Diagnostic::new(format!("spawn clang -shared failed: {e}"), Span::dummy()))?;
    if !compile.status.success() {
        let stderr = String::from_utf8_lossy(&compile.stderr);
        return Err(Diagnostic::new(
            format!("clang -shared {} failed: {stderr}", c_src.display()),
            Span::dummy(),
        ));
    }
    if !dylib.is_file() {
        return Err(Diagnostic::new(
            format!("dynamic lib missing after clang: {}", dylib.display()),
            Span::dummy(),
        ));
    }
    Ok(())
}

fn work_dir(prefix: &str) -> Result<PathBuf, Diagnostic> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "{prefix}-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir)
        .map_err(|e| Diagnostic::new(format!("temp dir failed: {e}"), Span::dummy()))?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    fn module_of(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn for_await_of_custom_async_iterable_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/annex-b/for_await_of.drac"
        ))
        .expect("read");
        let m = module_of(&src);
        assert!(
            crate::es_generators::is_es_generators_module(&m),
            "expected es_generators classify to accept for_await_of"
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "for_await_of must not use hello stub:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-for-await-of").expect("workdir");
        let bin = dir.join("for_await_of");
        build_native_binary(&ir, &bin).expect("build");
        let output = std::process::Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "6\n9\n30\n6\n3\n4\n",
            "stdout={:?}",
            String::from_utf8_lossy(&output.stdout)
        );
    }

    #[test]
    fn empty_program_emits_runtime_hello() {
        let ir = emit_llvm_ir(&module_of("")).expect("emit");
        assert!(
            ir.contains("draconic_rt_hello"),
            "IR must declare/call runtime hello:\n{ir}"
        );
        assert!(
            ir.contains("define i32 @main"),
            "IR must define main:\n{ir}"
        );
        assert!(
            ir.contains("call void @draconic_rt_hello"),
            "main must call hello:\n{ir}"
        );
        assert!(ir.contains("ret i32 0"), "main must return 0:\n{ir}");
    }

    #[test]
    fn empty_native_binary_prints_hello() {
        let ir = emit_llvm_ir(&module_of("")).expect("emit");
        let dir = work_dir("draconic-llvm-test").expect("workdir");
        let bin = dir.join("hello");
        build_native_binary(&ir, &bin).expect("build_native_binary");

        let output = Command::new(&bin).output().expect("run binary");
        assert!(
            output.status.success(),
            "binary exit {:?}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "hello\n", "stdout={stdout:?}");
    }

    #[test]
    fn unsupported_js_module_errors() {
        let err = emit_llvm_ir(&module_of("let o = {};")).expect_err("must reject");
        let msg = err.to_string();
        assert!(
            msg.contains("unsupported") || msg.contains("native target"),
            "diagnostic should mention unsupported native IR:\n{msg}"
        );
        assert!(
            !msg.contains("draconic_rt_hello"),
            "error must not be a hello-stub success path:\n{msg}"
        );
    }

    #[test]
    fn es_expr_unary_keywords_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let t_num = typeof 1;
            let t_str = typeof "hi";
            let t_bool = typeof true;
            let t_null = typeof null;
            let v0 = void 0;
            let v1 = void 1;
            let d_lit = delete 1;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr unary keywords must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_str"),
            "should print string/undefined results:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bool"),
            "should print bool delete result:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-unary-keywords").expect("workdir");
        let bin = dir.join("unary_keywords");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "number\nstring\nboolean\nobject\nundefined\nundefined\ntrue\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_arithmetic_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let sum = 1 + 2;
            let diff = 10 - 4;
            let prod = 3 * 4;
            let quot = 20 / 5;
            let rem = 10 % 3;
            let prec = 1 + 2 * 3;
            let grouped = (1 + 2) * 3;
            let unary_minus = -5;
            let unary_plus = +7;
            let chain = 1 + 2 + 3 - 4;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr arithmetic must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-arith").expect("workdir");
        let bin = dir.join("arith");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "3\n6\n12\n4\n1\n7\n9\n-5\n7\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_numbers_number_literals_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/numbers/number_literals.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "number_literals must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-number-literals").expect("workdir");
        let bin = dir.join("number_literals");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout,
            "42\n0\n3.1400000000000001\n0.5\n0.5\n5\n1000\n1000\n150\n200\n0.10000000000000001\n6.02e+23\n255\n255\n16\n10\n10\n15\n15\n1000\n1000000\n65535\n161\n1000.5\n10000000000\n36\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_numbers_bigint_literals_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/numbers/bigint_literals.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "bigint_literals must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_i64"),
            "should print i64 BigInt results:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bytes") || ir.contains("draconic_rt_print_str"),
            "should print typeof string:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-bigint-literals").expect("workdir");
        let bin = dir.join("bigint_literals");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout,
            "1\n2\n3\n2\n20\n3\n1\n255\n255\n10\n10\n15\n15\n1000\n65535\n161\n0\n-1\nbigint\n36\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_numbers_bigint_pow_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/numbers/bigint_pow.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "bigint_pow must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_i64"),
            "should print i64 BigInt results:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bytes") || ir.contains("draconic_rt_print_str"),
            "should print typeof string:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-bigint-pow").expect("workdir");
        let bin = dir.join("bigint_pow");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout,
            "8\n1024\n512\n64\n1\n1\n32\n9\n-8\n16\nbigint\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_numbers_math_basics_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/numbers/math_basics.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "math_basics must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("llvm.fabs.f64") || ir.contains("llvm.floor.f64"),
            "should use Math f64 intrinsics:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 Math results:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bytes") || ir.contains("draconic_rt_print_str"),
            "should print typeof Math string:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-math-basics").expect("workdir");
        let bin = dir.join("math_basics");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout,
            "3\n3\n4\n4\n1\n3\n1024\n3\n-1\ntrue\ntrue\nobject\n4\ntrue\ntrue\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_numbers_number_global_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/numbers/number_global.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "number_global must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("fcmp uno") || ir.contains("0x7FF8000000000000"),
            "should lower Number.isNaN / NaN:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bool"),
            "should print boolean Number results:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bytes") || ir.contains("draconic_rt_print_str"),
            "should print typeof strings:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-number-global").expect("workdir");
        let bin = dir.join("number_global");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout,
            "true\nfalse\ntrue\nfalse\ntrue\nfalse\ntrue\nfalse\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\nfunction\nnumber\nnumber\ntrue\ntrue\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_values_symbol_basics_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/values/symbol_basics.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "symbol_basics must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_symbol_new") && ir.contains("draconic_rt_symbol_for"),
            "should lower Symbol / Symbol.for:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_symbol_key_for"),
            "should lower Symbol.keyFor:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-symbol-basics").expect("workdir");
        let bin = dir.join("symbol_basics");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout,
            "symbol\nsymbol\ntrue\ntrue\nshared\nfunction\nfunction\nfunction\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_values_symbol_property_keys_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/values/symbol_property_keys.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "symbol_property_keys must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_object_set_symbol")
                && ir.contains("draconic_rt_object_get_symbol"),
            "should lower symbol-keyed get/set:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "should alloc objects:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-symbol-property-keys").expect("workdir");
        let bin = dir.join("symbol_property_keys");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout,
            "1\nundefined\n2\nundefined\n3\n3\nundefined\n4\nundefined\n5\n6\n7\n6\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_arithmetic_with_local_refs_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let a = 10;
            let b = 3;
            let sum = a + b;
            let prod = a * b;
            let div = a / b;
            let rem = a % b;
            let chain = a + b * 2 - 4;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "local-ref arithmetic must not use hello stub:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-arith-local").expect("workdir");
        let bin = dir.join("arith_local");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "10\n3\n13\n30\n3.3333333333333335\n1\n12\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }


    #[test]
    fn es_objects_lit_access_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/objects/object_lit_access.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit object_lit_access");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_objects must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "es_objects must alloc objects:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-objects").expect("workdir");
        let bin = dir.join("object_lit_access");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "1\n1\n3\n3\n4\n4\n", "stdout={stdout:?}");
    }

    #[test]
    fn es_objects_property_assign_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/objects/property_assign.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit property_assign");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_objects must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_object_set"),
            "es_objects must set properties:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-objects-assign").expect("workdir");
        let bin = dir.join("property_assign");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "2\n3\n4\n5\n6\n7\n8\n8\n", "stdout={stdout:?}");
    }

    #[test]
    fn es_objects_this_method_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/objects/this_method.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit this_method");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_objects must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_") || ir.contains("define double @es_m_fn_"),
            "es_objects must emit method functions:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-objects-this").expect("workdir");
        let bin = dir.join("this_method");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "10\n10\n15\n3\n7\n7\n", "stdout={stdout:?}");
    }

    #[test]
    fn es_objects_new_ctor_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/objects/new_ctor.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit new_ctor");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_objects must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "es_objects new must alloc instances:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_"),
            "es_objects must emit constructor functions:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-objects-new").expect("workdir");
        let bin = dir.join("new_ctor");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "1\n2\n10\n10\n3\n6\n3\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_objects_prototype_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/objects/prototype.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit prototype");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_objects must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_object_set_proto"),
            "es_objects prototype must set [[Prototype]]:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_"),
            "es_objects must emit prototype methods:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-objects-proto").expect("workdir");
        let bin = dir.join("prototype");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "3\n3\n6\n9\n7\n7\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_objects_lit_sugar_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/objects/object_lit_sugar.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit object_lit_sugar");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_objects must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "es_objects sugar must alloc objects:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_"),
            "es_objects sugar must emit method functions:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-objects-sugar").expect("workdir");
        let bin = dir.join("object_lit_sugar");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "1\n2\n1\n2\n3\n4\n5\n6\n7\n1\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_classes_basic_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/classes/class_basic.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit class_basic");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_classes must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "es_classes must alloc objects:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_object_set_proto"),
            "es_classes must set [[Prototype]]:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_"),
            "es_classes must emit ctor/method functions:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-classes-basic").expect("workdir");
        let bin = dir.join("class_basic");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "1\n2\n3\n6\n7\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_classes_extends_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/classes/class_extends.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit class_extends");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_classes extends must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "es_classes extends must alloc objects:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_object_set_proto"),
            "es_classes extends must set [[Prototype]]:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_"),
            "es_classes extends must emit ctor/method functions:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-classes-extends").expect("workdir");
        let bin = dir.join("class_extends");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "1\n3\n1\n1\n2\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_classes_static_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/classes/class_static.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit class_static");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_classes static must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "es_classes static must alloc objects:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_"),
            "es_classes static must emit ctor/method functions:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-classes-static").expect("workdir");
        let bin = dir.join("class_static");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "5\n42\n7\n9\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_classes_super_access_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/classes/class_super_access.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit class_super_access");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_classes super must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "es_classes super must alloc objects:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_object_set_proto"),
            "es_classes super must set [[Prototype]]:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_"),
            "es_classes super must emit ctor/method functions:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-classes-super-access").expect("workdir");
        let bin = dir.join("class_super_access");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "5\n7\n15\n9\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_arrays_lit_access_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/arrays/array_lit_access.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit array_lit_access");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_arrays must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_new"),
            "es_arrays must alloc arrays:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_get"),
            "es_arrays must get elements:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_len"),
            "es_arrays must read length:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-arrays").expect("workdir");
        let bin = dir.join("array_lit_access");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "1\n2\n3\n3\n0\n10\n21\n1\n2\ntwo\n7\n8\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_arrays_element_assign_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/arrays/array_element_assign.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit array_element_assign");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_arrays assign must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_set"),
            "es_arrays assign must set elements:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_get"),
            "es_arrays assign must get elements:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-arrays-assign").expect("workdir");
        let bin = dir.join("array_element_assign");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "10\n2\n20\n2\n30\n40\n40\n7\n1\n9\n3\n5\n6\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_arrays_spread_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/arrays/array_spread.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit array_spread");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_arrays spread must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_spread_array")
                || ir.contains("draconic_rt_array_spread_cstr"),
            "es_arrays spread must call spread helpers:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-arrays-spread").expect("workdir");
        let bin = dir.join("array_spread");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout,
            "1\n2\n2\n1\n2\n3\n3\n0\n1\n2\n3\n4\n10\n1\n2\n3\n1\n2\n99\n7\n0\n5\n1\n1\n2\n3\n3\na\nb\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_arrays_for_of_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/arrays/array_for_of.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit array_for_of");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_arrays for-of must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_get") && ir.contains("draconic_rt_array_len"),
            "es_arrays for-of must iterate via array get/len:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-arrays-for-of").expect("workdir");
        let bin = dir.join("array_for_of");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout,
            "6\n0\nab\n60\n15\n5\n3\n5\n6\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_arrays_destructure_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/arrays/array_destructure.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(src.as_str())).expect("emit array_destructure");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_arrays destructure must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_get") && ir.contains("draconic_rt_array_new"),
            "es_arrays destructure must use array ABI:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-arrays-destructure").expect("workdir");
        let bin = dir.join("array_destructure");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout,
            "1\n2\n3\n1\n2\n3\n4\n2\n10\n20\n30\n60\n7\n8\n15\n100\n200\n300\n2\n5\n6\n11\n2\n12\n13\n1\n3\n7\n4\n1\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_call_spread_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/arrays/call_spread.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit call_spread");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "call_spread must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("cs_fn_") || ir.contains("call double"),
            "call_spread must emit calls:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-call-spread").expect("workdir");
        let bin = dir.join("call_spread");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "6\n60\n6\n6\n6\nxyz\n7\n8\n1\n2\n15\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_string_literal_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let s = "hi";
            let n = 1 + 2;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr string must not use hello stub:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-string-lit").expect("workdir");
        let bin = dir.join("string_lit");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "hi\n3\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_strings_lit_access_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/strings/string_lit_access.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "string_lit_access must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bytes"),
            "should print length-aware strings:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-string-lit-access").expect("workdir");
        let bin = dir.join("string_lit_access");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = output.stdout;
        let expected = b"hello\nworld\n\n\nhelloworld\nabc\nn1\n2n\nx\ny\nabc\n3\n0\na\nb\nc\n1\nb\na\nb\na\tb\na\rb\na\\b\na\"b\na'b\na\0b\nit's \"ok\"\nstring\ntrue\ntrue\n";
        assert_eq!(
            stdout, expected,
            "stdout={:?}\nir=\n{ir}",
            String::from_utf8_lossy(&stdout)
        );
    }

    #[test]
    fn es_strings_template_lit_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/strings/template_lit.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "template_lit must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bytes"),
            "should print length-aware strings:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-template-lit").expect("workdir");
        let bin = dir.join("template_lit");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = output.stdout;
        let expected = b"hello\n\na\nb\nworld\nhello world\nworld!\naworldb\n3\nn=3\nx1y2z\nsum=3\nouter inner world end\na`b$c\\d\na\nb\nstring\ntrue\nab\n";
        assert_eq!(
            stdout, expected,
            "stdout={:?}\nir=\n{ir}",
            String::from_utf8_lossy(&stdout)
        );
    }

    #[test]
    fn es_strings_unicode_escapes_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/strings/unicode_escapes.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "unicode_escapes must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bytes"),
            "should print length-aware strings:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-unicode-escapes").expect("workdir");
        let bin = dir.join("unicode_escapes");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = output.stdout;
        // Content is UTF-8; .length is UTF-16 code units (emoji=2, ©=1).
        let mut expected = Vec::new();
        expected.extend_from_slice(b"AB\na\n");
        expected.push(0); // hex_null
        expected.extend_from_slice(b"\nA\n");
        expected.extend_from_slice("\u{00A9}".as_bytes());
        expected.extend_from_slice(b"\n \nA\n");
        expected.extend_from_slice("\u{1F600}".as_bytes());
        expected.extend_from_slice(b"\n");
        expected.extend_from_slice("\u{00FF}".as_bytes());
        expected.extend_from_slice(b"\nABC\nHi\nOK\n");
        expected.extend_from_slice("\u{1F4A9}".as_bytes());
        expected.extend_from_slice(b"\nxAy\ntrue\ntrue\ntrue\n2\n1\nHi\n");
        assert_eq!(
            stdout, expected,
            "stdout={:?}\nir=\n{ir}",
            String::from_utf8_lossy(&stdout)
        );
    }

    #[test]
    fn es_strings_tagged_template_prints_native() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/strings/tagged_template.drac"
        ))
        .expect("read fixture");
        let ir = emit_llvm_ir(&module_of(&src)).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "tagged_template must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_new"),
            "should build quasi array:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-tagged-template").expect("workdir");
        let bin = dir.join("tagged_template");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout,
            "hello1\nworld\nhello world!\nx1y2z3\na`b\ntrue\np9q\nm7n\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_comparison_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let lt = 1 < 2;
            let lte = 2 <= 2;
            let gt = 3 > 1;
            let gte = 3 >= 3;
            let eq_loose = 1 == 1;
            let ne_loose = 1 != 2;
            let eq_strict = 1 === 1;
            let ne_strict = 1 !== 2;
            let chain = 1 < 2 === true;
            let falsey = 2 < 1;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr comparison must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bool"),
            "should print bool results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-cmp").expect("workdir");
        let bin = dir.join("cmp");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "true\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\nfalse\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_logical_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let and_tt = true && true;
            let and_tf = true && false;
            let and_ft = false && true;
            let or_ff = false || false;
            let or_ft = false || true;
            let or_tf = true || false;
            let not_t = !true;
            let not_f = !false;
            let prec = !false && true || false;
            let value_and = 1 && 2;
            let value_or = 0 || 3;
            let group = !(false || true);
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr logical must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_bool"),
            "should print bool results:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print number results for value-preserving &&/||:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-logical").expect("workdir");
        let bin = dir.join("logical");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "true\nfalse\nfalse\nfalse\ntrue\ntrue\nfalse\ntrue\ntrue\n2\n3\nfalse\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_bitwise_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let band = 5 & 3;
            let bor = 5 | 2;
            let bxor = 5 ^ 1;
            let bnot = ~0;
            let shl = 1 << 3;
            let shr = -8 >> 2;
            let ushr = -8 >>> 2;
            let prec = 1 | 2 & 4;
            let group = (1 | 2) & 4;
            let chain = 15 & 7 | 8;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr bitwise must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-bitwise").expect("workdir");
        let bin = dir.join("bitwise");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "1\n7\n4\n-1\n8\n-2\n1073741822\n1\n0\n15\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_exponentiation_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let pow = 2 ** 3;
            let right_assoc = 2 ** 3 ** 2;
            let prec = 2 * 3 ** 2;
            let group = (2 * 3) ** 2;
            let nested = 2 ** (1 + 2);
            let zero = 5 ** 0;
            let one = 9 ** 1;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr exponentiation must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("llvm.pow.f64"),
            "should use pow intrinsic:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-exponentiation").expect("workdir");
        let bin = dir.join("exponentiation");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "8\n512\n18\n36\n8\n1\n9\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_conditional_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let t = true ? 1 : 2;
            let f = false ? 1 : 2;
            let nested = true ? false ? 3 : 4 : 5;
            let right_assoc = false ? 1 : true ? 2 : 3;
            let prec = 1 < 2 ? 10 : 20;
            let group = (false ? 1 : 2) + 3;
            let num = 0 ? 100 : 200;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr conditional must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("select i1"),
            "should use select for ternary:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-conditional").expect("workdir");
        let bin = dir.join("conditional");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "1\n2\n4\n2\n10\n5\n200\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_assignment_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let x = 0;
            x = 1;
            let y = 0;
            y = x = 2;
            let z = 0;
            z = true ? 3 : 4;
            let w = 0;
            w = false ? 5 : w = 6;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr assignment must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("store double"),
            "should store assigned values:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-assignment").expect("workdir");
        let bin = dir.join("assignment");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "2\n2\n3\n6\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_update_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let a = 1;
            let pre_inc = ++a;
            let b = 1;
            let pre_dec = --b;
            let c = 1;
            let post_inc = c++;
            let d = 1;
            let post_dec = d--;
            let e = 5;
            ++e;
            e++;
            --e;
            e--;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr update must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("fadd double") || ir.contains("fsub double"),
            "should use fadd/fsub for ++/--:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-update").expect("workdir");
        let bin = dir.join("update");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "2\n2\n0\n0\n2\n1\n0\n1\n5\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_compound_assignment_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let a = 10;
            a += 5;
            let b = 10;
            b -= 3;
            let c = 4;
            c *= 3;
            let d = 20;
            d /= 4;
            let e = 17;
            e %= 5;
            let f = 2;
            f **= 3;
            let g = 1;
            g <<= 3;
            let h = 16;
            h >>= 2;
            let i = -8;
            i >>>= 1;
            let j = 15;
            j &= 9;
            let k = 12;
            k ^= 5;
            let l = 8;
            l |= 3;
            let m = 1;
            let n = 2;
            m += n += 3;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr compound assignment must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-compound-assignment").expect("workdir");
        let bin = dir.join("compound_assignment");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "15\n7\n12\n5\n2\n8\n8\n4\n2147483644\n9\n9\n11\n6\n5\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_comma_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let a = (1, 2);
            let b = (1, 2, 3);
            let c = 0;
            let d = (c = 1, c = 2, 3);
            let e = (true ? 1 : 2, 4);
            let side = 0;
            let f = (side = side + 1, side = side + 1, side);
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr comma must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-comma").expect("workdir");
        let bin = dir.join("comma");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "2\n3\n2\n3\n4\n2\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_expr_break_continue_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let a = 0;
            while (true) {
              a = a + 1;
              if (a === 3) break;
            }
            let b = 0;
            let c = 0;
            while (b < 5) {
              b = b + 1;
              if (b === 2) continue;
              c = c + 1;
            }
            let d = 0;
            for (let i = 0; i < 10; i = i + 1) {
              d = d + 1;
              if (i === 2) break;
            }
            let e = 0;
            for (let j = 0; j < 5; j = j + 1) {
              if (j === 2) continue;
              e = e + 1;
            }
            let f = 0;
            do {
              f = f + 1;
              if (f === 2) break;
            } while (true);
            let g = 0;
            let h = 0;
            do {
              g = g + 1;
              if (g === 2) continue;
              h = h + 1;
            } while (g < 4);
            let outer = 0;
            let inner = 0;
            while (outer < 3) {
              outer = outer + 1;
              while (true) {
                inner = inner + 1;
                break;
              }
            }
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_expr break/continue must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-break-continue").expect("workdir");
        let bin = dir.join("break_continue");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "3\n5\n4\n3\n4\n2\n4\n3\n3\n3\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_switch_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/statements/switch.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "switch fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("switch_end") || ir.contains("case"),
            "should lower switch with case labels:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-switch").expect("workdir");
        let bin = dir.join("switch");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "10\n20\n40\n11\n1\n5\n1\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_labeled_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/statements/labeled.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "labeled fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("lbl_end") || ir.contains("while_end") || ir.contains("for_end"),
            "should lower labeled break/continue targets:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-labeled").expect("workdir");
        let bin = dir.join("labeled");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "1\n5\n4\n1\n1\n2\n1\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_for_in_of_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/statements/for_in_of.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "for_in_of fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("forin_") || ir.contains("forof_"),
            "should lower for-in/for-of loops:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_cstr_concat")
                && ir.contains("draconic_rt_cstr_from_u64")
                && ir.contains("draconic_rt_cstr_from_code_unit"),
            "should use cstr helpers:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-for-in-of").expect("workdir");
        let bin = dir.join("for_in_of");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "ab\n01\nab\nx\n0\nx\nyz\nz\n01\n1\n2\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_function_decl_return_call_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/functions/decl_return_call.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "decl_return_call fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 result:\n{ir}"
        );
        assert!(
            ir.contains("define double @"),
            "should emit LLVM function for JS function decl:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-fn-decl").expect("workdir");
        let bin = dir.join("decl_return_call");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "1\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_function_nested_capture_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/functions/nested_capture.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "nested_capture fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        assert!(
            ir.contains("define double @"),
            "should emit LLVM functions for nested decls:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-fn-nested").expect("workdir");
        let bin = dir.join("nested_capture");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "42\n17\n3\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_function_default_params_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/functions/default_params.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "default_params fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        assert!(
            ir.contains("define double @"),
            "should emit LLVM functions with defaults:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-fn-defaults").expect("workdir");
        let bin = dir.join("default_params");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "10\n7\n11\n3\n3\n9\n5\n9\n6\n8\n10\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_function_rest_params_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/functions/rest_params.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "rest_params fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        assert!(
            ir.contains("define double @"),
            "should emit LLVM functions with rest:\n{ir}"
        );
        assert!(
            ir.contains("%rest_buf") || ir.contains("rest_buf"),
            "should pack rest args:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-fn-rest").expect("workdir");
        let bin = dir.join("rest_params");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "0\n1\n6\n12\n7\n2\n9\n0\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn native_ints_add_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let a: i32 = 10;
            let b: i32 = 3;
            let sum: i32 = a + b;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "native int program should not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_i64"),
            "should print ints:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n01").expect("workdir");
        let bin = dir.join("ints");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "10\n3\n13\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn native_ints_function_call() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            function add(x: i32, y: i32): i32 {
              return x + y;
            }
            let s: i32 = add(20, 22);
            "#,
        ))
        .expect("emit");
        let dir = work_dir("draconic-llvm-n01-fn").expect("workdir");
        let bin = dir.join("fn");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "42\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    /// F06.03: `extern "C"` lowers to LLVM `declare` ABI surface; call links libc `abs`.
    #[test]
    fn native_extern_c_declare_and_call_abs() {
        let m = module_of(
            r#"
            extern "C" function abs(x: i32): i32;
            extern "C" function puts(s: *u8): i32;
            extern "C" function free(p: *u8): void;
            let a: i32 = abs(-42);
            "#,
        );
        assert!(m.has_extern_ffi);
        assert!(
            m.body.iter().any(|s| matches!(
                s,
                draconic_ir::Stmt::ExternFunction { name, .. } if name == "abs"
            )),
            "IR must keep ExternFunction: {:?}",
            m.body
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "extern module must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("declare i32 @abs(i32)"),
            "expected declare abs:\n{ir}"
        );
        assert!(
            ir.contains("declare i32 @puts(ptr)"),
            "expected declare puts:\n{ir}"
        );
        assert!(
            ir.contains("declare void @free(ptr)"),
            "expected declare free:\n{ir}"
        );
        assert!(
            ir.contains("call i32 @abs("),
            "expected call abs:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-f06-03-extern").expect("workdir");
        let bin = dir.join("extern_abs");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "42\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    /// F01.01: multi-arg i32 extern "C" call → runtime `draconic_rt_add_i32`.
    #[test]
    fn native_extern_c_call_i32_multi_arg() {
        let m = module_of(
            r#"
            extern "C" function draconic_rt_add_i32(a: i32, b: i32): i32;
            let s: i32 = draconic_rt_add_i32(20, 22);
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            ir.contains("declare i32 @draconic_rt_add_i32(i32, i32)"),
            "expected declare add_i32:\n{ir}"
        );
        assert!(
            ir.contains("call i32 @draconic_rt_add_i32("),
            "expected call add_i32:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-f01-01-add-i32").expect("workdir");
        let bin = dir.join("extern_add_i32");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "42\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    /// F01.02: i64 / f64 / void extern "C" calls via Runtime ABI.
    #[test]
    fn native_extern_c_call_i64_f64_void() {
        let m = module_of(
            r#"
            extern "C" function draconic_rt_add_i64(a: i64, b: i64): i64;
            extern "C" function draconic_rt_add_f64(a: f64, b: f64): f64;
            extern "C" function draconic_rt_touch_void(): void;
            draconic_rt_touch_void();
            let s: i64 = draconic_rt_add_i64(3000000000, 2000000000);
            let t: f64 = draconic_rt_add_f64(10.5, 2.0);
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            ir.contains("declare i64 @draconic_rt_add_i64(i64, i64)"),
            "expected declare add_i64:\n{ir}"
        );
        assert!(
            ir.contains("declare double @draconic_rt_add_f64(double, double)"),
            "expected declare add_f64:\n{ir}"
        );
        assert!(
            ir.contains("declare void @draconic_rt_touch_void()"),
            "expected declare touch_void:\n{ir}"
        );
        assert!(
            ir.contains("call i64 @draconic_rt_add_i64("),
            "expected call add_i64:\n{ir}"
        );
        assert!(
            ir.contains("call double @draconic_rt_add_f64("),
            "expected call add_f64:\n{ir}"
        );
        assert!(
            ir.contains("call void @draconic_rt_touch_void()"),
            "expected call touch_void:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-f01-02-i64-f64-void").expect("workdir");
        let bin = dir.join("extern_i64_f64_void");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "void\n5000000000\n12.5\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    /// F01.03: pointer (`*i32`) and null args to extern "C" via Runtime ABI.
    #[test]
    fn native_extern_c_call_ptr_and_null() {
        let m = module_of(
            r#"
            extern "C" function draconic_rt_load_i32(p: *i32): i32;
            let x: i32 = 42;
            let p: *i32 = &x;
            let a: i32 = draconic_rt_load_i32(p);
            let b: i32 = draconic_rt_load_i32(&x);
            let n: *i32 = null;
            let c: i32 = draconic_rt_load_i32(n);
            let d: i32 = draconic_rt_load_i32(null);
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            ir.contains("declare i32 @draconic_rt_load_i32(ptr)"),
            "expected declare load_i32:\n{ir}"
        );
        assert!(
            ir.contains("call i32 @draconic_rt_load_i32(ptr"),
            "expected call load_i32 with ptr:\n{ir}"
        );
        assert!(
            ir.contains("call i32 @draconic_rt_load_i32(ptr null)")
                || ir.contains("call i32 @draconic_rt_load_i32(ptr null,"),
            "expected call with null pointer:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-f01-03-ptr").expect("workdir");
        let bin = dir.join("extern_ptr");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "42\n42\n42\n0\n0\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    /// F02.01: export a Draconic fn as a C function pointer (pass to extern).
    #[test]
    fn native_export_fn_as_c_function_pointer() {
        let m = module_of(
            r#"
            function twice(x: i32): i32 {
              return x + x;
            }
            extern "C" function draconic_rt_fnptr_nonnull(cb: function): i32;
            let ok: i32 = draconic_rt_fnptr_nonnull(twice);
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            ir.contains("declare i32 @draconic_rt_fnptr_nonnull(ptr)"),
            "expected declare fnptr helper:\n{ir}"
        );
        assert!(
            ir.contains("define i32 @d_twice_"),
            "expected Draconic fn define:\n{ir}"
        );
        assert!(
            ir.contains("call i32 @draconic_rt_fnptr_nonnull(ptr @d_twice_"),
            "expected pass of fn address as ptr:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-f02-01-fnptr").expect("workdir");
        let bin = dir.join("export_fnptr");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "1\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    /// F02.02: host invokes callback with scalar args; return value observed.
    #[test]
    fn native_host_invokes_callback_scalar_args() {
        let m = module_of(
            r#"
            function add(a: i32, b: i32): i32 {
              return a + b;
            }
            extern "C" function draconic_rt_call_i32_i32(cb: function, a: i32, b: i32): i32;
            let r: i32 = draconic_rt_call_i32_i32(add, 20, 22);
            let s: i32 = draconic_rt_call_i32_i32(add, -5, 12);
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            ir.contains("declare i32 @draconic_rt_call_i32_i32(ptr, i32, i32)"),
            "expected declare call helper:\n{ir}"
        );
        assert!(
            ir.contains("define i32 @d_add_"),
            "expected Draconic fn define:\n{ir}"
        );
        assert!(
            ir.contains("call i32 @draconic_rt_call_i32_i32(ptr @d_add_"),
            "expected pass of fn address as ptr:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-f02-02-invoke").expect("workdir");
        let bin = dir.join("invoke_scalar");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "42\n7\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    /// F03.01: native layout field offsets match C ABI (i32+i64 padding; i8+i32 padding).
    #[test]
    fn native_repr_c_struct_field_offsets() {
        let m = module_of(
            r#"
            type Pair = { a: i32; b: i64 };
            type Small = { x: i8; y: i32 };
            extern "C" function draconic_rt_layout_i32_i64_a(p: *u8): i32;
            extern "C" function draconic_rt_layout_i32_i64_b(p: *u8): i64;
            extern "C" function draconic_rt_layout_i32_i64_write(p: *u8, a: i32, b: i64): void;
            extern "C" function draconic_rt_layout_i8_i32_x(p: *u8): i8;
            extern "C" function draconic_rt_layout_i8_i32_y(p: *u8): i32;
            let p: Pair = { a: 10, b: 20 };
            let ra: i32 = draconic_rt_layout_i32_i64_a(&p);
            let rb: i64 = draconic_rt_layout_i32_i64_b(&p);
            let q: Pair = { a: 0, b: 0 };
            draconic_rt_layout_i32_i64_write(&q, 7, 8);
            let qa: i32 = q.a;
            let qb: i64 = q.b;
            let s: Small = { x: 1, y: 99 };
            let sx: i8 = draconic_rt_layout_i8_i32_x(&s);
            let sy: i32 = draconic_rt_layout_i8_i32_y(&s);
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            ir.contains("declare i32 @draconic_rt_layout_i32_i64_a(ptr)"),
            "expected declare layout a:\n{ir}"
        );
        assert!(
            ir.contains("{ i32, i64 }"),
            "expected LLVM struct {{ i32, i64 }}:\n{ir}"
        );
        assert!(
            ir.contains("{ i8, i32 }"),
            "expected LLVM struct {{ i8, i32 }}:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-f03-01-layout").expect("workdir");
        let bin = dir.join("layout_offsets");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "10\n20\n10\n20\n7\n8\n7\n8\n1\n99\n1\n99\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    /// F03.02: pass/return native layout struct by value or pointer across FFI.
    #[test]
    fn native_pass_return_struct_across_ffi() {
        let m = module_of(
            r#"
            type Pair = { a: i32; b: i64 };
            extern "C" function draconic_rt_layout_pass_i32_i64(p: Pair): i32;
            extern "C" function draconic_rt_layout_ret_i32_i64(a: i32, b: i64): Pair;
            extern "C" function draconic_rt_layout_pass_i32_i64_ptr(p: *u8): i32;
            let p: Pair = { a: 10, b: 20 };
            let by_val: i32 = draconic_rt_layout_pass_i32_i64(p);
            let by_ptr: i32 = draconic_rt_layout_pass_i32_i64_ptr(&p);
            let q: Pair = draconic_rt_layout_ret_i32_i64(7, 8);
            let qa: i32 = q.a;
            let qb: i64 = q.b;
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            ir.contains("declare i32 @draconic_rt_layout_pass_i32_i64([2 x i64])"),
            "expected by-value Pair param as [2 x i64]:\n{ir}"
        );
        assert!(
            ir.contains("declare [2 x i64] @draconic_rt_layout_ret_i32_i64(i32, i64)"),
            "expected by-value Pair return as [2 x i64]:\n{ir}"
        );
        assert!(
            ir.contains("declare i32 @draconic_rt_layout_pass_i32_i64_ptr(ptr)"),
            "expected pointer Pair param:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-f03-02-pass-return").expect("workdir");
        let bin = dir.join("pass_return");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "10\n20\n30\n30\n7\n8\n7\n8\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn native_ints_wrapping_i8() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let a: i8 = 120;
            let b: i8 = a + 10;
            "#,
        ))
        .expect("emit");
        let dir = work_dir("draconic-llvm-n01-wrap").expect("workdir");
        let bin = dir.join("wrap");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        // 120 + 10 = 130 → i8 wrap → -126
        assert_eq!(stdout, "120\n-126\n", "stdout={stdout:?}");
    }

    #[test]
    fn native_floats_add_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let a: f64 = 10.5;
            let b: f64 = 2.0;
            let sum: f64 = a + b;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "native float program should not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print floats:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n02").expect("workdir");
        let bin = dir.join("floats");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "10.5\n2\n12.5\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn native_bool_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let t: bool = true;
            let f: bool = false;
            "#,
        ))
        .expect("emit");
        let dir = work_dir("draconic-llvm-n02-bool").expect("workdir");
        let bin = dir.join("bool");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "true\nfalse\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn native_struct_field_read_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            type Point = { x: i32; y: i32 };
            let p: Point = { x: 10, y: 20 };
            let a: i32 = p.x;
            let b: i32 = p.y;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "native struct program should not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("getelementptr"),
            "should GEP struct fields:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n03-struct").expect("workdir");
        let bin = dir.join("struct");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "10\n20\n10\n20\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_promise_basics_prints_after_drain() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let tf = typeof Promise;
            let resolved = 0;
            let rejected = 0;
            let chained = 0;
            let p = new Promise(function (resolve) {
              resolve(42);
            });
            p.then(function (v) {
              resolved = v;
            });
            let q = new Promise(function (_resolve, reject) {
              reject(7);
            });
            q.then(
              function () {
                rejected = -1;
              },
              function (e) {
                rejected = e;
              }
            );
            new Promise(function (resolve) {
              resolve(1);
            }).then(function (v) {
              return v + 1;
            }).then(function (v) {
              chained = v;
            });
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "Promise basics must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_construct"),
            "should construct via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_then"),
            "should then via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_job_drain"),
            "should drain jobs before observe:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n06-promise").expect("workdir");
        let bin = dir.join("promise");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "function\n42\n7\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_promise_resolve_reject_catch_prints_after_drain() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let tResolve = typeof Promise.resolve;
            let tReject = typeof Promise.reject;
            let resolved = 0;
            let rejected = 0;
            let caught = 0;
            let p = Promise.resolve(42);
            p.then(function (v) {
              resolved = v;
            });
            let q = Promise.reject(7);
            q.then(
              function () {
                rejected = -1;
              },
              function (e) {
                rejected = e;
              }
            );
            let r = Promise.reject(9);
            r.catch(function (e) {
              caught = e;
            });
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "Promise resolve/reject must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_new"),
            "should allocate via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_resolve"),
            "should resolve via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_reject"),
            "should reject via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_then"),
            "should then/catch via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_job_drain"),
            "should drain jobs before observe:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n06-promise-rr").expect("workdir");
        let bin = dir.join("promise_rr");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "function\nfunction\n42\n7\n9\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_promise_finally_prints_after_drain() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let tFinally = typeof Promise.resolve(1).finally;
            let fulfilledSide = 0;
            let rejectedSide = 0;
            let resolved = 0;
            let caught = 0;
            let p = Promise.resolve(42);
            p.finally(function () {
              fulfilledSide = 1;
            }).then(function (v) {
              resolved = v;
            });
            let q = Promise.reject(7);
            q.finally(function () {
              rejectedSide = 1;
            }).catch(function (e) {
              caught = e;
            });
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "Promise finally must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_finally"),
            "should finally via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_then"),
            "should then/catch via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_job_drain"),
            "should drain jobs before observe:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n06-promise-finally").expect("workdir");
        let bin = dir.join("promise_finally");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "function\n1\n1\n42\n7\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_promise_all_prints_after_drain() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let tAll = typeof Promise.all;
            let emptyLen = -1;
            let allLen = -1;
            let a0 = -1;
            let a1 = -1;
            let mixed0 = -1;
            let mixed1 = -1;
            let rejected = 0;
            Promise.all([]).then(function (v) {
              emptyLen = v.length;
            });
            Promise.all([Promise.resolve(10), Promise.resolve(20)]).then(function (v) {
              allLen = v.length;
              a0 = v[0];
              a1 = v[1];
            });
            Promise.all([1, Promise.resolve(2)]).then(function (v) {
              mixed0 = v[0];
              mixed1 = v[1];
            });
            Promise.all([Promise.resolve(1), Promise.reject(7)]).then(
              function () {
                rejected = -1;
              },
              function (e) {
                rejected = e;
              }
            );
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "Promise.all must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_all"),
            "should Promise.all via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_new"),
            "should allocate arrays via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_job_drain"),
            "should drain jobs before observe:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n06-promise-all").expect("workdir");
        let bin = dir.join("promise_all");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "function\n0\n2\n10\n20\n1\n2\n7\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_promise_race_prints_after_drain() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let tRace = typeof Promise.race;
            let winner = -1;
            let mixed = -1;
            let rejected = 0;
            Promise.race([Promise.resolve(10), Promise.resolve(20)]).then(function (v) {
              winner = v;
            });
            Promise.race([1, Promise.resolve(2)]).then(function (v) {
              mixed = v;
            });
            Promise.race([Promise.reject(7), Promise.resolve(1)]).then(
              function () {
                rejected = -1;
              },
              function (e) {
                rejected = e;
              }
            );
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "Promise.race must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_race"),
            "should Promise.race via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_job_drain"),
            "should drain jobs before observe:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n06-promise-race").expect("workdir");
        let bin = dir.join("promise_race");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "function\n10\n1\n7\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_promise_all_settled_prints_after_drain() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let tAllSettled = typeof Promise.allSettled;
            let emptyLen = -1;
            let settledLen = -1;
            let s0 = "";
            let v0 = -1;
            let s1 = "";
            let r1 = -1;
            let mixed0 = "";
            let mixedV0 = -1;
            let mixed1 = "";
            let mixedV1 = -1;
            Promise.allSettled([]).then(function (v) {
              emptyLen = v.length;
            });
            Promise.allSettled([Promise.resolve(10), Promise.reject(7)]).then(function (v) {
              settledLen = v.length;
              s0 = v[0].status;
              v0 = v[0].value;
              s1 = v[1].status;
              r1 = v[1].reason;
            });
            Promise.allSettled([1, Promise.resolve(2)]).then(function (v) {
              mixed0 = v[0].status;
              mixedV0 = v[0].value;
              mixed1 = v[1].status;
              mixedV1 = v[1].value;
            });
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "Promise.allSettled must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_all_settled"),
            "should Promise.allSettled via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_object_get"),
            "should read status/value/reason via object_get:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_job_drain"),
            "should drain jobs before observe:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n06-promise-all-settled").expect("workdir");
        let bin = dir.join("promise_all_settled");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "function\n0\n2\nfulfilled\n10\nrejected\n7\nfulfilled\n1\nfulfilled\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_promise_any_prints_after_drain() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let tAny = typeof Promise.any;
            let winner = -1;
            let mixed = -1;
            let allRejected = 0;
            let errName = "";
            let errLen = -1;
            let emptyRejected = 0;
            let emptyName = "";
            let emptyLen = -1;
            Promise.any([Promise.resolve(10), Promise.resolve(20)]).then(function (v) {
              winner = v;
            });
            Promise.any([1, Promise.resolve(2)]).then(function (v) {
              mixed = v;
            });
            Promise.any([Promise.reject(7), Promise.reject(9)]).then(
              function () {
                allRejected = -1;
              },
              function (e) {
                allRejected = 1;
                errName = e.name;
                errLen = e.errors.length;
              }
            );
            Promise.any([]).then(
              function () {
                emptyRejected = -1;
              },
              function (e) {
                emptyRejected = 1;
                emptyName = e.name;
                emptyLen = e.errors.length;
              }
            );
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "Promise.any must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_any"),
            "should Promise.any via Runtime ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_object_get"),
            "should read name/errors via object_get:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_job_drain"),
            "should drain jobs before observe:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n06-promise-any").expect("workdir");
        let bin = dir.join("promise_any");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "function\n10\n1\n1\nAggregateError\n2\n1\nAggregateError\n0\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_async_await_prints_after_drain() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let resolved = 0;
            let fromAwait = 0;
            let rejected = 0;
            let exprResolved = 0;
            async function f() {
              return 42;
            }
            f().then(function (v) {
              resolved = v;
            });
            async function g() {
              let x = await Promise.resolve(7);
              return x + 1;
            }
            g().then(function (v) {
              fromAwait = v;
            });
            async function h() {
              throw 9;
            }
            h().then(
              function () {
                rejected = -1;
              },
              function (e) {
                rejected = e;
              }
            );
            let af = async function () {
              return 1;
            };
            af().then(function (v) {
              exprResolved = v;
            });
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "async/await must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_promise_await") || ir.contains("draconic_rt_promise_new"),
            "should lower async via Runtime Promise ABI:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_job_drain"),
            "should drain jobs before observe:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n06-async-await").expect("workdir");
        let bin = dir.join("async_await");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "42\n8\n9\n1\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn native_fixed_array_index_read_prints() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            type Vec3 = [i32, i32, i32];
            let v: Vec3 = [10, 20, 30];
            let a: i32 = v[0];
            let b: i32 = v[1];
            let c: i32 = v[2];
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "native fixed-array program should not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("getelementptr"),
            "should GEP array elements:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n03-array").expect("workdir");
        let bin = dir.join("array");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "10\n20\n30\n10\n20\n30\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_direct_eval_prints_via_embed() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let t = typeof eval;
            let g = globalThis.eval === eval;
            let a = eval("1 + 2");
            let b = eval("typeof undefined");
            let c = eval("3 * 4");
            let d = eval("'hi'");
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "direct eval must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("N07.02") || ir.contains("direct eval"),
            "should use eval emit path:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n07-direct-eval").expect("workdir");
        let bin = dir.join("direct_eval");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "function\ntrue\n3\nundefined\n12\nhi\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_new_function_prints_via_embed() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            let tf = typeof Function;
            let same = globalThis.Function === Function;
            let f = new Function("a", "b", "return a + b");
            let g = Function("x", "return x * 2");
            let h = new Function("return 7");
            let r1 = f(1, 2);
            let r2 = g(3);
            let r3 = h();
            let t1 = typeof f;
            let t2 = typeof g;
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "new Function must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("N07.03") || ir.contains("Function via Embed"),
            "should use Function emit path:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n07-new-function").expect("workdir");
        let bin = dir.join("new_function");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "function\ntrue\nfunction\nfunction\nfunction\n3\n6\n7\nfunction\nfunction\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_labelled_function_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/annex-b/labelled_function.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "labelled_function fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-16-11-labelled-fn").expect("workdir");
        let bin = dir.join("labelled_function");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "5\n2\n3\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn es_if_function_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/annex-b/if_function.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "if_function fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_str"),
            "should print typeof string:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-16-12-if-fn").expect("workdir");
        let bin = dir.join("if_function");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "1\nundefined\n4\n5\n7\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_block_function_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/annex-b/block_function.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "block_function fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_str"),
            "should print typeof strings:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-16-13-block-fn").expect("workdir");
        let bin = dir.join("block_function");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "1\nfunction\nundefined\n3\n4\n2\n5\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_arguments_object_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/annex-b/arguments_object.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "arguments_object fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-16-24-arguments").expect("workdir");
        let bin = dir.join("arguments_object");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "5\n32\n1\n8\n3\n6\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_var_decl_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/annex-b/var_decl.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "var_decl fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_str"),
            "should print undefined strings:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-16-14-var-decl").expect("workdir");
        let bin = dir.join("var_decl");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "3\n2\nundefined\n4\nundefined\nundefined\nundefined\n6\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_var_for_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/annex-b/var_for.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "var_for fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64") && ir.contains("draconic_rt_print_str"),
            "should print number and string results:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_array_get") || ir.contains("forof_"),
            "should lower array for-of:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-16-15-var-for").expect("workdir");
        let bin = dir.join("var_for");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "ab\n78\n3\nxy\nx\ny\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_if_else_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/statements/if_else.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "if_else fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("br i1") && ir.contains("then") && ir.contains("endif"),
            "should lower if/else with branches:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-if-else").expect("workdir");
        let bin = dir.join("if_else");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "1\n0\n10\n20\n3\n5\n1\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_while_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/statements/while.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "while fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("br i1") && ir.contains("while_head") && ir.contains("while_end"),
            "should lower while with loop branches:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-while").expect("workdir");
        let bin = dir.join("while");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "3\n0\n2\n3\n6\n0\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_do_while_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/statements/do_while.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "do_while fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("br i1") && ir.contains("do_body") && ir.contains("do_end"),
            "should lower do/while with loop branches:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-do-while").expect("workdir");
        let bin = dir.join("do_while");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "3\n1\n2\n3\n6\n0\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_for_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/statements/for.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "for fixture must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("br i1") && ir.contains("for_head") && ir.contains("for_end"),
            "should lower for with loop branches:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-for").expect("workdir");
        let bin = dir.join("for");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "3\n2\n6\n3\n5\n3\n2\n2\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_throw_try_catch_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/exceptions/throw_try_catch.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "throw_try_catch must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("N08.10") || ir.contains("throw/try/catch"),
            "should use exceptions emit path:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-throw-try-catch").expect("workdir");
        let bin = dir.join("throw_try_catch");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "1\n1\n1\n7\n5\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn r04_01_catchable_exceptions_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/security/panic_policy/catchable_exceptions.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "catchable_exceptions must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("N08.10") || ir.contains("throw/try/catch"),
            "should use exceptions emit path:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-r04-01-catchable").expect("workdir");
        let bin = dir.join("catchable_exceptions");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "1\n1\n7\n1\n", "stdout={stdout:?}\nir=\n{ir}");
    }

    #[test]
    fn r04_02_abort_process_kills_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/security/panic_policy/abort_process.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "abort_process must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_abort"),
            "should call Runtime abort:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-r04-02-abort").expect("workdir");
        let bin = dir.join("abort_process");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            !output.status.success(),
            "abort must kill the process; exit {:?}\nstdout={}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.is_empty(),
            "abort must not print after; stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_try_finally_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/exceptions/try_finally.drac"
        )))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "try_finally must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("N08.10.02") || ir.contains("finally"),
            "should use exceptions finally emit path:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 results:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-try-finally").expect("workdir");
        let bin = dir.join("try_finally");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "11\n11\n23\n5\n1\n11\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_indirect_eval_prints_via_embed() {
        let ir = emit_llvm_ir(&module_of(
            r#"
            globalThis.gx = 100;
            function probeDirect() {
              let gx = 200;
              return eval("gx");
            }
            function probeIndirectComma() {
              let gx = 200;
              return (0, eval)("gx");
            }
            function probeIndirectGlobalThis() {
              let gx = 200;
              return globalThis.eval("gx");
            }
            let d = probeDirect();
            let i = probeIndirectComma();
            let g = probeIndirectGlobalThis();
            let t = typeof (0, eval);
            let same = globalThis.eval === eval;
            let a = (0, eval)("1 + 2");
            let b = globalThis.eval("'hi'");
            "#,
        ))
        .expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "indirect eval must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("N07.04") || ir.contains("indirect eval"),
            "should use indirect eval emit path:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n07-indirect-eval").expect("workdir");
        let bin = dir.join("indirect_eval");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "200\n100\n100\nfunction\ntrue\n3\nhi\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_legacy_with_basic_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/legacy/with_basic.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit with_basic");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_legacy must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "es_legacy must print numbers:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-legacy-with").expect("workdir");
        let bin = dir.join("with_basic");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "1\n2\n10\n20\n3\n", "stdout={stdout:?}");
    }

    #[test]
    fn es_legacy_with_nested_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/legacy/with_nested.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit with_nested");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "es_legacy must not use hello stub:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-legacy-with-nested").expect("workdir");
        let bin = dir.join("with_nested");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "1\n2\n7\n", "stdout={stdout:?}");
    }

    #[test]
    fn es_class_expr_prints_native() {
        let ir = emit_llvm_ir(&module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/annex-b/class_expr.drac"
            ))
            .expect("read fixture")
            .as_str(),
        ))
        .expect("emit class_expr");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "class_expr must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "class_expr must alloc objects:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-class-expr").expect("workdir");
        let bin = dir.join("class_expr");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, "1\n2\n3\n6\nCounter\n13\n10\n7\n42\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    #[test]
    fn es_class_expr_name_prints_native() {
        let ir = emit_llvm_ir(&module_of(include_str!(
            "../../../tests/conformance/fixtures/es/annex-b/class_expr_name.drac"
        )))
        .expect("emit class_expr_name");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "class_expr_name must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_str"),
            "class_expr_name must print name strings:\n{ir}"
        );
        let dir = work_dir("draconic-llvm-n08-16-34-class-expr-name").expect("workdir");
        let bin = dir.join("class_expr_name");
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout,
            "cls\nX\nfunction\ndCls\nY\nfunction\noCls\nZ\nfunction\npCls\nW\nfunction\naCls\nQ\n",
            "stdout={stdout:?}\nir=\n{ir}"
        );
    }

    /// F04.01: extra `.a` on the link line resolves a C symbol not in Runtime/libc.
    #[test]
    fn native_link_static_lib_resolves_c_symbol() {
        let dir = work_dir("draconic-llvm-f04-01-link-static").expect("workdir");
        let c_src = dir.join("touch.c");
        std::fs::write(
            &c_src,
            "void draconic_link_static_touch(void) {}\n",
        )
        .expect("write c");
        let archive = dir.join("libtouch.a");
        build_c_static_lib(&c_src, &archive).expect("build .a");

        let m = module_of(
            r#"
            extern "C" function draconic_link_static_touch(): void;
            draconic_link_static_touch();
            let x: i32 = 1;
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            ir.contains("declare void @draconic_link_static_touch()"),
            "expected declare:\n{ir}"
        );
        assert!(
            ir.contains("call void @draconic_link_static_touch()"),
            "expected call:\n{ir}"
        );

        let missing = dir.join("no_lib");
        let err = build_native_binary(&ir, &missing).expect_err("link without .a must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("draconic_link_static_touch")
                || msg.contains("undefined")
                || msg.contains("Unresolved"),
            "expected unresolved symbol, got {msg}"
        );

        let bin = dir.join("linked");
        build_native_binary_with_static_libs(&ir, &bin, &[archive]).expect("link with .a");
        assert!(bin.is_file(), "native binary missing at {}", bin.display());
    }

    /// F04.02: call a linked static symbol; native stdout is the C return value.
    #[test]
    fn native_link_static_lib_call_end_to_end() {
        let dir = work_dir("draconic-llvm-f04-02-link-static-call").expect("workdir");
        let c_src = dir.join("add.c");
        std::fs::write(
            &c_src,
            "int draconic_link_static_add(int a, int b) { return a + b; }\n",
        )
        .expect("write c");
        let archive = dir.join("libadd.a");
        build_c_static_lib(&c_src, &archive).expect("build .a");

        let m = module_of(
            r#"
            extern "C" function draconic_link_static_add(a: i32, b: i32): i32;
            let s: i32 = draconic_link_static_add(20, 22);
            let t: i32 = draconic_link_static_add(-5, 12);
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            ir.contains("declare i32 @draconic_link_static_add(i32, i32)"),
            "expected declare:\n{ir}"
        );
        assert!(
            ir.contains("call i32 @draconic_link_static_add"),
            "expected call:\n{ir}"
        );

        let bin = dir.join("linked");
        build_native_binary_with_static_libs(&ir, &bin, &[archive]).expect("link with .a");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "42\n7\n",
            "stdout must be C-computed returns"
        );
    }

    /// F05.01: extra shared lib on the link line resolves a C symbol not in Runtime/libc.
    #[test]
    fn native_link_dynamic_lib_resolves_c_symbol() {
        let dir = work_dir("draconic-llvm-f05-01-link-dynamic").expect("workdir");
        let c_src = dir.join("touch.c");
        std::fs::write(
            &c_src,
            "void draconic_link_dynamic_touch(void) {}\n",
        )
        .expect("write c");
        let dylib = dir.join(dynamic_lib_file_name("touch"));
        build_c_dynamic_lib(&c_src, &dylib).expect("build shared lib");

        let m = module_of(
            r#"
            extern "C" function draconic_link_dynamic_touch(): void;
            draconic_link_dynamic_touch();
            let x: i32 = 1;
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            ir.contains("declare void @draconic_link_dynamic_touch()"),
            "expected declare:\n{ir}"
        );
        assert!(
            ir.contains("call void @draconic_link_dynamic_touch()"),
            "expected call:\n{ir}"
        );

        let missing = dir.join("no_lib");
        let err = build_native_binary(&ir, &missing).expect_err("link without dylib must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("draconic_link_dynamic_touch")
                || msg.contains("undefined")
                || msg.contains("Unresolved"),
            "expected unresolved symbol, got {msg}"
        );

        let bin = dir.join("linked");
        build_native_binary_with_dynamic_libs(&ir, &bin, &[dylib]).expect("link with shared lib");
        assert!(bin.is_file(), "native binary missing at {}", bin.display());
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "1\n",
            "stdout must be the local let, proving the shared-lib symbol resolved"
        );
    }

    /// F05.02: call a linked dynamic symbol; native stdout is the C return value.
    #[test]
    fn native_link_dynamic_lib_call_end_to_end() {
        let dir = work_dir("draconic-llvm-f05-02-link-dynamic-call").expect("workdir");
        let c_src = dir.join("add.c");
        std::fs::write(
            &c_src,
            "int draconic_link_dynamic_add(int a, int b) { return a + b; }\n",
        )
        .expect("write c");
        let dylib = dir.join(dynamic_lib_file_name("add"));
        build_c_dynamic_lib(&c_src, &dylib).expect("build shared lib");

        let m = module_of(
            r#"
            extern "C" function draconic_link_dynamic_add(a: i32, b: i32): i32;
            let s: i32 = draconic_link_dynamic_add(20, 22);
            let t: i32 = draconic_link_dynamic_add(-5, 12);
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            ir.contains("declare i32 @draconic_link_dynamic_add(i32, i32)"),
            "expected declare:\n{ir}"
        );
        assert!(
            ir.contains("call i32 @draconic_link_dynamic_add"),
            "expected call:\n{ir}"
        );

        let bin = dir.join("linked");
        build_native_binary_with_dynamic_libs(&ir, &bin, &[dylib]).expect("link with shared lib");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "42\n7\n",
            "stdout must be C-computed returns"
        );
    }

    /// F05.02: missing shared lib is a typed diagnostic (E0402), not a raw linker dump.
    #[test]
    fn native_link_dynamic_lib_missing_is_typed_error() {
        let dir = work_dir("draconic-llvm-f05-02-missing-dylib").expect("workdir");
        let m = module_of(
            r#"
            extern "C" function draconic_link_dynamic_add(a: i32, b: i32): i32;
            let s: i32 = draconic_link_dynamic_add(20, 22);
            "#,
        );
        let ir = emit_llvm_ir(&m).expect("emit");
        let missing = dir.join(dynamic_lib_file_name("no_such"));
        assert!(!missing.is_file(), "fixture path must not exist");
        let bin = dir.join("no_bin");
        let err = build_native_binary_with_dynamic_libs(&ir, &bin, &[missing.clone()])
            .expect_err("missing dylib must fail");
        assert_eq!(
            err.code,
            Some(draconic_diagnostics::codes::MISSING_DYNAMIC_LIB),
            "missing dylib must carry E0402, got {err}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("E0402"),
            "typed error must include E0402, got {msg}"
        );
        assert!(
            msg.contains("dynamic lib not found"),
            "typed error must name the miss, got {msg}"
        );
        assert!(
            msg.contains(&missing.display().to_string()),
            "typed error must include the path, got {msg}"
        );
    }
}
