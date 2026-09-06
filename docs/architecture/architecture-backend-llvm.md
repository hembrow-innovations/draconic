---
id: "architecture-backend-llvm"
title: "LLVM backend"
kind: architecture
description: "IR to native object or binary via private adapters, Runtime link, and hard errors."
domain: draconic
area: backends
tags: []
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# LLVM backend

## Overview

The crate `draconic-backend-llvm` lowers a shared [[architecture-ir|IR]] `Module` to LLVM IR text, then clang links that text with the [[architecture-runtime|Runtime]] into a native object or binary. There is one public lowerer (`emit_llvm_ir` / `emit_llvm_ir_with_debug`). Private adapters each claim a supported subset. Unclassified non-empty IR is a hard diagnostic. The empty Program is the B08 hello stub only.

Host I/O lowerings live in this crate (the `host_*` adapters) as well as in the Runtime ABI they call.

## Context

[[0002-shared-ir-dual-backends]] forbids a second typed AST and a WASM-only IR fork. [[0003-gc-runtime-and-dual-worlds]] requires a tracing GC Runtime for JS values; native types stay unboxed. Conformance `native.stdout` must be program results, not the B08 `hello\n` fallback. See [[CONTEXT]] (LLVM backend, Native-only, JS-only) and [[system-design-dual-backends]].

## Design

### Dispatcher (`lib.rs`)

`emit_llvm_ir_raw` tries adapters in a fixed order (`is_*_module` then `emit_*`). First match wins. If none match:

- **Empty body**: `emit_empty_hello` — `main` calls `draconic_rt_hello` (B08 demo). Not used for non-empty unsupported IR.
- **Otherwise**: `unsupported_native_diagnostic()` — hard error listing supported subsets. No silent hello success.

`emit_llvm_ir_with_debug` runs the same raw emit, then [[debug_info]] attaches DWARF when requested.

Link/build helpers in `lib.rs` (not adapters): `build_native_binary` and variants with extra static libs, extra dynamic libs, and optional LTO; `build_c_static_lib` / `build_c_dynamic_lib`; clang/ar discovery (`CLANG` / `AR`). Object compile is a separate clang `-c` step so DWARF from IR metadata is kept (then `dsymutil` on macOS when debug is on). Runtime link flags come from [[architecture-runtime]].

### Hello stub versus real observations

B08 empty-program IR prints `hello\n` when run. Every non-empty supported adapter must not contain `draconic_rt_hello`. Conformance `native.stdout` is the process stdout of the linked binary (Runtime `print_*` and host writes), not the stub.

Many ES adapters classify a fixture-sized subset, evaluate that subset (sometimes at compile time), and emit LLVM that prints the observations through Runtime ABI. That is subset lowering, not a general JS VM in LLVM. `native_ints` is the unboxed native-scalar/layout/pointer lowerer. `es_eval` folds `eval` / `Function` via [[architecture-embed]] then prints observations.

### Native scalars and layouts

- **native_ints.rs**: N01–N03 (and dual-world `number` as IEEE-754 double with explicit casts). Lowers pure native scalar/layout/pointer Programs, including `Stmt::ExternFunction` C ABI. Prints via Runtime `print_i64` / `print_u64` / `print_f64` / `print_bool`. Can emit `; draconic-dbg: line col` markers for DWARF.

### ES adapters (`es_*.rs`)

Private classify-and-emit modules for ECMAScript subsets. Grouped by area; every `es_*.rs` file is named here. Do not treat each file as its own architecture note.

