---
id: "slice-781-for-head-destructure"
title: "for-of assignment destructure compiles"
kind: slice
status: active
sprint: "dragons-audit"
blocked_by: []
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T12:00:00Z"
---

# for-of assignment destructure compiles

## Why

The statements Conformance fixture for for-head destructure is red on js. Assignment array patterns in `for-of` must typecheck like declaration heads already do.

## Done

`es/statements/for_head_destructure` runs on its declared js target. `for ([u] of [[4], [5]])` is not `array pattern cannot be used as a value`.

## Blocked by

None.

## Non-goals

- **native target** for this fixture (meta is js)
- **LLVM walker**
- **new ROADMAP atom**

## Oracle checklist

- [ ] O1: for_head_destructure_runs green
  CHECK: cargo test -p draconic-conformance --test statements for_head_destructure_runs --offline
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

- [[task-788-for-head-destructure]]

## See also

[[ticket-774-for-head-destructure]], tests/conformance/fixtures/es/statements/for_head_destructure.drac, crates/draconic-check/src/checker/checker_expr.rs
