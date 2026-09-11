---
id: "ticket-775-annex-b-native-false-green"
title: "Annex B native fixtures fail while ROADMAP is done"
kind: ticket
status: closed
ticket_type: bug
tags: []
blocked_by: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T06:45:26Z"
---

# Annex B native fixtures fail while ROADMAP is done

## Signal

`cargo test -p draconic-conformance --test annex_b` fails three `_runs` tests on native with `unsupported IR`. Fixtures declare `targets: js,native` and `native.stdout`. ROADMAP N08.16.35, N08.16.37, N08.16.38 are `done`.

- **async_methods**: `es/annex-b/async_methods`
- **private_methods**: `es/annex-b/private_methods`
- **static_private_fields**: `es/annex-b/static_private_fields`

## Fit

Honesty: [[slice-782-annex-b-native-honesty]] (`met`). Native restore: [[slice-784-llvm-no-fingerprint]] (`met`). Closed.

## Notes

- LLVM `emit_llvm_ir_raw` has no adapter that claims these programs. Leftover `es_expr` classify rejects them.
- Intent: a Roadmap item is done only when tests are green on every applicable target.
- Related open: [[ticket-773-native-console-log]] (same unsupported-IR shape, different program).

## Parent

[[slice-782-annex-b-native-honesty]]