- **Expressions and control (`es_expr.rs`, `es_nullish.rs`)**: numeric/BigInt/string/boolean expressions, assignment and updates, `typeof`/`void`/`delete`, `if`/`while`/`do`/`for`/`switch`/labels/`break`/`continue`, `for-in`/`for-of` over strings, untagged templates, number/Math/NaN/Infinity. Nullish and logical assignment live in `es_nullish.rs`.
- **Coercion and values (`es_coercion.rs`, `es_to_primitive.rs`, `es_values.rs`)**: abstract `==` / `!=`, ToPrimitive hooks, Symbol basics and symbol property keys.
- **Calls and strings (`es_call_spread.rs`, `es_tagged_template.rs`, `es_optional_chain.rs`)**: spread in call/`new`, tagged templates, optional chaining.
- **Functions (`es_functions.rs`, `es_param_dstr.rs`, `es_new_target.rs`)**: decl/expr/arrow, return, call, defaults, rest, capture, IIFE; parameter destructuring; `new.target`.
- **Eval (`es_eval.rs`)**: constant-string `eval` / `Function` fold via Embed; not a full interpreter in this file.
- **Async and generators (`es_promise.rs`, `es_async_methods.rs`, `es_generators.rs`)**: Promise/async/await subset via Runtime Promise ABI; async methods; generators, async generators, `for await`.
- **Objects and destructuring (`es_objects.rs`, `es_object_destructure.rs`, `es_destructure_defaults.rs`)**: object literals, property access/assignment, method `this`; object destructuring and defaults.
- **Classes and private (`es_classes.rs`, `es_class_expr_name.rs`, `es_instanceof.rs`, `es_private_methods.rs`, `es_private_accessors.rs`, `es_private_in.rs`, `es_static_private_fields.rs`, `es_static_private_methods.rs`, `es_static_blocks.rs`)**: class decl/heritage/static methods/`super`; class expression `.name`; `instanceof`; private methods/accessors/`in`; static private fields/methods; static blocks. Several of these evaluate the fixture surface at compile time after IR class desugar, then print observations.
- **Arrays (`es_arrays.rs`)**: array literals, index, `.length`, spread, `for-of`, array destructuring via Runtime array ABI.
- **Exceptions (`es_exceptions.rs`)**: `throw`, `try`/`catch`/`finally`, optional catch binding.
- **Modules (`es_modules.rs`)**: linked ESM flatten (named/default/namespace/cyclic) as number/string observations.
- **Proxies (`es_proxies.rs`)**: Proxy traps and Reflect subset.
- **Builtins and Annex B (`es_builtins.rs`, `es_legacy.rs`, `es_var_for.rs`)**: globals, Error, JSON, Date, RegExp, collections, typed arrays, `with`, Annex B, `var` and `var` in `for` heads.
- **Stdlib observations (`es_encoding.rs`, `es_collections.rs`, `es_mime.rs`, `es_logging.rs`, `es_testing.rs`)**: TextEncoder/Decoder, Base64/hex, `sha256` / `randomBytes` / HMAC / AEAD / gzip-deflate; `groupBy` / `chunk` / `Deque`; multipart; `createLogger`; `describe`/`it`/`expect`/hooks. Encoding uses the crypto/hash helpers below at compile time.

### Host I/O adapters (`host_*.rs`)

Native observations that call Runtime host ABI. Host errors that these files document (for example TCP `ECONN` / `EADDR` / `EPERM`) are Runtime/host behavior, not a second IR.

- **Process and OS (`host_process.rs`, `host_process_async.rs`, `host_os.rs`, `host_subprocess.rs`, `host_signals.rs`)**: args/env/exit/pid; async wait; cwd/hostname/temp/home; spawn/run/pipes; signal watch/ignore.
- **Stdio, path, files (`host_stdio.rs`, `host_path.rs`, `host_fs.rs`, `host_docs.rs`)**: stdout/stderr/stdin; path strings; whole-file and handle I/O. `host_docs.rs` is a dedicated lowering for the docs SSG Program (file I/O plus string scan / `if` / `while` for `website/generate.drac`), not a general ES adapter.
- **Time (`host_time.rs`, `host_timers.rs`)**: `nowMs` / `monotonicMs`; `setTimeout` / `setInterval`.
- **Network (`host_tcp.rs`, `host_tcp_async.rs`, `host_udp.rs`, `host_dns.rs`, `host_http.rs`, `host_http_server.rs`, `host_http2.rs`, `host_ws.rs`, `host_ws_e2e.rs`)**: TCP/TLS, async TCP, UDP, DNS, HTTP/1.1 helpers, HTTP server/client over TCP or TLS, HTTP/2 frames, WebSocket frames, WebSocket client dial+echo e2e.
- **Concurrency (`host_workers.rs`, `host_channels.rs`, `host_worker_channels.rs`, `host_atomics.rs`, `host_once.rs`, `host_cancel.rs`)**: workers, FIFO channels, worker+channel, shared-memory atomics, once cell, cancel tokens.

