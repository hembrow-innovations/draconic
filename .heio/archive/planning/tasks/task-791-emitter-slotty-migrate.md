---
id: "task-791-emitter-slotty-migrate"
title: "Adapters use crate SlotTy and Emitter"
kind: task
status: completed
mode: afk
blocked_by: ["task-790-emitter-escape"]
sprint: "dragons-audit"
slice: "slice-783-llvm-one-emitter"
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T20:30:00Z"
---

# Adapters use crate SlotTy and Emitter

## Blocked by

[[task-790-emitter-escape]]: one escape helper first.

## Done

Adapters whose slots are only Number / BigInt / Boolean / String / Undefined use `crate::emitter::{Emitter, SlotTy}`. Private duplicate enums for that set are gone. Host Handle/Stat extras may remain as extensions on one enum or a local wrapper, not 20 copies of Number/String. llvm tests green.

## Context

~35 private `enum SlotTy`. Shared `emitter.rs` SlotTy is the scalar set. es_expr and es_functions already use it. Migrate the rest that fit. Do not fold try_folded_walks here. Do not split classify/ok/eval.

If host Handle cannot fit without a large type change, extend crate SlotTy once and stop. Do not invent a second generic emitter.

## Verify

Slice O2. Count of `enum SlotTy` under llvm src is 1, or 1 plus documented host extension in emitter.rs only.

scope: crates/draconic-backend-llvm/src/emitter.rs and adapter SlotTy/Emitter definitions

## Links

[[slice-783-llvm-one-emitter]] [[task-790-emitter-escape]] [[slice-784-llvm-no-fingerprint]]

## Agent Brief

**Category:** prefactor
**Summary:** One SlotTy and Emitter for scalar adapters.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none
- Purpose: shared LLVM text helpers
- Contract-first: do not invent language behaviour

**Current behavior:**
Each adapter defines SlotTy and often Emitter.

**Desired behavior:**
Scalar adapters use crate types. llvm tests green.

**Key interfaces:**
- crate::emitter::{Emitter, SlotTy}
- size-file-budget: no new file over 1000

**Acceptance criteria:**
- [x] cargo test -p draconic-backend-llvm
- [x] no new nested classify.rs
- [x] try_folded_walks still present (not this sitting)

**Out of scope:**
- deleting the fingerprint chain
- inkwell
- host catalog emit (that is [[task-792-fold-host-catalog-emit]])

**Explain this part:**
If host Handle needs a crate SlotTy variant, add it once on emitter.rs. Do not leave 20 Handle copies.

Handle/Stat/Object extras stayed off crate SlotTy (Copy scalar set; `Object(HashMap)` and `Bytes(usize)` are a large type change). Mixed adapters use a private `LocalSlot` wrapper. One `enum SlotTy` in `emitter.rs`.

## Gauntlet

- **round**: 1
- **command**: python count `enum SlotTy`; cargo test -p draconic-backend-llvm --offline
- **result**: win
- **gap**: none. enum SlotTy count 1. O2 test result: ok. 315 passed. try_folded_walks present. no new classify.rs.
