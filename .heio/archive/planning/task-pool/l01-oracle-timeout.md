---
id: "l01-oracle-timeout"
title: "L01 encoding and workspace checks finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T19:47:45Z"
updated_at: "2026-09-04T20:11:29Z"
---

# L01 encoding and workspace checks finish

## Blocked by

None.

## Done

`cargo test -p draconic-conformance --test encoding` and `cargo test --workspace` each finish with `test result: ok.` after the ROADMAP L01 work.

## Context

Roadmap ID **L01** (Encoding: UTF-8 bytes↔string, Base64, hex). Review of [[s-l01]] left O1 and O2 unmet: `cargo test -p draconic-conformance --test encoding` timed out at 120s without matching `test result: ok.`, and `cargo test --workspace` also timed out at 120s. A Program can convert UTF-8 bytes↔string, Base64, and hex through the designed encoding surface, and invalid input errors rather than silently corrupting. Tests under `tests/conformance` fixtures `stdlib/encoding` lock that combined surface. If the hang comes from the L01 encoding surface, fix that surface so both checks hold. Mark L01 `done` only when those tests are green. Not L01.01 UTF-8, L01.02 Base64, or L01.03 hex (already `done`), and not L02 collections, L03 crypto, or L08 URL/query. Do not re-open [[s-l01]].

## Verify

`cargo test -p draconic-conformance --offline --test encoding` prints `test result: ok.` and finishes (does not hang). `cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --offline --test encoding` prints `test result: ok.` and finishes. L01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L01), `tests/conformance/tests/encoding.rs`, `tests/conformance/fixtures/stdlib/encoding`, `crates/draconic-backend-llvm/src/es_encoding.rs`, encoding surface as needed to unhang encoding and workspace tests after L01

## Links

[[s-l01-oracle-timeout]] [[ticket-179-l01-oracle-timeout]] [[s-l01]]