### Crypto, hash, and byte helpers

Not LLVM instruction lowerers. Compile-time helpers used by `es_encoding.rs` (and tests):

- **sha256.rs**: SHA-256 digest (`sha256` crate).
- **hmac.rs**: HMAC-SHA256 over that digest.
- **aead.rs**: AES-256-GCM (`aeadEncrypt` / `aeadDecrypt`).
- **hex.rs**: lowercase encode; mixed-case decode.
- **base64.rs**: RFC 4648.
- **compression.rs**: gzip / zlib-deflate via `flate2`.

### Cross compile (`cross_compile.rs`)

ROADMAP D04 matrix: linux/darwin/windows × amd64/arm64, each with an LLVM triple. `compile_object_for_triple` runs clang `-c -target <triple>`. Pairs clang cannot target return an error; D04 does not require a non-host success. `compile_object_for_non_host` tries non-host cells until one works.

### wasm32 WASI (`wasm32_wasi.rs`)

ROADMAP F09 smoke from the same shared IR (no WASM-only fork). Triple `wasm32-wasip1` with fallback `wasm32-wasi`. `compile_object_for_wasm32_wasi` needs a wasm-capable clang. `link_wasm32_wasi` uses `wasm-ld` with `--no-entry --export-all --allow-undefined`. Undefined Runtime symbols stay unresolved. Not a WASI libc or preview2 host. Output must start with wasm magic `\0asm`.

### Debug info (`debug_info.rs`)

U07 DWARF. `SourceDebug` holds path and source text. `attach_debug_info` adds `DICompileUnit` / `DIFile` / `DISubprogram` for `@main`, `!dbg` on main instructions, using the first non-dummy `body_spans` line unless `; draconic-dbg:` markers (from `native_ints`) set a finer location. Skips if `!llvm.dbg.cu` is already present.

## Trade-offs

- **Adapter dispatcher over one general lowerer**: each cluster can land real `native.stdout` without waiting for a complete LLVM JS VM. Cost: unclassified Programs hard-error even when JS emit would succeed (JS-only in practice until an adapter exists).
- **Compile-time evaluation in some ES adapters**: fixture observations match the JS target without lowering every IR node to LLVM. Those files say so in their module comments; they are not a second language semantics.
- **Hello stub only for empty Programs**: keeps B08, forbids stub-green Conformance.
- **Same IR for wasm object smoke**: F09 does not fork IR ([[0002-shared-ir-dual-backends]]).

## Consequences

- Native binaries always link [[architecture-runtime]] (GC, host ABI, prints). Dual worlds: JS values on the GC heap, native types unboxed ([[0003-gc-runtime-and-dual-worlds]]).
- Native-only features (pointers, `extern "C"`, registry `NATIVE_ONLY` host APIs) are valid here when an adapter covers them; the JS backend hard-errors.
- JS-only in this crate today is “no adapter + non-empty body”: `unsupported_native_diagnostic`, never wrong native code and never hello. The host-API registry currently has no js-only entries (native is required for every registered host name).
- Pipeline neighbors: [[architecture-pipeline]], [[architecture-frontend]], [[architecture-embed]] (eval fold).
