---
id: "l03-workspace-timeout"
title: "L03 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T20:00:05Z"
updated_at: "2026-09-04T20:15:51Z"
---

# L03 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP L03 work; the stdlib crypto conformance tests stay green.

## Context

Roadmap ID **L03** (`Crypto: SHA-256 digest + secure random bytes`). Review of [[s-l03]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`stdlib_crypto`) stayed green. The stdlib location still needs the L03 Loop to leave the workspace green, not only the crypto conformance fixtures for SHA-256 digest and secure random bytes. If the hang comes from the L03 change, fix that SHA-256 digest and secure-random surface so both the workspace check and those fixtures hold. Mark L03 `done` only when those tests are green. Not L03.01 SHA-256 digest, L03.02 secure random bytes (already `done`), L01 encoding, L04 compression, or L10 HMAC/AEAD. Do not re-open [[s-l03]]. Do not re-file Web Crypto or Node `crypto` as the v1 surface.

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test stdlib_crypto --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test stdlib_crypto` still prints `test result: ok.` L03 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L03), `tests/conformance/tests/stdlib_crypto.rs`, `tests/conformance/fixtures/stdlib/crypto`, stdlib crypto surface as needed to unhang workspace tests after L03

## Links

[[s-l03-workspace-timeout]] [[ticket-181-l03-workspace-timeout]] [[s-l03]]
