---
id: "task-796-type-world-methods"
title: "Add Type world predicates"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "dragons-audit"
slice: "slice-786-type-world-predicates"
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-10T05:30:00Z"
---

# Add Type world predicates

## Blocked by

None.

## Done

`Type` exposes `is_js_value`, `is_native_world`, and keeps or wraps `is_dual_world_boundary`. Unit tests on Type cover number vs i32 vs ptr. Checker call sites may still copy matches until [[task-797-type-world-checker-use]]. Slice O1 holds.

## Context

`types.rs` already has dual-world helpers nearby in checker_type. Put the predicates on `Type` so Checker and IR readers share them. Do not hide Type from IR. Do not split types.rs for file budget if it stays under 1000.

## Verify

Slice O1. `cargo test -p draconic-check --offline` ok.

scope: crates/draconic-check/src/types.rs and existing is_dual_world_boundary home

## Links

[[slice-786-type-world-predicates]] [[ticket-779-type-world-predicates]]

## Agent Brief

**Category:** types
**Summary:** World tests on Type. Do not rewrite check_expr yet.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none
- Purpose: dual worlds at the type seam
- Contract-first: do not invent new conversions

**Current behavior:**
Predicates are scattered; Type is a mixed enum.

**Desired behavior:**
Methods on Type. Check tests green.

**Key interfaces:**
- Type
- size-file-budget: no new file over 1000

**Acceptance criteria:**
- [ ] slice O1 on-type
- [ ] cargo test -p draconic-check
- [ ] Type still on IR exprs

**Out of scope:**
- rewriting check_expr match (that is [[task-797-type-world-checker-use]])
- JS backend native.rs
- new crate

**Explain this part:**
Expand then migrate. This sitting only adds the methods and tests them.
