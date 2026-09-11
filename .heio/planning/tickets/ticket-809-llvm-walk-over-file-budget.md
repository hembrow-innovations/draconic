---
id: "ticket-809-llvm-walk-over-file-budget"
title: "LLVM walk.rs is over the 1000-line target"
kind: ticket
status: open
ticket_type: observation
tags: []
blocked_by: []
created_at: "2026-09-11T21:47:50Z"
updated_at: "2026-09-11T21:47:50Z"
---

# LLVM walk.rs is over the 1000-line target

## Signal

Dragons audit 2026-09-12. `crates/draconic-backend-llvm/src/es_expr/walk.rs` is 1075 lines. AGENTS.md target is 1000, hard limit 1250. The extra is a host name cascade plus a 40-arm `es_kind` steal list, not a second feature. Closed [[ticket-777-llvm-file-budget-shards]] met the line cap by splitting classify/ok siblings. This file crossed again.

Not the eight native Conformance fails. That is [[ticket-807-llvm-walker-native-date-stdlib]].

## Fit

this project, later slice

## Notes

- Count: `wc -l crates/draconic-backend-llvm/src/es_expr/walk.rs` → 1075.
- No `#[test]` in this file.
- Do not mark E17.02 or E18.44 done.
