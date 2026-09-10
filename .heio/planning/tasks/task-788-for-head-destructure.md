---
id: "task-788-for-head-destructure"
title: "Compile for-of assignment array patterns"
kind: task
status: claimed
mode: afk
blocked_by: []
sprint: "dragons-audit"
slice: "slice-781-for-head-destructure"
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-10T05:30:00Z"
---
# Compile for-of assignment array patterns

## Blocked by

None.

## Done

`for_head_destructure_runs` is green. `for ([u] of [[4], [5]])` typechecks.

## Context

Fixture `es/statements/for_head_destructure.drac` line 16 uses an assignment array pattern as the for-of left. Checker reports `array pattern cannot be used as a value`. Declaration heads in the same file (`const [a]`, `let { x }`, `var [k]`) are not the panic. Targets: js only.

Do not add a native target. Do not touch LLVM.

## Verify

Slice O1.

scope: crates/draconic-check (for-in/of left / assignment pattern), tests/conformance/fixtures/es/statements/for_head_destructure.drac if the source must change

## Links

[[slice-781-for-head-destructure]] [[ticket-774-for-head-destructure]]

## Agent Brief

**Category:** bug
**Summary:** Assignment destructure in for-of must compile on js.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none (fixture already exists)
- Purpose: for-head destructure on js
- Contract-first: do not invent native coverage

**Current behavior:**
`for ([u] of [[4], [5]])` fails compile: array pattern cannot be used as a value.

**Desired behavior:**
The fixture's js.check exits 0. Slice O1 holds.

**Key interfaces:**
- Checker for-in/of left
- size-file-budget: no new file over 1000

**Acceptance criteria:**
- [ ] slice O1 for_head_destructure_runs ok
- [ ] no ROADMAP.md edit
- [ ] no new LLVM adapter

**Out of scope:**
- native target
- LLVM walker
- object-rest extras beyond the fixture

**Explain this part:**
The panic is the assignment head, not `const`/`let` heads. Fix the left-hand classify, do not rewrite the fixture to avoid assignment patterns.
