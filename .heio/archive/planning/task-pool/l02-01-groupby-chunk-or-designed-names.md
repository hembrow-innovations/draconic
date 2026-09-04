---
id: "l02-01-groupby-chunk-or-designed-names"
title: "L02.01 `groupBy` / `chunk` (or designed names) on arrays"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:42:30Z"
updated_at: "2026-09-04T15:28:21Z"
---

# L02.01 `groupBy` / `chunk` (or designed names) on arrays

## Done

ROADMAP L02.01 is implemented test-first on both targets: a Program can `groupBy` and `chunk` (or designed names) arrays through the designed collections surface without duplicating Array/Map/Set or ECMA `Object.groupBy` / `Map.groupBy`; `stdlib/collections` fixtures are green and L02.01 is `done`.

## Context

Roadmap ID **L02.01** (`groupBy` / `chunk` (or designed names) on arrays). Stdlib location: honest portable libs a simple service needs. Names may be designed; they must not duplicate Array/Map/Set or the ECMA builtins. Tests under `tests/conformance` fixtures `stdlib/collections`. Harness `tests/conformance/tests/stdlib_collections.rs`. Mark L02.01 `done` only when those tests are green. Not L02 parent remainder, L02.02 Deque, E-cluster Array/Map/Set builtins, L01 encoding, L03 crypto, or L07 flags.

## Verify

`cargo test -p draconic-conformance --test stdlib_collections` prints `test result: ok.` Workspace `cargo test --workspace` stays green. L02.01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L02.01), `tests/conformance/fixtures/stdlib/collections`, `tests/conformance/tests/stdlib_collections.rs`, stdlib collections surface as needed for both targets

## Links

[[s-l02-01]] [[ticket-79-l02-01-groupby-chunk-or-designed-names]]
