---
id: "ticket-774-for-head-destructure"
title: "for-of assignment array pattern fails compile"
kind: ticket
status: closed
ticket_type: bug
tags: []
blocked_by: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T12:45:00Z"
---

# for-of assignment array pattern fails compile

## Signal

`cargo test -p draconic-conformance --test statements for_head_destructure_runs` fails on js. `for ([u] of [[4], [5]])` reports `array pattern cannot be used as a value` at the pattern. Fixture `es/statements/for_head_destructure` targets js only.

## Fit

Promoted to [[slice-781-for-head-destructure]]. Slice is `met`. Closed.

## Notes

- **const/let heads**: `for (const [a] of …)` and `for (let { x } of …)` are in the same fixture; the panic is the assignment head.
- **Workspace**: 2026-09-10 `cargo test --workspace --no-fail-fast` 3303 passed, 4 failed. This is one of the four.

## Parent

[[slice-781-for-head-destructure]]
