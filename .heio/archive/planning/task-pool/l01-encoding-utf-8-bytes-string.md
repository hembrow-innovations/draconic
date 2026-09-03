---
id: "l01-encoding-utf-8-bytes-string"
title: "L01 Encoding: UTF-8 bytes↔string, Base64, hex"
kind: task
status: completed
tags: []
created_at: "2026-09-02T22:26:46Z"
updated_at: "2026-09-03T05:16:34Z"
---

# L01 Encoding: UTF-8 bytes↔string, Base64, hex

## Done

ROADMAP L01 is implemented test-first on both targets: a Program can convert UTF-8 bytes↔string, Base64, and hex through the designed encoding surface, invalid input errors rather than silently corrupting, `stdlib/encoding` fixtures lock that combined surface, and L01 is `done`.

## Context

Roadmap ID **L01** (Encoding: UTF-8 bytes↔string, Base64, hex). Stdlib location: honest portable libs a simple service needs. L01.01–L01.03 already land UTF-8 string↔bytes with invalid UTF-8 errors, Base64 encode/decode, and hex encode/decode; this sitting unifies them as one encoding library on both targets. Tests under `tests/conformance` fixtures `stdlib/encoding`. Harness `tests/conformance/tests/encoding.rs`. Mark L01 `done` only when those tests are green. Not L01.01, L01.02, L01.03 as separate atoms, L02 collections, L03 crypto, L08 URL, or re-filing E15 JSON/RegExp as encoding work.

## Verify

`cargo test -p draconic-conformance --test encoding` prints `test result: ok.` Workspace `cargo test --workspace` stays green. L01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L01), `tests/conformance/fixtures/stdlib/encoding`, `tests/conformance/tests/encoding.rs`, `crates/draconic-backend-llvm/src/es_encoding.rs`

## Links

[[s-l01]] [[ticket-77-l01-encoding-utf-8-bytes-string]]
