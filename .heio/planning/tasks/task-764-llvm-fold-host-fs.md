---
id: "task-764-llvm-fold-host-fs"
title: "Fold host_fs into the LLVM walker"
kind: task
status: ready
mode: afk
blocked_by: ["task-759-llvm-default-walker", "task-767-llvm-host-from-catalog"]
sprint: "rust-dev-audit"
slice: "slice-756-llvm-one-walker"
tags: []
created_at: "2026-09-09T17:30:00Z"
updated_at: "2026-09-09T17:30:00Z"
---

# Fold host_fs into the LLVM walker

## Blocked by

[[task-759-llvm-default-walker]]: default walker first.
[[task-767-llvm-host-from-catalog]]: host names come from the catalog.

## Done

readFileText and other host_fs call-sites lower in the walker as Runtime ABI calls. is_host_fs_module is gone from emit_llvm_ir_raw. host_fs tests still pass through emit_llvm_ir.

## Context

host_fs.rs is a second IR interpreter that claims whole programs. After the walker exists and catalog names are canonical, emit HOST_* calls from ordinary Call nodes. Do not fold every host_* adapter in this sitting — fs only.

## Verify

cargo test -p draconic-backend-llvm --offline. rg is_host_fs_module in lib.rs finds none.

scope: crates/draconic-backend-llvm/src/lib.rs, crates/draconic-backend-llvm/src/host_fs.rs, walker/emitter modules

## Links

[[slice-756-llvm-one-walker]] [[slice-757-host-catalog]]

## Agent Brief

**Category:** layout
**Summary:** FS host calls lower in the walker; drop the fs classifier.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: host I/O is call-sites, not a program-shape adapter
- Contract-first: do not invent language behaviour

**Current behavior:**
is_host_fs_module claims mixed programs before ES adapters.

**Desired behavior:**
FS names from the catalog lower as ABI calls in the walker.

**Key interfaces:**
- emit_llvm_ir
- Runtime AbiFn / catalog names
- size-file-budget: no new file over 1000

**Acceptance criteria:**
- [ ] no is_host_fs_module in emit_llvm_ir_raw
- [ ] cargo test -p draconic-backend-llvm
- [ ] no new is_* adapter

**Out of scope:**
- Other host_* adapters (tcp, http, …)
- Rewriting draconic_rt_host.c
- inkwell/llvm-sys
- Language behaviour or ROADMAP rows

**Explain this part:**
Leave other host_* arms in place. Contract-dispatch waits until fs is in the walker.

## Blocked

`cargo test -p draconic-backend-llvm --offline` cannot compile. In-flight [[task-760-llvm-fold-functions]] leaves `es_functions.rs` with `by_id` unbound in `collect_free_in_expr`. This sitting must not edit `es_functions.rs`. Fold is in the working tree (walker `walk_host_fs`, `is_host_fs_module` gone from `emit_llvm_ir_raw`) but unverified.

## Gauntlet

- **round**: 1
- **command**: cargo test -p draconic-backend-llvm --offline
- **result**: lose
- **gap**: crate does not compile; `es_functions.rs` `by_id` not in scope (task-760 uncommitted)
