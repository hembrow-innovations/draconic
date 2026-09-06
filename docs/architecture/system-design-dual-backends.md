---
id: "system-design-dual-backends"
title: "Dual backends"
kind: system-design
description: "One shared IR, two lowerings, and portable versus native-only versus JS-only policy."
status: draft
domain: draconic
area: backends
tags: []
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Dual backends

## Overview

After the [[architecture-frontend|Frontend]], every Program is one [[architecture-ir|IR]] `Module`. The [[architecture-backend-js|JS backend]] and [[architecture-backend-llvm|LLVM backend]] both lower that module. Observable behavior that both can express is a portable program. A feature valid on exactly one backend is native-only or JS-only; the other backend hard-errors with a diagnostic and must not emit silent wrong code. Locked in [[0002-shared-ir-dual-backends]] and named in [[CONTEXT]].

## Components

- **Shared IR (`draconic-ir`)**: `lower(&CheckedProgram) -> Module`. Statement/expression tree, checker types, class/private desugar, `has_extern_ffi`. See [[architecture-ir]].
- **JS backend (`draconic-backend-js`)**: `emit_js` / `emit_js_with_map`. Runnable ECMAScript plus optional Source Map v3. Polyfills from [[architecture-runtime]] when used. Rejects pointers, `extern "C"`, and js-unavailable host APIs.
- **LLVM backend (`draconic-backend-llvm`)**: `emit_llvm_ir` then clang + Runtime. Private adapters for supported subsets. Empty Program is B08 hello only. Unclassified non-empty IR is a hard diagnostic. Host I/O adapters live here and call Runtime ABI.
- **Runtime**: native GC, prints, host ABI, JS polyfill strings. Linked into native binaries ([[architecture-runtime]], [[0003-gc-runtime-and-dual-worlds]]).
- **Embed**: `eval` / `Function` fold on native via [[architecture-embed]]; JS emit leaves `eval` as JS.
- **Checker host registry**: `CompileTarget::{Js, Native}` and `HostAvailability::{BOTH, NATIVE_ONLY}` in `draconic-check`. Emit re-checks. See [[architecture-pipeline]].

## Interfaces

- **IR in**: `Module` from `draconic_ir::lower`.
- **JS out**: `Result<String, Diagnostic>` or `EmittedJs { code, map }`.
- **LLVM out**: LLVM IR text; `build_native_binary` (and lib/LTO variants) to an executable; `compile_object_for_triple` / `compile_object_for_wasm32_wasi` to objects; `link_wasm32_wasi` to a `.wasm` smoke artifact.
- **Diagnostics**: `draconic_diagnostics::Diagnostic`. Native-only JS messages name pointers / native-only FFI / `host API … unsupported on js target (native-only; …)`. LLVM unsupported IR names the native target and supported subsets. Never succeed via hello for a non-empty Program.

## Data model

The shared artifact is [[architecture-ir]] `Module` (locals, body, body_spans, shapes, has_extern_ffi). There is no per-backend typed AST and no second IR for wasm.

Portable / native-only / JS-only are not fields on `Module`. They are consequences of types, `has_extern_ffi`, host `IdentName`s, and whether each backend can lower:

- **Portable program** ([[CONTEXT]]): both backends accept with equivalent observable behavior after documented polyfills.
- **Native-only**: valid on LLVM only. JS hard-errors. From code, not a wish list: `Type::Ptr`, unary `&`/`*`, `*p = …`, `has_extern_ffi` / `Stmt::ExternFunction`, and host APIs with `HostAvailability::NATIVE_ONLY` (signals, file handles, TLS, async TCP, UDP, HTTP/2, WebSocket, `httpServeStatic`, `processWaitAsync`, `workerOsThread`, once, shared-memory atomics, and the other registry `NATIVE_ONLY` names).
- **JS-only**: valid on JS only; LLVM hard-errors. The host registry has no js-only entries (tests require every registered host name on native). JS-only in the LLVM crate is unclassified non-empty IR: `unsupported_native_diagnostic()`. Checker also rejects JS-only types on `extern` signatures (F06.02) so they never become native ABI.

Native scalars and layouts without pointers are portable by N04: JS erases them to ordinary JS values; LLVM `native_ints` lowers them unboxed.

## Interactions

1. Frontend produces `CheckedProgram` (link first if the Program has import/export).
2. `lower` produces one `Module`.
3. JS: reject native-only → prepend used polyfills → print ECMAScript (and maps).
4. LLVM: pick first matching adapter → emit LLVM IR → clang `-c` → link Runtime (+ extra libs). Empty body → hello stub. No adapter → error.
5. Conformance compares JS stdout to engine/Node output and native stdout to the linked binary (`native.stdout`), not to `hello\n`, except the empty B08 case.

Eval on native: `es_eval` asks Embed to fold, then prints observations. Eval on JS: identifier `eval` in emitted source.

Wasm32/wasi and D04 cross-compile consume the same LLVM IR text; they change clang target, not the IR crate.

## Error handling

- **Hard error beats wrong code**: no pointer-as-no-op JS; no hello-stub success for `let o = {}`; no invented lowering for an unmatched adapter.
- **Check vs emit**: `check` without `CompileTarget` may accept a Program that `build --target js` will reject. Emit is authoritative for target availability.
- **Clang / tools missing**: LLVM build/cross/wasm paths return diagnostics (`clang not found`, target not available). Wasm link is smoke (`--allow-undefined`), not a hidden WASI runtime.
- **Catchable exceptions** stay Program `throw`/`try` ([[CONTEXT]], ADR-0011). Host I/O failures follow Runtime/host adapters (stderr tokens and exit on some TCP paths), not IR.

## Trade-offs

From [[0002-shared-ir-dual-backends]]:

- **Rejected: typed AST per backend.** Semantics would drift (class desugar, private brands, extern ABI would fork).
- **Rejected: foreign IR (WASM-only, and similar).** GC, Embed, and JS-faithful behavior need full control. F09 wasm object/link is a clang target on the same LLVM backend.

Further choices in code:

- **JS pretty-print vs LLVM adapters**: JS can emit a wide ES surface because the engine runs it. LLVM lands cluster-by-cluster with real `native.stdout`. Portable is “both accept,” not “LLVM is a complete JS VM.”
- **Polyfills live in Runtime, injection in JS backend**: one source for sha256/host bridges; LLVM uses Runtime C (or compile-time helpers in encoding).
- **Hello stub retained only for empty Programs**: historical B08 without poisoning Conformance.

Neighbors: [[architecture-pipeline]], [[architecture-frontend]], [[architecture-runtime]], [[architecture-embed]].
