---
id: "task-755-llvm-no-extra-globals"
title: "Remove extra LLVM thread-locals and process globals"
kind: task
status: completed
mode: afk
blocked_by: ["task-747-llvm-split-es-builtins", "task-753-llvm-split-es-over-1250", "task-754-llvm-split-over-1000"]
sprint: "rust-dev-audit"
slice: "slice-712-backend-llvm-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T21:30:00Z"
---

# Remove extra LLVM thread-locals and process globals

## Blocked by

[[task-747-llvm-split-es-builtins]]: finish that sitting first.
[[task-753-llvm-split-es-over-1250]]: finish that sitting first.
[[task-754-llvm-split-over-1000]]: finish that sitting first.

## Done

REGEXP_STATICS, FN_REG, and SINK_LINES are gone or are per-call context. TOOLS OnceLock in wasm32_wasi.rs is gone or not process-global. AtomicU64 next_object_id / next_id counters are not process-global. CURRENT_THIS and CURRENT_NEW_TARGET may remain.

## Context

own-thread-local allows linker package context and LLVM JS interp this/new.target only. Pre-split sites: es_builtins.rs REGEXP_STATICS and next_object_id, es_new_target.rs FN_REG and next_id, es_logging.rs SINK_LINES, wasm32_wasi.rs TOOLS OnceLock, es_private_accessors.rs next_id.

## Verify

Slice O2. rg those names; they must not be thread_local!/OnceLock/AtomicU64 process state except CURRENT_THIS and CURRENT_NEW_TARGET.

scope: LLVM files that still hold those names after the splits, plus wasm32_wasi.rs and es_logging.rs

## Links

[[slice-712-backend-llvm-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Drop extra LLVM globals; keep allowed this/new.target thread-locals.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
Extra thread-locals and process-global id caches exist.

**Desired behavior:**
Only allowed thread-locals remain. Emit behaviour unchanged.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] rg REGEXP_STATICS FN_REG SINK_LINES finds no thread_local leftovers
- [x] TOOLS is not a process-global OnceLock
- [x] no process-global AtomicU64 object ids
- [x] CURRENT_THIS and CURRENT_NEW_TARGET may remain
- [x] cargo test -p draconic-backend-llvm

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Removing CURRENT_THIS or CURRENT_NEW_TARGET

**Explain this part:**
If a split already moved the names, edit the new files. Do not re-bloat files over 1000.

## Gauntlet

- **round 1**: `cargo test -p draconic-backend-llvm --offline` — win — 314 passed; rg finds no REGEXP_STATICS/FN_REG/SINK_LINES/OnceLock leftovers
