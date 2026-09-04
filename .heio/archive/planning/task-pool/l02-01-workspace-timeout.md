---
id: "l02-01-workspace-timeout"
title: "L02.01 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T19:51:04Z"
updated_at: "2026-09-04T19:55:24Z"
---

# L02.01 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP L02.01 work; stdlib collections groupBy/chunk (or designed names) fixtures stay green.

## Context

Roadmap ID **L02.01** (`groupBy` / `chunk` (or designed names) on arrays). Review of [[s-l02-01]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`stdlib_collections`) stayed green. If the hang comes from the L02.01 change, fix that groupBy/chunk (or designed names) surface so both the workspace check and those fixtures hold. Mark L02.01 `done` only when those tests are green. Not L02 parent remainder, L02.02 Deque, E-cluster Array/Map/Set builtins, L01 encoding, L03 crypto, or L07 flags. Do not re-open [[s-l02-01]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test stdlib_collections --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test stdlib_collections` still prints `test result: ok.` L02.01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L02.01), `tests/conformance/tests/stdlib_collections.rs`, `tests/conformance/fixtures/stdlib/collections`, groupBy/chunk (or designed names) surface as needed to unhang workspace tests after L02.01

## Links

[[s-l02-01-workspace-timeout]] [[ticket-180-l02-01-workspace-timeout]] [[s-l02-01]]
