---
id: "task-754-llvm-split-over-1000"
title: "Split llvm files between 1001 and 1250"
kind: task
status: completed
mode: afk
blocked_by: ["task-763-llvm-contract-dispatch"]
sprint: "rust-dev-audit"
slice: "slice-712-backend-llvm-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T20:00:00Z"
---

# Split llvm files between 1001 and 1250

## Blocked by

[[task-763-llvm-contract-dispatch]]: one walker before splitting leftover LLVM files.

## Done

Each of es_testing.rs, es_values.rs, es_collections.rs, host_stdio.rs, host_process_async.rs, host_channels.rs, es_new_target.rs, host_process.rs, host_http.rs, es_modules.rs, host_http2.rs is ≤1000, or replaced by sibling files each ≤1000. Public emit entry unchanged.

## Context

size-file-budget on es_testing.rs, es_values.rs, es_collections.rs, host_stdio.rs, host_process_async.rs, host_channels.rs, es_new_target.rs, host_process.rs, host_http.rs, es_modules.rs, host_http2.rs. Split by a deeper feature seam inside each file. Move same-file tests with the seam. Do not edit other oversized LLVM files in this sitting.

## Verify

cargo test -p draconic-backend-llvm. Named files and new siblings ≤1000.

scope: crates/draconic-backend-llvm/src/es_testing.rs, crates/draconic-backend-llvm/src/es_values.rs, crates/draconic-backend-llvm/src/es_collections.rs, crates/draconic-backend-llvm/src/host_stdio.rs, crates/draconic-backend-llvm/src/host_process_async.rs, crates/draconic-backend-llvm/src/host_channels.rs, crates/draconic-backend-llvm/src/es_new_target.rs, crates/draconic-backend-llvm/src/host_process.rs, crates/draconic-backend-llvm/src/host_http.rs, crates/draconic-backend-llvm/src/es_modules.rs, crates/draconic-backend-llvm/src/host_http2.rs and new sibling modules those files declare

## Links

[[slice-712-backend-llvm-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split es_testing.rs, es_values.rs, es_collections.rs, host_stdio.rs, host_process_async.rs, host_channels.rs, es_new_target.rs, host_process.rs, host_http.rs, es_modules.rs, host_http2.rs under 1000 lines.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
es_testing.rs, es_values.rs, es_collections.rs, host_stdio.rs, host_process_async.rs, host_channels.rs, es_new_target.rs, host_process.rs, host_http.rs, es_modules.rs, host_http2.rs over the line budget.

**Desired behavior:**
Named files ≤1000. No new file over 1250.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] each named file ≤1000 or gone after split
- [x] no new sibling over 1000
- [x] cargo test -p draconic-backend-llvm
- [x] no inkwell/llvm-sys

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Editing LLVM files not in this task scope

**Explain this part:**
LLVM JS interp CURRENT_THIS / CURRENT_NEW_TARGET may stay. Do not add process-global state.
