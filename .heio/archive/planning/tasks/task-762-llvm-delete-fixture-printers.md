---
id: "task-762-llvm-delete-fixture-printers"
title: "Delete LLVM fixture OBS printers"
kind: task
status: completed
mode: afk
blocked_by: ["task-761-llvm-fold-classes"]
sprint: "rust-dev-audit"
slice: "slice-756-llvm-one-walker"
tags: []
created_at: "2026-09-09T17:30:00Z"
updated_at: "2026-09-09T23:30:00Z"
---

# Delete LLVM fixture OBS printers

## Blocked by

[[task-761-llvm-fold-classes]]: class IR in the walker first.

## Done

No LLVM src file contains `const OBS`. es_private_methods, es_async_methods, and es_static_private_fields classifiers are gone. Those programs lower through the walker or hard-error honestly. Tests through emit_llvm_ir stay green or the sitting stops and files a ticket.

## Context

These adapters fingerprint fixture locals and print hardcoded numbers. Conformance can go green without a lowerer. After classes are in the walker, delete the printers. Do not replace them with a new is_* adapter.

## Verify

Slice O2 python no-obs. cargo test -p draconic-backend-llvm --offline.

scope: crates/draconic-backend-llvm/src/es_private_methods.rs, es_async_methods.rs, es_static_private_fields.rs, lib.rs dispatch, walker

## Links

[[slice-756-llvm-one-walker]]

## Agent Brief

**Category:** layout
**Summary:** Remove hardcoded OBS native printers.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: native backend must lower or fail, not print fixture tables
- Contract-first: do not invent language behaviour

**Current behavior:**
Named adapters match Point/Greeter/__drac_pm_* and print OBS.

**Desired behavior:**
No const OBS. Programs run through the walker or diagnostic.

**Key interfaces:**
- emit_llvm_ir
- size-file-budget

**Acceptance criteria:**
- [x] python no-obs check from the slice
- [x] those is_*_module arms gone from emit_llvm_ir_raw
- [x] cargo test -p draconic-backend-llvm
- [x] if tests cannot pass without OBS, stop; do not keep the printer; file a ticket

**Out of scope:**
- host_*
- inkwell/llvm-sys
- Language behaviour or ROADMAP rows

**Explain this part:**
Honest unsupported is better than a green OBS table. Do not add a new fingerprint adapter.

## Gauntlet

- **round**: 1
- **command**: python no-obs; cargo test -p draconic-backend-llvm --offline; rg is_es_private_methods_module|is_es_async_methods_module|is_es_static_private_fields_module crates/draconic-backend-llvm/src/lib.rs
- **result**: win
- **gap**: none
