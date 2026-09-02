---
id: "l02-collections-helpers-groupby-chunk-deque"
title: "L02 Collections helpers (groupBy/chunk/Deque as designed; not redundant with Array/Map/Set)"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:45:43Z"
updated_at: "2026-09-02T13:45:43Z"
---

# L02 Collections helpers (groupBy/chunk/Deque as designed; not redundant with Array/Map/Set)

## Done

ROADMAP L02 is implemented test-first on both targets: a Program can use designed collections helpers — groupBy/chunk on arrays and a Deque with push/pop at both ends (or designed names) — without those ops duplicating Array, Map, or Set; `stdlib/collections` fixtures lock that combined surface and L02 is `done`.

## Context

Roadmap ID **L02** (Collections helpers (groupBy/chunk/Deque as designed; not redundant with Array/Map/Set)). Stdlib location: honest portable libs a simple service needs. L02.01 and L02.02 land the per-class groupBy/chunk and Deque fixtures; this sitting unifies them as one collections helper library that is not a redundant Array/Map/Set clone. Tests under `tests/conformance` fixtures `stdlib/collections`. Harness `tests/conformance/tests/stdlib_collections.rs`. Mark L02 `done` only when those tests are green. Not L02.01, L02.02, L01 encoding, L03 crypto, L07 flags, L08 URL, or ECMA-262 Array/Map/Set builtins.

## Verify

`cargo test -p draconic-conformance --test stdlib_collections` prints `test result: ok.` Workspace `cargo test --workspace` stays green. L02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L02), `tests/conformance/fixtures/stdlib/collections`, `tests/conformance/tests/stdlib_collections.rs`, stdlib collections helper surface as needed for both targets

## Links

[[s-l02]] [[ticket-78-l02-collections-helpers-groupby-chunk-deque]]
