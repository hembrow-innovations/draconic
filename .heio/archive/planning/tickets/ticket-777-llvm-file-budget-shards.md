---
id: "ticket-777-llvm-file-budget-shards"
title: "LLVM file-budget shards fail the deletion test"
kind: ticket
status: closed
ticket_type: observation
tags: []
blocked_by: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T06:57:16Z"
---
# LLVM file-budget shards fail the deletion test

## Signal

Dragons audit 2026-09-10. No llvm `.rs` file is over 1000 lines. Eighteen sit at 900–993. Features were split into `classify` / `ok` / `eval` siblings. Shared `emitter.rs` `SlotTy` and `Emitter` are used by leftover `es_expr` and `es_functions` only. About 37 `escape_llvm_string` copies and 35 private `SlotTy` enums remain.

## Fit

Promoted to [[slice-783-llvm-one-emitter]] as prefactor for [[slice-784-llvm-no-fingerprint]].

## Notes

- [[slice-712-backend-llvm-file-budget]] is met on the line cap. The shards are the cost.
- Deleting a nested `classify.rs` would move the same predicates, not concentrate them.

## Parent

[[slice-783-llvm-one-emitter]]
