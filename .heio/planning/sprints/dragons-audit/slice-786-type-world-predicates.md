---
id: "slice-786-type-world-predicates"
title: "Dual-world tests live on Type"
kind: slice
status: active
sprint: "dragons-audit"
blocked_by: []
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T14:30:00Z"
---

# Dual-world tests live on Type

## Why

JS value vs native type rules bounce across expr, ops, and assign. Callers should ask `Type`, not re-match Native / Ptr / Number.

## Done

World predicates live on `Type`. `checker_expr`, `checker_ops`, and `checker_assign` call them instead of copying the matches. `check_expr` stays one match. Check tests stay green. No new file over 1000 lines.

## Blocked by

None.

## Non-goals

- **hiding Type from IR**
- **splitting checker.rs for file budget**
- **new crate seam**

## Oracle checklist

- [x] O1: Type owns world tests
  CHECK: python3 -c 'from pathlib import Path; t=Path("crates/draconic-check/src/types.rs").read_text(); print("on-type" if "fn is_js_value" in t and "fn is_native_world" in t else "missing")'
  EXPECT: on-type
  EVIDENCE: on-type (task-796)
- [x] O2: check crate tests green
  CHECK: cargo test -p draconic-check --offline
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 283 passed (task-797)

## Pool

- [[task-796-type-world-methods]]
- [[task-797-type-world-checker-use]]

## See also

[[ticket-779-type-world-predicates]], crates/draconic-check/src/types.rs, crates/draconic-check/src/checker/checker_ops.rs, crates/draconic-check/src/checker/checker_type.rs
