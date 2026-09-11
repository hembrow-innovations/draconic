//! LLVM backend: IR → native (one lowerer; private adapters for supported subsets).

mod aead;
mod base64;
mod compression;
mod debug_info;
mod emitter;
mod es_arrays;
mod es_builtins;
mod es_call_spread;
mod es_class_expr_name;
mod es_classes;
mod es_coercion;
mod es_collections;
mod es_console;
mod es_destructure_defaults;
mod es_encoding;
mod es_eval;
mod es_exceptions;
mod es_expr;
mod es_functions;
mod es_generators;
mod es_instanceof;
mod es_legacy;
mod es_logging;
mod es_mime;
mod es_modules;
mod es_new_target;
mod es_optional_chain;
mod es_param_dstr;
mod es_private_accessors;
mod es_static_private_methods;
mod es_testing;
mod hex;
mod hmac;
mod sha256;

mod es_nullish;
mod es_object_destructure;
mod es_objects;
mod es_static_blocks;

mod cross_compile;
mod es_private_in;
mod es_promise;
mod es_proxies;
mod es_tagged_template;
mod es_to_primitive;
mod es_values;
mod es_var_for;
mod host_atomics;
mod host_cancel;
mod host_catalog;
mod host_channels;
mod host_dns;
mod host_docs;
mod host_fs;
mod host_http;
mod host_http2;
mod host_http_server;
mod host_once;
mod host_os;
mod host_path;
mod host_process;
mod host_process_async;
mod host_signals;
mod host_stdio;
mod host_subprocess;
mod host_tcp;
mod host_tcp_async;
mod host_time;
mod host_timers;
mod host_udp;
mod host_worker_channels;
mod host_workers;
mod host_ws;
mod host_ws_e2e;
mod native_ints;
mod native_link;
mod wasm32_wasi;

pub use cross_compile::{
    compile_object_for_non_host, compile_object_for_triple, cross_compile_matrix,
    host_cross_compile_pair, CrossCompilePair,
};
pub use debug_info::SourceDebug;
pub(crate) use native_link::find_clang;
#[cfg(test)]
pub(crate) use native_link::work_dir;
pub use native_link::{
    build_c_dynamic_lib, build_c_static_lib, build_native_binary,
    build_native_binary_with_dynamic_libs, build_native_binary_with_lto,
    build_native_binary_with_static_libs, dynamic_lib_file_name,
};
pub use wasm32_wasi::{compile_object_for_wasm32_wasi, link_wasm32_wasi, WASM32_WASI_TRIPLE};

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::Module;

use es_expr::emit_es_expr_walk;
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
pub fn emit_llvm_ir_with_debug(module: &Module, debug: &SourceDebug) -> Result<String, Diagnostic> {
    emit_llvm_ir_inner(module, Some(debug))
}

fn emit_llvm_ir_inner(module: &Module, debug: Option<&SourceDebug>) -> Result<String, Diagnostic> {
    let ir = emit_llvm_ir_raw(module, debug)?;
    if let Some(dbg) = debug {
        Ok(debug_info::attach_debug_info(&ir, module, dbg))
    } else {
        Ok(ir)
    }
}

fn emit_llvm_ir_raw(module: &Module, debug: Option<&SourceDebug>) -> Result<String, Diagnostic> {
    if is_native_int_module(module) {
        return emit_native_ints(module, debug);
    }
    if is_empty_program(module) {
        return Ok(emit_empty_hello());
    }
    let _ = debug;
    emit_es_expr_walk(module).map_err(|_| unsupported_native_diagnostic())
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

#[cfg(test)]
mod emit_es_control;
#[cfg(test)]
mod emit_es_eval;
#[cfg(test)]
mod emit_es_expr;
#[cfg(test)]
mod emit_es_functions;
#[cfg(test)]
mod emit_es_objects;
#[cfg(test)]
mod emit_es_promise;
#[cfg(test)]
mod emit_native;

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    use draconic_frontend::compile_source;
    use draconic_ir::Module;

    fn module_of(src: &str) -> Module {
        compile_source(src).expect("compile")
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
    fn leftover_expr_control_flow_emits_via_walker() {
        let m = module_of("if (false) { 1 + 2; }");
        assert!(
            !crate::es_expr::is_es_expr_module(&m),
            "must miss is_es_expr_module so dispatch falls through to the walker"
        );
        let ir = emit_llvm_ir(&m).expect("walker emit");
        assert!(
            ir.contains("define i32 @main"),
            "walker must emit LLVM main:\n{ir}"
        );
        assert!(
            !ir.contains("draconic_rt_hello"),
            "leftover expr must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("br i1"),
            "walker should emit if/then control flow:\n{ir}"
        );
    }

    #[test]
    fn leftover_function_decl_emits_via_walker() {
        let m = module_of("function f() { return 1; }\nlet x = f();");
        assert!(
            !crate::es_expr::is_es_expr_module(&m),
            "function IR must miss is_es_expr_module so the walker lowers it"
        );
        let ir = emit_llvm_ir(&m).expect("walker emit");
        assert!(
            ir.contains("define double @"),
            "walker must emit LLVM function for JS function decl:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_f64"),
            "should print f64 result:\n{ir}"
        );
        assert!(
            !ir.contains("draconic_rt_hello"),
            "leftover function must not use hello stub:\n{ir}"
        );
    }

    #[test]
    fn leftover_class_decl_emits_via_walker() {
        let m = module_of(
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/conformance/fixtures/es/classes/class_basic.drac"
            ))
            .expect("read fixture")
            .as_str(),
        );
        assert!(
            !crate::es_expr::is_es_expr_module(&m),
            "class IR must miss is_es_expr_module so the walker lowers it"
        );
        let ir = emit_llvm_ir(&m).expect("walker emit");
        assert!(
            ir.contains("draconic_rt_alloc_object"),
            "walker must alloc class instances:\n{ir}"
        );
        assert!(
            ir.contains("define double @m_fn_"),
            "walker must emit ctor/method functions:\n{ir}"
        );
        assert!(
            !ir.contains("draconic_rt_hello"),
            "leftover class must not use hello stub:\n{ir}"
        );
    }
}
