---
id: "l10-workspace-timeout"
title: "L10 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-05T20:29:12Z"
---

# L10 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. stdlib_crypto hmac and aead stay green.

## Context

Reviewer miss on [[s-l10]]. O3 matched EXPECT but `exit=timeout`. Not a product fail. Not a new L10 Loop atom. L10 stays `done`.

## Verify

`cargo test --workspace` exit 0 with `test result: ok.` stdlib_crypto hmac_sha256 and aead still print `test result: ok.`

scope: workspace test runtime as needed so O3 finishes in budget

## Links

[[s-l10-workspace-timeout]] [[ticket-200-l10-workspace-timeout]]

## Gauntlet

- **round 1**: `cargo test --workspace` — win. Workspace finished with `test result: ok.` (161 ok lines, exit 0, no hang / no exit 101). Native runtime C fallback in `draconic-runtime` covers stale `CARGO_MANIFEST_DIR`. ROADMAP row remains `done`.
