---
id: api-embed
title: Embed eval API
kind: api
description: Public eval surface of crates/draconic-embed (eval, Function, fold, fuzz). Not an HTTP API.
domain: draconic
area: api
tags: [api, embed]
source: crates/draconic-embed/src/lib.rs
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Embed eval API

## Overview

Embed is the compiler-in-runtime path for `eval` / `Function` on native ([[CONTEXT]], [[architecture-embed]]). The public Rust surface is `crates/draconic-embed`: compile a source string through Frontend → IR, then a minimal IR interpreter. Outer-program fold-at-emit lives in the same crate (`fold_eval_program`). The CLI `repl --target embed` calls `eval_source` ([[api-cli]]). This is not OpenAPI and not a JS host `eval` polyfill.

Limits (R01): max source size, alloc budget, time budget. Exhaustion fails closed with a `Diagnostic`, not a catchable JS exception ([[0011-catchable-exceptions-vs-abort]], [[security]]).

## Endpoints

Constants:

- **`MAX_EVAL_SOURCE_BYTES`**: 1_048_576 (1 MiB). Checked before compile.
- **`DEFAULT_EVAL_ALLOC_BUDGET_BYTES`**: 16 MiB of newly allocated strings. `0` means unlimited.

Types:

- **`EmbedValue`**: `Undefined`, `Null`, `Boolean`, `Number`, `String`. Helpers: `as_number`, `as_str`, `typeof_name`.
- **`Observation`**: fold-at-emit stdout-shaped values (`Number`, `String`, `Bool`, `Function`).

Eval:

- **`eval_source(source)`**: compile as Script; return completion value. Uses the default alloc budget and unlimited time.
- **`eval_source_with_alloc_budget(source, budget_bytes)`**: same, explicit alloc budget (`0` unlimited).
- **`eval_source_with_time_budget(source, budget)`**: same, wall-clock budget (`Duration::ZERO` unlimited).
- **`eval_source_with_limits(source, budget_bytes, time_budget)`**: combined R01 limits. Oversize source rejected before compile.
- **`eval_source_with_bindings(source, bindings)`**: prepend `let` bindings for free names (direct / indirect eval). Duplicate names error.
- **`eval_function_call(params, body, args)`**: evaluate a `Function` body (`return expr` subset). Missing args are `undefined`; extra args ignored. Body size checked against `MAX_EVAL_SOURCE_BYTES`.

Fold / fuzz:

- **`fold_eval_program(module)`**: fold an IR module that uses `eval` / `Function`; observations for top-level `let` bindings.
- **`is_eval_fold_module(module)`**: true when the supported eval/Function subset folds to a non-empty observation list.
- **`fuzz_eval(data)`**: public fuzz hook. Lossy UTF-8; `eval_source` and `eval_function_call`; diagnostics discarded; contract is no panic.

Supported eval subset today: literals, arithmetic, unary `+/-`, grouping, `typeof`, string concat, comma, `void`, `!`, completion of last expression. Unsupported statements/expressions return a diagnostic (`embed eval does not support …`).

## Auth

None. Callers are in-process Rust (Runtime, CLI repl embed target, tests). No grants API on this crate. Host I/O grants belong to `draconic run` ([[api-cli]], [[0008-host-io-sockets-first-http]]).

## Errors

All fallible eval functions return `Result<_, Diagnostic>`.

- **Oversize source**: message includes `maximum source size` and the byte cap.
- **Alloc budget exceeded**: `embed eval: alloc budget exceeded`.
- **Time budget exceeded**: `embed eval: time budget exceeded`.
- **Duplicate / invalid binding or parameter names**: diagnostic; Function param names are ASCII identifier-shaped.
- **Unsupported IR**: `embed eval does not support statement/expression/op`.
- **Unbound local**: `embed eval: unbound local`.

These diagnostics are fail-closed resource or subset errors, not catchable JS exceptions ([[0011-catchable-exceptions-vs-abort]]). `fuzz_eval` swallows `Err` and must not panic.
