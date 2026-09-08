---
id: "task-761-llvm-fold-classes"
title: "Fold es_classes into the LLVM walker"
kind: task
status: ready
mode: afk
blocked_by: ["task-760-llvm-fold-functions"]
sprint: "rust-dev-audit"
slice: "slice-756-llvm-one-walker"
tags: []
created_at: "2026-09-09T17:30:00Z"
updated_at: "2026-09-09T17:30:00Z"
---

# Fold es_classes into the LLVM walker

## Blocked by

[[task-760-llvm-fold-functions]]: functions in the walker first.

## Done

Class-builder IR lowers in the shared walker. is_es_classes_module is gone from emit_llvm_ir_raw. Class tests still pass through emit_llvm_ir.

## Context

es_classes.rs matches IIFE class-builder shape. The walker must lower that IR, not fingerprint observation locals. Delete the classifier arm. Do not delete private-method OBS printers here.

## Verify

cargo test -p draconic-backend-llvm --offline. rg is_es_classes_module in lib.rs finds none.

scope: crates/draconic-backend-llvm/src/lib.rs, crates/draconic-backend-llvm/src/es_classes.rs, walker/emitter modules

## Links

[[slice-756-llvm-one-walker]]

## Agent Brief

**Category:** layout
**Summary:** Classes lower in the walker; drop the classes classifier.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: delete the class whole-program adapter
- Contract-first: do not invent language behaviour

**Current behavior:**
is_es_classes_module claims class-builder programs.

**Desired behavior:**
Class IR lowers in the default walker. Native observations unchanged for existing class tests.

**Key interfaces:**
- emit_llvm_ir
- size-file-budget: no new file over 1000

**Acceptance criteria:**
- [ ] no is_es_classes_module in emit_llvm_ir_raw
- [ ] cargo test -p draconic-backend-llvm
- [ ] no new is_* adapter

**Out of scope:**
- Fixture OBS printers (next task)
- host_*
- inkwell/llvm-sys
- Language behaviour or ROADMAP rows

**Explain this part:**
Do not keep try_extract_class as a second lowerer. If a case cannot lower yet, diagnostic — do not add a new adapter.
