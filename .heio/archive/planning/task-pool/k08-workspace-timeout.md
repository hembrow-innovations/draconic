---
id: "k08-workspace-timeout"
title: "K08 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T18:59:00Z"
updated_at: "2026-09-04T19:08:07Z"
---

# K08 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K08 work; the `draconic-pkg` hash tests for recomputing canonical tree SHA-256 against the lock pin and refusing a mismatched OID or content hash (no silent wrong tree) stay green.

## Context

Roadmap ID **K08** (Integrity: verify lock hashes; refuse tampered cache). Review of [[s-k08]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`draconic-pkg` hash) stayed green. If the hang comes from the K08 change, fix that verify-lock-hashes / refuse-tampered-cache surface so both the workspace check and those crate tests hold. Mark K08 `done` only when those tests are green. Not K08.01 recompute tree SHA-256 / match lock or hard-fail, K08.02 refuse mismatched OID/hash / no silent wrong tree, K02 Lockfile (`draconic.lock`) resolved pins, K03 module cache layout / git clone, R03 / R03.01 / R03.02 integration supply-chain tests once K08 lands, or K09 E2E temp git dep + consumer Program. Do not re-open [[s-k08]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline hash` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-pkg hash` still prints `test result: ok.` K08 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K08), `crates/draconic-pkg/src/hash.rs`, `crates/draconic-pkg`, verify-lock-hashes / refuse-tampered-cache surface as needed to unhang workspace tests after K08

## Links

[[s-k08-workspace-timeout]] [[ticket-171-k08-workspace-timeout]] [[s-k08]]
