---
id: "architecture-check"
title: "Frontend checker"
kind: architecture
description: "Binder, TypeScript-inspired Checker, dual-worlds boundaries, and host API registry."
domain: draconic
area: frontend
tags: [architecture, frontend, checker]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Frontend checker

## Overview

`draconic-check` binds scopes then typechecks a [[architecture-ast|Program]] (Roadmap B04 / B05, done). The Checker is TypeScript-inspired, not tsc-compatible ([[0005-ts-inspired-not-tsc]]). Native types are unboxed and outside the JS heap ([[0003-gc-runtime-and-dual-worlds]], [[CONTEXT]] Dual worlds). `host_api.rs` registers host I/O names and per-target availability; it does not assign Host API signatures.

## Context

[[CONTEXT]] Checker: static analysis that assigns and validates types. Native-only / JS-only features must hard-error on the other backend, never emit silent wrong code. Script vs Module goal is supplied by the [[architecture-frontend|Frontend]] (`check` vs `check_module`). Target-specific host and `extern` policy is `check_for_target` / `check_module_for_target` when the backend is known.

Roadmap T01–T07.05 are `done`. Do not invent further type-system product rules here; remaining work is whatever [[ROADMAP]] still marks `todo` on other tracks.

## Design

### Crate layout

- [x] **src/lib.rs**: binder, `Type` / `NativeType`, `BoundProgram`, `CheckedProgram`, `check*` entry points, unit tests
- [x] **src/host_api.rs**: `CompileTarget`, `HostAvailability`, `HostApiEntry`, registry, unit tests
- [x] **Cargo.toml**: `draconic-ast`, `draconic-diagnostics`; dev-dep `draconic-parser`

### Public entry points

- **bind / bind_module**: scope analysis + identifier resolution. Module goal: top-level functions are lexical, binder starts strict.
- **check / check_module**: bind then Checker. Script rejects top-level `await` / `for await`. Module allows them (`in_async` at program body); nested non-async functions still reject `await`. No host-target policy.
- **check_for_target / check_module_for_target**: same plus `CompileTarget`. Free host API refs unavailable on that target → `codes::HOST_API_UNSUPPORTED`. `extern "C"` on js → `codes::EXTERN_UNSUPPORTED`.
- **extern_unsupported_on_js_diagnostic**: helper for F08.01.

The Frontend facade calls `check` / `check_module` only. Target-gated check is a library API used by tests today; wiring it at emit time is outside this crate.

### Binder

Private `Binder`: lexical scope stack, var environments (program + function), builtin host globals (dummy span, shadowable), `with_depth`, strict, super-allowed.

`BoundProgram`: original `Program`, `symbols`, use-site span → `SymbolId`. Queries: `resolve`, `symbol`, `use_at_offset`, `decl_at_offset` (LSP hover / go-to-definition).

`Symbol`: id, name, declaration span, `BindingKind`, `with_depth` (identifier uses inside `with` only rewrite to Locals declared in the innermost with body).

Builtins include ECMA globals (`Math`, `Object`, `Promise`, typed arrays, `eval`, …) and stdlib helpers installed as const (`parseFlags`, `sha256`, `gzip`, …). Host APIs from `host_api.rs` are **not** binder builtins: they stay free identifiers (H00).

### Checker types

`Type`: `Number` `BigInt` `String` `Boolean` `Null` `Function` `Object` `Shape(id)` `Union(id)` `Intersection(id)` `TypeParam(id)` `GenericFn(id)` `Native(NativeType)` `Ptr(NativeType)` `Any`.

`NativeType`: `i8`–`i64`, `u8`–`u64`, `f32` `f64`, `bool` (N02; distinct from JS `boolean`).

`CheckedProgram` holds bound program, per-symbol and per-expr types, shape/union/intersection/generic-fn tables, type aliases. `format_type` expands shapes and unions.

Unannotated JS stays permissive (`Any`, inferred object shapes are not `strict`). Annotated shapes are `strict` (T07.03 unknown property, T07.05 excess property). Annotated functions record `FnSig` for arity and argument assignability (T07.01). Missing return in annotated non-void functions is T07.02. Call/`new` of annotated non-callable is T07.04.

### Dual worlds (T06)

Locked in Checker code and conformance `types/dual`, not as extra ADR text:

- JS values vs native types coexist; native stays unboxed ([[0003-gc-runtime-and-dual-worlds]]).
- Explicit boundary is `expr as T` (`Expr::As`). Allowed conversion in `is_dual_world_boundary`: JS `number` ↔ unboxed native numeric (`Native` where `!is_bool`). Other `as` pairs that are not assignable error: `cannot convert type … across dual-worlds boundary`.
- Numeric literals may contextually type as native integers/floats (not native `bool`).
- `as` is erased at IR emit (owned by [[architecture-ir]], not this crate).

Do not document tsc `as` / type-assertion erasure as the product rule.

### Host API module

`host_api.rs` is a name registry, not a typed I/O checker.

- **CompileTarget**: `Js` / `Native`.
- **HostAvailability**: `BOTH` or `NATIVE_ONLY` (no js-only entries in the table).
- **HostApiEntry**: free-identifier `name`, availability, diagnostic `note` (cluster id).
- **lookup / is_host_api / is_available / unsupported_diagnostic**.

Unresolved free refs to registered names type as `Any` unless `host_target` rejects them. Comments on each entry describe intended runtime shape (args, files, sockets, workers, …) for implementers; the Checker does not enforce those signatures.

Native-only examples in the registry: open file handles, UDP, TLS, WebSocket, HTTP/2, signals, shared memory atomics, `processWaitAsync`, TCP async, `workerOsThread`, `makeOnce`. Both-targets examples: `processArgs`, env, cwd, stdio, path, fs (non-handle), TCP/HTTP/1.1 helpers after H17.04, workers/channels, cancel tokens.

`makeMutex` is explicitly not a user Host API (C03.02).

### Tests

- **lib.rs**: bind/check of lets, const reassignment is not a compile reject (E19.60 runtime TypeError), dual-world and T07 diagnostics, `check_for_target` js rejects `extern`.
- **host_api.rs**: registry availability, js hard-error for native-only, free identifier H00, shadowed host names, `check_for_target` for TCP/HTTP/shared memory.

Conformance fixtures under `tests/conformance/types` lock T01–T07.05 and dual-world behavior end-to-end; those files are not this crate.

## Trade-offs

- **Not tsc**: no `tsconfig`, no declaration emit, no type-erasure-only JS. Native types and dual worlds need a real Checker ([[0005-ts-inspired-not-tsc]]).
- **Untyped JS permissive**: annotations opt into T07. Unannotated programs typecheck like dynamic JS.
- **Host APIs untyped**: availability is the compile-time gate; arity/shape of `readFileText` etc. is runtime / later work, not invented here.
- **Fail fast**: first `Diagnostic`, same as parse.

## Consequences

[[architecture-frontend]] must pass Module goal into `check_module` so top-level await matches parse. Lowering ([[architecture-ir]]) consumes `CheckedProgram`. Dual-world `as` must remain a type-level boundary, not a JS `as` keyword with runtime meaning. If a new type feature is needed, it belongs on [[ROADMAP]] first.
