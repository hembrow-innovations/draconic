---
id: "task-763-llvm-contract-dispatch"
title: "Contract LLVM emit_llvm_ir_raw to one walker"
kind: task
status: completed
mode: afk
blocked_by: ["task-762-llvm-delete-fixture-printers", "task-764-llvm-fold-host-fs"]
sprint: "rust-dev-audit"
slice: "slice-756-llvm-one-walker"
tags: []
created_at: "2026-09-09T17:30:00Z"
updated_at: "2026-09-09T23:59:00Z"
---

# Contract LLVM emit_llvm_ir_raw to one walker

## Blocked by

[[task-762-llvm-delete-fixture-printers]]: printers gone.
[[task-764-llvm-fold-host-fs]]: fs in the walker.

## Done

emit_llvm_ir_raw has at most native-ints, one walker, and empty hello. Remaining is_* adapters are either folded into the walker in this sitting or deleted with tests still green through emit_llvm_ir. Slice O1 walker-dispatch holds.

## Context

Contract step. Fold leftover es_* / host_* arms into the walker or drop them if unused. Do not split leftover adapter files to meet 1000 lines — that is [[slice-712-backend-llvm-file-budget]] after this task.

## Verify

Slice O1 python walker-dispatch. Slice O3 cargo test -p draconic-backend-llvm --offline.

scope: crates/draconic-backend-llvm/src/lib.rs emit_llvm_ir_raw and remaining adapter modules required to clear the cascade

## Links

[[slice-756-llvm-one-walker]] [[slice-712-backend-llvm-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Delete the is_* cascade; one walker remains.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: LLVM backend is one lowerer
- Contract-first: do not invent language behaviour

**Current behavior:**
Dozens of is_*_module arms remain after earlier folds.

**Desired behavior:**
≤4 _module( calls inside emit_llvm_ir_raw. Native tests green.

**Key interfaces:**
- emit_llvm_ir
- size-file-budget: no new file over 1000

**Acceptance criteria:**
- [x] slice O1 walker-dispatch
- [x] cargo test -p draconic-backend-llvm
- [x] no new is_* adapter

**Out of scope:**
- File-budget splits of the walker (slice-712)
- inkwell/llvm-sys
- Language behaviour or ROADMAP rows

**Explain this part:**
If an adapter cannot fold in one sitting, stop and file a ticket. Do not leave a 20-arm cascade and mark this done.

## Gauntlet

- **round**: 1
- **command**: python O1 walker-dispatch; cargo test -p draconic-backend-llvm --offline; git diff has no new is_* adapter
- **result**: win
- **gap**: none
