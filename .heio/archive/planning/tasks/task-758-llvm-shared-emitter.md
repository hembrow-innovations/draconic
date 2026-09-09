---
id: "task-758-llvm-shared-emitter"
title: "Share LLVM SlotTy and Emitter"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-756-llvm-one-walker"
tags: []
created_at: "2026-09-09T17:30:00Z"
updated_at: "2026-09-09T19:30:00Z"
---

# Share LLVM SlotTy and Emitter

## Blocked by

None.

## Done

One crate-private SlotTy and Emitter live in a sibling module. es_expr.rs and es_functions.rs use it. Dispatch in emit_llvm_ir_raw is unchanged. New files ≤1000.

## Context

Dragons audit expand step. `enum SlotTy` is copied in 35 LLVM files. Extract the shared shape es_expr and es_functions already agree on. Do not change is_*_module order. Do not split other adapters in this sitting.

## Verify

cargo test -p draconic-backend-llvm --offline. rg "enum SlotTy" in es_expr.rs and es_functions.rs finds none (they use the shared type). New sibling ≤1000.

scope: crates/draconic-backend-llvm/src/es_expr.rs, crates/draconic-backend-llvm/src/es_functions.rs, new sibling those files declare, crates/draconic-backend-llvm/src/lib.rs only for `mod`

## Links

[[slice-756-llvm-one-walker]]

## Agent Brief

**Category:** layout
**Summary:** One SlotTy/Emitter for es_expr and es_functions.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: deepen LLVM lowerer; stop copying SlotTy
- Contract-first: do not invent language behaviour

**Current behavior:**
Each adapter defines its own SlotTy and Emitter.

**Desired behavior:**
es_expr and es_functions share one private implementation. emit_llvm_ir behaviour unchanged.

**Key interfaces:**
- crate-private module, not a new public emit entry
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] shared SlotTy/Emitter module exists
- [x] es_expr.rs and es_functions.rs do not define enum SlotTy
- [x] emit_llvm_ir_raw still has the existing is_* cascade
- [x] cargo test -p draconic-backend-llvm
- [x] no new file over 1000

**Out of scope:**
- Deleting is_* adapters
- Editing host_* files
- inkwell/llvm-sys
- Language behaviour or ROADMAP rows

**Explain this part:**
This is expand. Later tasks migrate adapters onto this emitter. Do not rename emit_llvm_ir.

## Gauntlet

- **round**: 1
- **command**: cargo test -p draconic-backend-llvm --offline; rg "enum SlotTy" es_expr.rs es_functions.rs; wc -l emitter.rs; emit_llvm_ir_raw is_* cascade
- **result**: win
- **gap**: none
