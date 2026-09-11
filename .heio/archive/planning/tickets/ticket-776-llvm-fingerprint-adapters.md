---
id: "ticket-776-llvm-fingerprint-adapters"
title: "LLVM emit is still a fingerprint chain"
kind: ticket
status: closed
ticket_type: observation
tags: []
blocked_by: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T06:45:26Z"
---

# LLVM emit is still a fingerprint chain

## Signal

Dragons audit 2026-09-10. `emit_llvm_ir_raw` dispatches native-ints, empty hello, then `emit_es_expr_walk`. That walk is `try_folded_walks`: ~60 `walk_*` whole-module fingerprints in load-bearing order, then leftover scalar classify. [[slice-756-llvm-one-walker]] is met. The cascade moved from `lib.rs` into `es_expr.rs`.

## Fit

Promoted to [[slice-784-llvm-no-fingerprint]], after [[slice-783-llvm-one-emitter]]. Slice is `met`. Closed.

## Notes

- Public interface stays `emit_llvm_ir`. Internals emit a full `@main` per adapter.
- Steal-order tests in llvm `lib.rs` exist so leftover class/function IR is not stolen.
- ADR-0002: one shared IR, two backends. Not an AST fork.
- Archived [[ticket-10-collapse-llvm-multipath]] closed the hello-stub lie, not this chain.

## Parent

[[slice-784-llvm-no-fingerprint]]
