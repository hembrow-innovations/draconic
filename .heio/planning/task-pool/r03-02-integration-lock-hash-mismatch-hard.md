---
id: "r03-02-integration-lock-hash-mismatch-hard"
title: "R03.02 Integration: lock hash mismatch hard-fails build"
kind: task
status: ready
tags: []
created_at: "2026-09-02T14:00:00Z"
updated_at: "2026-09-02T14:00:00Z"
---

# R03.02 Integration: lock hash mismatch hard-fails build

## Done

ROADMAP R03.02 is implemented test-first on the compiler target: integration tests under `tests/integration` lock that a lock hash mismatch hard-fails the build (no silent wrong tree); those tests are green and R03.02 is `done`.

## Context

Roadmap ID **R03.02** (Integration: lock hash mismatch hard-fails build). K08.01 and K08.02 already land tree SHA-256 and OID/hash refuse in `draconic-pkg`; this sitting proves those checks through the compiler integration surface when the lock pin does not match the resolved tree. Tests under `tests/integration` (`supply_chain_lock_hash_mismatch`) lock that a lock hash mismatch cannot silently succeed. Mark R03.02 `done` only when those tests are green. Not R03 parent remainder, R03.01 tampered-cache refuse, K08 pkg surface, K08.01/K08.02 (already `done`), K09, or R02.

## Verify

`cargo test -p draconic-integration-tests --test supply_chain_lock_hash_mismatch` prints `test result: ok.` Workspace `cargo test --workspace` stays green. R03.02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (R03.02), `tests/integration`

## Links

[[s-r03-02]] [[ticket-112-r03-02-integration-lock-hash-mismatch-hard]]
