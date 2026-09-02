---
id: "k08-integrity-verify-lock-hashes-refuse"
title: "K08 Integrity: verify lock hashes; refuse tampered cache"
kind: task
status: ready
tags: []
created_at: "2026-09-02T22:31:31Z"
updated_at: "2026-09-02T22:31:31Z"
---

# K08 Integrity: verify lock hashes; refuse tampered cache

## Done

ROADMAP K08 is implemented test-first on the compiler target: verifying a locked package recomputes the canonical tree SHA-256 and hard-fails on mismatch; mismatched checkout OID or content hash refuses the tree (no silent wrong tree); `draconic-pkg` hash tests are green and K08 is `done`.

## Context

Roadmap ID **K08** (`Integrity: verify lock hashes; refuse tampered cache`). K08.01–K08.02 already land recompute-and-hard-fail and refuse-mismatched-OID/hash; this sitting unifies them as one honest verify-lock-hashes / refuse-tampered-cache surface on the compiler target. Tests in `crates/draconic-pkg`. Harness `cargo test -p draconic-pkg hash`. Mark K08 `done` only when those tests are green. Not K08.01–K08.02 as separate atoms, K02, K03, R03 / R03.01 / R03.02, or K09.

## Verify

`cargo test -p draconic-pkg hash` prints `test result: ok.` Workspace `cargo test --workspace` stays green. K08 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K08), `crates/draconic-pkg`, `crates/draconic-pkg/src/hash.rs`

## Links

[[s-k08]] [[ticket-56-k08-integrity-verify-lock-hashes-refuse]]
