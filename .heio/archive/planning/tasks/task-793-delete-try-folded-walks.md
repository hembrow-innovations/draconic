---
id: "task-793-delete-try-folded-walks"
title: "Delete try_folded_walks"
kind: task
status: completed
mode: afk
blocked_by: [ "task-792-fold-host-catalog-emit" ]
sprint: "dragons-audit"
slice: "slice-784-llvm-no-fingerprint"
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T23:45:00Z"
---
# Delete try_folded_walks

## Blocked by

[[task-792-fold-host-catalog-emit]]: host classification is not a fingerprint chain.

## Done

`fn try_folded_walks` is gone. `emit_es_expr_walk` is a walker (statement/expression match) plus honest unsupported Diagnostic. Steal-order tests that exist only to protect adapter order are gone. llvm tests green. Slice O1 holds.

## Context

Contract step. Native-ints and empty hello stay in `emit_llvm_ir_raw`. Everything else lowers in one walk or hard-errors. Do not replace the or_else chain with a Vec of function pointers. Do not add is_*_module arms back to lib.rs.

If an ES subset cannot lower in this sitting, it must hard-error, not claim the module. Stop and ticket rather than leave a 20-arm or_else.

## Verify

Slice O1 and O3.

scope: crates/draconic-backend-llvm/src/es_expr.rs, lib.rs steal-order tests, leftover walk_* callers

## Links

[[slice-784-llvm-no-fingerprint]] [[ticket-776-llvm-fingerprint-adapters]]

## Agent Brief

**Category:** layout
**Summary:** Delete the fingerprint or_else chain.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless unsupported Diagnostic text changes
- Purpose: LLVM backend is one lowerer
- Contract-first: do not invent language behaviour

**Current behavior:**
try_folded_walks is ~60 walk_* or_else then leftover classify.

**Desired behavior:**
No try_folded_walks. Walker or Diagnostic. llvm tests green.

**Key interfaces:**
- emit_llvm_ir
- size-file-budget: no new file over 1000

**Acceptance criteria:**
- [x] slice O1 no-fold
- [x] cargo test -p draconic-backend-llvm
- [x] no new walk_* adapter
- [x] no hello-stub success for unsupported IR

**Out of scope:**
- restoring annex-b native (that is [[task-794-annex-b-native-through-walker]])
- inkwell
- file-budget splits

**Explain this part:**
If a subset cannot fold, hard-error and ticket. Do not leave a shorter or_else chain and mark this done.

## Gauntlet

- **round**: 1
- **command**: python3 no-fold check on es_expr.rs; cargo test -p draconic-backend-llvm --offline
- **result**: win (O1 no-fold; 318 passed)
- **gap**: none. Walker matches stmts/exprs then emit or Diagnostic. Steal-order tests gone. Remaining risk: kind select still uses adapter is_* after the walk; annex-b restore is task-794.
