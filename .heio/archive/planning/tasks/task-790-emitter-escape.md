---
id: "task-790-emitter-escape"
title: "One LLVM escape_llvm_string"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "dragons-audit"
slice: "slice-783-llvm-one-emitter"
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T19:30:00Z"
---

# One LLVM escape_llvm_string

## Blocked by

None.

## Done

`fn escape_llvm_string` exists only in `emitter.rs`. Adapters call that helper. Slice O1 holds. llvm tests green.

## Context

About 37 copies. `emitter.rs` already has `escape_llvm_bytes`. Make the string helper crate-private there and delete the copies. Do not change emit behavior. Do not split files to meet 1000 lines. If a file would exceed 1000 after inlining, call the helper; do not add a nested module.

## Verify

Slice O1 and O2.

scope: crates/draconic-backend-llvm/src/emitter.rs and adapter files that define escape_llvm_string

## Links

[[slice-783-llvm-one-emitter]] [[ticket-777-llvm-file-budget-shards]] [[ticket-801-llvm-escape-variants]]

## Agent Brief

**Category:** prefactor
**Summary:** Delete duplicate LLVM string escapes.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none
- Purpose: one escape helper
- Contract-first: do not invent language behaviour

**Current behavior:**
Many private `fn escape_llvm_string` copies.

**Desired behavior:**
One definition in emitter.rs. Tests through emit_llvm_ir stay green.

**Key interfaces:**
- crate-private emitter helpers
- size-file-budget: no new file over 1000; no new classify.rs

**Acceptance criteria:**
- [x] slice O1 one-escape
- [x] cargo test -p draconic-backend-llvm
- [x] no new walk_* adapter

**Out of scope:**
- deleting try_folded_walks
- SlotTy unify (that is [[task-791-emitter-slotty-migrate]])
- inkwell

**Explain this part:**
Behavior-identical. If a copy differs, stop and file a ticket. Do not pick a random variant.

Round 2 re-check: six source texts still differ, but all 256 byte values match `escape_llvm_bytes` in `emitter.rs`. Canonical body is that existing helper wrapped as `escape_llvm_string`. Closed [[ticket-801-llvm-escape-variants]].

## Gauntlet

- **round**: 1
- **command**: hashed every `fn escape_llvm_string` body under `crates/draconic-backend-llvm/src`
- **result**: lose
- **gap**: six source variants; cannot unify without a chosen canonical body
- **round**: 2
- **command**: slice O1 python one-escape; cargo test -p draconic-backend-llvm --offline
- **result**: win
- **gap**: none. O1 one-escape. O2 test result: ok. 315 passed.
