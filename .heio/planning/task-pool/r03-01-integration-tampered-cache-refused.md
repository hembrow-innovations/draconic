---
id: "r03-01-integration-tampered-cache-refused"
title: "R03.01 Integration: tampered cache refused (depends K08)"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:59:30Z"
updated_at: "2026-09-02T13:59:30Z"
---

# R03.01 Integration: tampered cache refused (depends K08)

## Done

ROADMAP R03.01 is implemented test-first on the compiler target: integration tests under `tests/integration` lock that a tampered module cache is refused (no silent wrong tree); those tests are green and R03.01 is `done`.

## Context

Roadmap ID **R03.01** (Integration: tampered cache refused (depends **K08**)). K08.01 and K08.02 already land tree SHA-256 and OID/hash refuse in `draconic-pkg`; this sitting proves those checks through the compiler integration surface when cache contents have been altered after the lock pin. Tests under `tests/integration` (`supply_chain_tampered_cache`) lock that a tampered cache cannot silently succeed. Mark R03.01 `done` only when those tests are green. Not R03 parent remainder, R03.02 lock-hash-mismatch hard-fail, K08 pkg surface, K08.01/K08.02 (already `done`), K09, or R02.

## Verify

`cargo test -p draconic-integration-tests --test supply_chain_tampered_cache` prints `test result: ok.` Workspace `cargo test --workspace` stays green. R03.01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (R03.01), `tests/integration`

## Links

[[s-r03-01]] [[ticket-111-r03-01-integration-tampered-cache-refused-depends]]
