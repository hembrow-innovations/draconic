---
id: "architecture-backend-js"
title: "JS backend"
kind: architecture
description: "IR to runnable ECMAScript, with polyfills and native-only hard errors."
domain: draconic
area: backends
tags: []
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# JS backend

## Overview

The crate `draconic-backend-js` lowers a shared [[architecture-ir|IR]] `Module` to ECMAScript source. Public entry points are `emit_js` (code only) and `emit_js_with_map` (code plus Source Map v3). Where the JS target can express the Program, emit is meant to be semantically equivalent. Native-only IR hard-errors. Documented polyfills from [[architecture-runtime]] are prepended when the body actually uses those names.

Modules in this crate:

- **lib.rs**: native-only / FFI rejection, host-API rejection, use-detection, polyfill prepend, `emit_js` / `emit_js_with_map`.
- **emit.rs**: statement and expression text.
- **source_map.rs**: Source Map v3 (VLQ), `SourceMapOptions`, mapping URL comment.

## Context

[[0002-shared-ir-dual-backends]] requires both backends to consume one IR. The JS backend is the path that prints ECMAScript. Native pointers, `extern "C"`, and host APIs the checker marks unavailable on `CompileTarget::Js` must not become silent wrong JS. See [[CONTEXT]] (JS backend, Portable program, Native-only) and [[system-design-dual-backends]].

## Design

### Pipeline

`emit_js_full` runs in order:

1. `reject_native_only` — walk locals and the statement tree.
2. `reject_extern_ffi` — if `module.has_extern_ffi`, diagnostic via `extern_unsupported_on_js_diagnostic` (F08.01).
3. Optionally prepend [[architecture-runtime]] polyfill source when the body uses a named local or free `IdentName`.
4. `emit::emit_stmt` for each top-level statement. Names come from `module.locals`.
5. If source-map options were passed, record one mapping at the start of each top-level statement using `module.body_spans`. Nested statements share that origin.

`Stmt::ExternFunction` is a no-op in `emit.rs` because FFI is rejected before emit.

### Native-only (hard error, never silent JS)

From `reject_native_only` and related helpers in `lib.rs`:

- **Pointer types**: a local with `IrType::Ptr(_)` errors (`*T` is native-only).
- **Pointer operators**: unary `&` / `*` (`UnaryOp::Ref` / `Deref`) error.
- **Pointer store**: `AssignTarget::Deref` errors (`*p = …`).
- **Host APIs unavailable on js**: free `IdentName` / `UpdateTarget::Name` / `AssignTarget::Name` / `Pattern::Name` go through `host_api_unsupported_diagnostic(name, CompileTarget::Js)`. The checker registry (`HostAvailability::NATIVE_ONLY`) is the list; the JS backend re-checks at emit so `draconic check` without a target can still fail at `build --target js`.

Native scalars (`i32` and the rest), layout structs, and fixed arrays are not rejected. Type annotations are already gone at IR. Values lower as ordinary JS numbers, objects, and arrays (N04 polyfill/erase). That is portable erasure, not a hard error.

### Polyfills (only if used)

Use-detection walks the body. Presence in `module.locals` is not enough, because binder symbols include builtins. Host APIs lower as free `IdentName`, so those walks look for names, not locals.

When a use is found, the corresponding `draconic_runtime::*_js_polyfill()` string is prepended (and assigned on `globalThis` where the polyfill does that):

- **Crypto / bytes / compression**: `sha256`, `randomBytes`, `hmacSha256`, `aeadEncrypt` / `aeadDecrypt`, `gzip` / `gunzip` / `deflate` / `inflate`.
- **Flags / URL / query / MIME / logging / collections / tests**: `parseFlags` / `flagHelp`, `parseUrl`, `parseQuery` / `serializeQuery`, `parseMultipart` / `serializeMultipart`, `createLogger`, `groupBy` / `chunk` / `Deque`, `describe` / `it` / `expect` / `before` / `after` / `beforeEach` / `afterEach` (`IdentName` so a user `let it` does not collide).
- **Process / OS (Node bridge)**: `processArgs`, `envGet` / `envSet` / `envDelete`, `exit` / `exitCode` / `setExitCode`, `pid` / `ppid`, `cwd` / `chdir`, `hostname` / `osType` / `osArch`, `tempDir` / `homeDir`, `processRun`, spawn/pipe/kill (`processSpawn` and related).
- **Workers / channels / cancel**: `spawnWorker` / `joinWorker` / `terminateWorker`, `makeChannel` / `channelSend` / `channelRecv`, cancel-token and timeout helpers.
- **Time / stdio / path / fs**: `nowMs`, `monotonicMs`, `setTimeout` / `clearTimeout`, `setInterval` / `clearInterval`, `stdoutWrite`, `stderrWrite`, `stdinReadLine` / `stdinReadBytes`, path helpers, whole-file fs helpers.
- **HTTP / DNS / TCP (H17.04 Node bridge)**: HTTP/1.1 parse/write helpers, `dnsLookup`, sync TCP (`tcpListen` and related).

Host APIs that are native-only in the registry never get a polyfill here; they error in step 1.

### Emit (`emit.rs`)

Pretty-printer from IR to ECMAScript. Binding kinds print as `let` / `const` / `var` / `using` / `await using` (function bindings as `let`). Control, functions, try/catch/finally, `with`, patterns, calls, optional chaining, templates, `import()` / `import.meta` print as JS.

Documented emit choices in this crate (not a full semantic spec):

- Object / function / class at statement start may be parenthesized so they are not parsed as a block or declaration.
- Direct `eval` stays an Identifier (not `(eval)`).
- `import.defer(…)` is rewritten to `import(…)` for Node hosts; `import.source` is kept.

Semantic equivalence is “where the target can express the program”: JS of the emitted text, plus prepended polyfills, plus host bridges that only exist on Node-like runtimes for those APIs.

### Source maps (`source_map.rs`)

Source Map revision 3. `SourceMapOptions` records original path, optional source content, generated file name, and whether to inline `sourcesContent`. `emit_js_with_map` places one mapping segment at each top-level IR statement. VLQ encode/decode and `source_mapping_url_comment` are public for the CLI.

## Trade-offs

- **Print JS, do not interpret**: the JS engine (or Node) is the observer. Polyfills and Node bridges are extra source, not a second runtime in this crate.
- **Re-check native-only at emit**: matches [[CONTEXT]] even when check ran without `CompileTarget::Js`.
- **Coarse maps**: one segment per top-level statement, not per nested expression.

## Consequences

- A portable program is one both backends accept with equivalent observable behavior after these polyfills ([[CONTEXT]]).
- Native-only Programs must fail here with a diagnostic. They must not emit pointer-shaped JS or skip `extern`.
- Host I/O that is available on both targets is still implemented twice: JS polyfill in [[architecture-runtime]], native lowering in [[architecture-backend-llvm]]. This crate only injects the JS side.
- Dual worlds and GC are native Runtime concerns ([[0003-gc-runtime-and-dual-worlds]]). JS emit does not link that Runtime.
- Pipeline neighbors: [[architecture-pipeline]], [[architecture-frontend]], [[architecture-embed]].
