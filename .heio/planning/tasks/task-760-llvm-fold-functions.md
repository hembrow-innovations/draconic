---
id: "task-760-llvm-fold-functions"
title: "Fold es_functions into the LLVM walker"
kind: task
status: ready
mode: afk
blocked_by: ["task-759-llvm-default-walker"]
sprint: "rust-dev-audit"
slice: "slice-756-llvm-one-walker"
tags: []
created_at: "2026-09-09T17:30:00Z"
updated_at: "2026-09-09T17:30:00Z"
---

# Fold es_functions into the LLVM walker

## Blocked by

[[task-759-llvm-default-walker]]: default walker first.

## Done

Function decl/expr/arrow/return/call lower in the shared walker. is_es_functions_module is gone from emit_llvm_ir_raw. Existing function tests still pass through emit_llvm_ir.

## Context

Migrate, then delete the functions adapter arm. Keep behaviour. If es_functions.rs becomes a thin re-export, delete it. Do not fold classes or host in this sitting.

## Verify

cargo test -p draconic-backend-llvm --offline. rg is_es_functions_module in lib.rs finds none.

scope: crates/draconic-backend-llvm/src/lib.rs, crates/draconic-backend-llvm/src/es_functions.rs, walker/emitter modules

## Links

[[slice-756-llvm-one-walker]]

## Agent Brief

**Category:** layout
**Summary:** Functions lower in the walker; drop the functions classifier.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: delete one whole-program adapter
- Contract-first: do not invent language behaviour

**Current behavior:**
is_es_functions_module claims whole programs.

**Desired behavior:**
Function nodes lower in the default walker. Tests through emit_llvm_ir stay green.

**Key interfaces:**
- emit_llvm_ir
- size-file-budget: no new file over 1000; shrinking es_functions.rs may delete it

**Acceptance criteria:**
- [ ] no is_es_functions_module in emit_llvm_ir_raw
- [ ] cargo test -p draconic-backend-llvm
- [ ] no new is_* adapter

**Out of scope:**
- Classes, host I/O, fixture printers
- inkwell/llvm-sys
- Language behaviour or ROADMAP rows

**Explain this part:**
If a nested function case only the old adapter handled, teach the walker. Do not keep a parallel functions lowerer.
