---
id: "r03-supply-chain-policy-tests-once"
title: "R03 Supply-chain policy tests once K08 lands (lock verify refuse tamper)"
kind: task
status: ready
tags: []
created_at: "2026-09-02T22:35:46Z"
updated_at: "2026-09-02T22:35:46Z"
---

# R03 Supply-chain policy tests once K08 lands (lock verify refuse tamper)

## Done

ROADMAP R03 is implemented test-first on the compiler target: integration tests under `tests/integration` lock that lock-hash verify refuses tamper (tampered cache refused; lock hash mismatch hard-fails the build), and R03 is `done`.

## Context

Roadmap ID **R03** (Supply-chain policy tests once **K08** lands (lock verify refuse tamper)). Runtime-hardening location: the parent row that K08 lock-verify-refuse-tamper is proven through the compiler integration surface. K08.01 and K08.02 already land tree SHA-256 and OID/hash refuse in `draconic-pkg`; this sitting unifies tampered-cache refuse and lock-hash-mismatch hard-fail so neither can silently succeed. Tests under `tests/integration`. Harness `cargo test -p draconic-integration-tests --test supply_chain`. Mark R03 `done` only when those tests are green. Not R03.01 tampered-cache or R03.02 lock-hash-mismatch as separate atoms, K08 pkg surface, K08.01/K08.02 (already `done`), K09 E2E, or R02 permission model.

## Verify

`cargo test -p draconic-integration-tests --test supply_chain` prints `test result: ok.` Workspace `cargo test --workspace` stays green. R03 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (R03), `tests/integration`, `crates/draconic-pkg`

## Links

[[s-r03]] [[ticket-110-r03-supply-chain-policy-tests-once]]
