---
id: "task-797-type-world-checker-use"
title: "Checker walks call Type world predicates"
kind: task
status: completed
mode: afk
blocked_by: ["task-796-type-world-methods"]
sprint: "dragons-audit"
slice: "slice-786-type-world-predicates"
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T14:30:00Z"
---

# Checker walks call Type world predicates

## Blocked by

[[task-796-type-world-methods]]: methods exist.

## Done

`checker_expr`, `checker_ops`, and `checker_assign` use Type world predicates instead of copied Native/Ptr/Number matches for dual-world rules. `check_expr` remains one match. Slice O2 holds. No new checker_*.rs file.

## Context

Migrate call sites. Delete local `is_js_to_number_operand` style helpers if they become wrappers. Keep `check_expr` as one match; extract arms as functions in the same file if needed. Do not split the hub for line count.

## Verify

Slice O2.

scope: crates/draconic-check/src/checker/*.rs dual-world branches, types.rs only if a predicate must grow

## Links

[[slice-786-type-world-predicates]] [[task-796-type-world-methods]]

## Agent Brief

**Category:** types
**Summary:** Checker asks Type for world tests.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none
- Purpose: dual worlds at the type seam
- Contract-first: do not invent new conversions

**Current behavior:**
Copied matches in expr, ops, assign.

**Desired behavior:**
Call Type methods. Check tests green. No extra checker file.

**Key interfaces:**
- Type::is_js_value / is_native_world / is_dual_world_boundary
- size-file-budget: no new file over 1000

**Acceptance criteria:**
- [x] cargo test -p draconic-check
- [x] no new checker_*.rs
- [x] check_expr still one match

**Out of scope:**
- hiding Type from IR
- LLVM
- host catalog

**Explain this part:**
If an arm is not dual-world, leave it. Do not drive-by refactor the whole 983-line match.

## Gauntlet

- **round**: 1
- **command**: cargo test -p draconic-check --offline; check_expr one match; no new checker_*.rs
- **result**: win
- **gap**: none
