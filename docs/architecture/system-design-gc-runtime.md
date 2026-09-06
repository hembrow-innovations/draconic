---
id: "system-design-gc-runtime"
title: "GC Runtime heap"
kind: system-design
description: "Mark-sweep GC for JS values, root stack, alloc budgets, and how jobs and Promises sit on the heap."
status: draft
domain: draconic
area: runtime
tags: [runtime]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# GC Runtime heap

## Overview

This design is the tracing GC inside `draconic_rt.c`, linked with native binaries ([[architecture-runtime]]). It owns JS values (strings, objects, arrays, Promises). Native types stay unboxed ([[architecture-dual-worlds]]). Decision: [[0003-gc-runtime-and-dual-worlds]]. Fail-closed abort vs catchable exceptions: [[0011-catchable-exceptions-vs-abort]]. Embed alloc/time budgets: [[architecture-embed]].

## Components

- **Heap list**: intrusive `DraconicValue *next` from `g_heap_head`. Tags: `STRING`, `OBJECT`, `PROMISE`, `ARRAY`.
- **Root stack**: growable (`ROOT_STACK_INITIAL` 64, doubles). `gc_root_push` / `gc_root_pop`. Grow failure and pop underflow print `draconic_rt: root stack grow failed` / `root stack underflow` and abort. Tests: `abort_policy_tests.rs`.
- **Mark-sweep collect**: mark from roots (object props plus Prototype internal slot, promise result plus derived reactions, array elements). Sweep unmarked. `is_heap_value` ignores inttoptr numbers stored in slots.
- **Alloc threshold (N09.05)**: default 1024 live objects; `0` disables. Alloc path collects before growing past the threshold.
- **Alloc budget (R01.02)**: `gc_set_alloc_budget`; `0` = unlimited. Exceeding collects once then returns `NULL` — not a JS exception. Tests: `gc_alloc_budget_tests.rs`, `r01_resource_limits_tests.rs`.
- **Eval time budget (R01.03)**: `eval_set_time_budget_ms` / `eval_time_begin` / `eval_time_exceeded`. Fail-closed flag. Tests: `eval_time_budget_tests.rs`.
- **Job queue**: FIFO `DraconicJob` list in the same C file; not GC-managed. Promise reactions enqueue jobs. Drain promotes timers, signals, process waits, and host IO ([[architecture-runtime]]).
- **Abort**: `draconic_rt_abort` → stderr `draconic_rt: abort`, backtrace (**R06**), `abort()`.

## Interfaces

LLVM declares shapes from `abi.rs` (`GC_INIT`, `ALLOC_STRING`, `ALLOC_OBJECT`, `GC_ROOT_PUSH` / `POP`, `GC_COLLECT`, `GC_LIVE_COUNT`, threshold/budget, `JOB_*`, `PROMISE_*`, `ARRAY_*`, `OBJECT_*`). Headers: `draconic_rt.h` (GC/jobs/Promises) includes `draconic_rt_host.h` (host I/O). The [[architecture-backend-llvm|LLVM backend]] emits calls; the [[architecture-backend-js|JS backend]] does not.

## Data model

A `DraconicValue` is a tagged union:

- **String**: WTF-8 bytes (UTF-8 plus unpaired surrogates); JS `.length` is UTF-16 code units via `utf16_len`.
- **Object**: linked own props (string key or symbol id) plus nullable Prototype internal slot.
- **Promise**: pending/fulfilled/rejected, opaque `result`, reaction list (including `finally` mode).
- **Array**: `void **` elements (GC pointer or inttoptr).

Unboxed native `i32` / structs / pointers are not in this union. JS `number` on native is an LLVM `double`, not a heap tag, unless stored as an object/array slot ([[architecture-dual-worlds]]).

## Interactions

1. Program starts: `gc_init`.
2. Allocate string/object/array/promise → maybe auto-collect → maybe `NULL` if budget exceeded.
3. Live JS values the backend still needs are `root_push`ed; drop with `root_pop`.
4. `promise_then` / host async (TCP, process wait) enqueue jobs; `job_drain` runs them until empty, waiting on timers/IO rather than spinning.
5. Shutdown frees the heap list and root stack.

Stress and mark/cycle/root-growth tests live in `lib.rs` (**N09.01–N09.05**), not separate files.

## Error handling

- **Catchable**: JS `throw` of a heap or primitive JS value. Not implemented as C unwind in this Runtime; policy is [[0011-catchable-exceptions-vs-abort]].
- **Fail closed, not catchable**: alloc budget `NULL`; eval time exceeded flag; Embed diagnostics ([[architecture-embed]]).
- **Abort**: invariant failure and `draconic_rt_abort`. After abort the process is dead; `try`/`catch` does not run.

Host I/O errors are integer codes, not GC objects. Native Programs print `host_error_name` tokens and exit 1; JS throws catchable `HostError`.

## Trade-offs

Mark-sweep with an explicit root stack is simple for LLVM to call and can collect cycles (**N09.03**). It requires every live heap pointer to be rooted; missing roots look like leaks after collect. Returning `NULL` on OOM (rather than throwing) matches [[0011-catchable-exceptions-vs-abort]] — GC/OOM is not a JS exception in v1. Alternative rejected: arena-only (no cyclic JS graphs) and treating every native fault as abort (host `EPERM` / `ENOENT` must remain Program-visible codes).
