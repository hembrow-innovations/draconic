---
id: "l02-02-deque-or-designed-push-pop"
title: "L02.02 Deque (or designed): push/pop both ends"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:43:46Z"
updated_at: "2026-09-02T13:43:46Z"
---

# L02.02 Deque (or designed): push/pop both ends

## Done

ROADMAP L02.02 is implemented test-first on both targets: a Program can construct a Deque (or designed name) and push/pop at both ends without treating Array as that type; `stdlib/collections` fixtures are green and L02.02 is `done`.

## Context

Roadmap ID **L02.02** (`Deque (or designed): push/pop both ends`). Stdlib location: honest portable libs a simple service needs; not a second Array/Map/Set. L02.01 covers array groupBy/chunk; this sitting is the double-ended queue. Tests under `tests/conformance` fixtures `stdlib/collections`. Harness `tests/conformance/tests/stdlib_collections.rs`. Mark L02.02 `done` only when those tests are green. Not L02 parent remainder, L02.01 groupBy/chunk, E15/Array/Map/Set builtins, L01 encoding, L03 crypto, or C02 channels.

## Verify

`cargo test -p draconic-conformance --test stdlib_collections` prints `test result: ok.` Workspace `cargo test --workspace` stays green. L02.02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L02.02), `tests/conformance/fixtures/stdlib/collections`, `tests/conformance/tests/stdlib_collections.rs`, stdlib collections Deque surface as needed for both targets

## Links

[[s-l02-02]] [[ticket-80-l02-02-deque-or-designed-push-pop]]
