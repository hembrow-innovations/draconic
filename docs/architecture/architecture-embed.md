---
id: "architecture-embed"
title: "Embed"
kind: architecture
description: "Compiler-in-runtime for eval and new Function: Frontend to IR interpreter, fold-at-emit, resource limits, fuzz."
domain: draconic
area: runtime
tags: [runtime]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Embed

## Overview

Embed is enough of the Frontend and Compiler shipped so `eval`, `new Function`, and similar can compile source at run time on the native target. The crate is `draconic-embed` (`lib.rs`, `fold.rs`, `fuzz.rs`). It is not the C Runtime heap. Native binaries link [[architecture-runtime]]; LLVM constant-string `eval` / `Function` fold through this crate at emit. Glossary: [[CONTEXT]]. Decision: [[0004-full-ecma-262-and-embed]]. Pipeline: [[architecture-pipeline]]. LLVM consumer: [[architecture-backend-llvm]]. CLI REPL can call `eval_source` ([[architecture-cli]]).

## Context

[[0004-full-ecma-262-and-embed]] sets the destination as literally all of ECMA-262, including `eval`, `new Function`, and `with`. JS-backend-only eval and permanent omission were rejected. Roadmap **N07** is the designed native subset: compile simple expression strings through Frontend → IR and interpret them; fold constant-string `eval` / `Function` in the outer Program so LLVM prints real observations.

Resource limits (**R01**) fail closed with a diagnostic, not a catchable JS exception ([[0011-catchable-exceptions-vs-abort]]). Fuzz entry is **R05.02**.

## Design

### `lib.rs` — eval interpreter

`eval_source` compiles a Script via `compile_source` (Frontend) and interprets the IR module’s completion value.

Supported **N07.01** subset:

- **Literals**: number, string, boolean, null; `undefined` as a local name.
- **Arithmetic**: `+` `-` `*` `/` `%` (string `+` concatenates).
- **Unary**: `+` `-` `typeof` `void` `!`.
- **Grouping** and comma.
- **Statements**: expression statements, blocks, `let` declare with optional init.

Unsupported statements and expressions return a diagnostic (`embed eval does not support …`).

Public values: `EmbedValue` (`Undefined`, `Null`, `Boolean`, `Number`, `String`).

Other entry points:

- **`eval_source_with_bindings`**: prepend `let` bindings for free names (**N07.04** direct lexical vs indirect global; caller merges shadowing).
- **`eval_function_call`**: `Function` body as `return <expr>;` with positional args (**N07.03**). Missing args are `undefined`.
- **Limits**: `MAX_EVAL_SOURCE_BYTES` is 1 MiB (checked before parse). `DEFAULT_EVAL_ALLOC_BUDGET_BYTES` is 16 MiB of newly allocated strings (`0` = unlimited). Wall-clock time budget (`Duration::ZERO` = unlimited). Combined: `eval_source_with_limits`.

Exhaustion is a `Diagnostic`, not a JS value. The C Runtime also exposes eval time-budget flags for native fail-closed checks ([[system-design-gc-runtime]]).

### `fold.rs` — outer Program fold-at-emit

`fold_eval_program` interprets an IR module that uses `eval` / `Function` and returns `Observation` values (`Number`, `String`, `Bool`, `Function`) for top-level user `let` bindings in declaration order. `is_eval_fold_module` is true when that fold succeeds with a non-empty observation list.

Fold-only tags (not `EmbedValue`): builtin `eval`, builtin `Function`, `globalThis`, user functions, dynamic `Function` objects. Direct eval injects caller lexical bindings; indirect eval (`(0, eval)(s)` / `globalThis.eval(s)`) uses global object properties. LLVM only prints the observations ([[architecture-backend-llvm]]).

### `fuzz.rs` — `fuzz_eval`

Treats bytes as source (lossy UTF-8), runs `eval_source` and `eval_function_call`. Diagnostics discarded; the contract is no panic (**R05.02**). Distinct from Runtime `fuzz_runtime`.

## Trade-offs

An IR interpreter plus fold-at-emit gives native observations for the **N07** fixtures without shipping a full compiler inside `draconic_rt.c`. That is smaller and fail-closed under **R01**, but it is not yet the ADR destination of compiling arbitrary `eval` strings inside a running native process. Ownership-only eval (no Frontend) was not used; Embed always goes Frontend → IR.

## Consequences

JS-backend `eval` remains host JavaScript. Native constant-string `eval` / `new Function` must stay inside the fold subset or LLVM will not take this path. Dynamic or unsupported source fails with an Embed diagnostic (fail closed), not a catchable exception. Remaining full ECMA `eval` / `with` / object and function scripts is the [[0004-full-ecma-262-and-embed]] destination and further ECMA/Roadmap work — do not treat the **N07** interpreter as a complete compiler-in-Runtime.
