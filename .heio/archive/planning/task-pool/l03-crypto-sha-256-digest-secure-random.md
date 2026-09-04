---
id: "l03-crypto-sha-256-digest-secure-random"
title: "L03 Crypto: SHA-256 digest + secure random bytes"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T13:24:30Z"
updated_at: "2026-09-04T18:58:45Z"
---

# L03 Crypto: SHA-256 digest + secure random bytes

## Blocked by

None.

## Done

ROADMAP L03 is implemented test-first on both targets: a Program can SHA-256 digest bytes against known vectors and draw secure random bytes of a requested length through the designed crypto surface; `stdlib/crypto` fixtures are green and L03 is `done`.

## Context

Roadmap ID **L03** (`Crypto: SHA-256 digest + secure random bytes`). Stdlib location: honest portable libs a simple service needs. L03.01–L03.02 already land SHA-256 digest over bytes with known test vectors and secure random bytes from the OS CSPRNG with a length parameter; this sitting unifies them as one crypto library on both targets. Tests under `tests/conformance` fixtures `stdlib/crypto`. Harness `tests/conformance/tests/stdlib_crypto.rs`. Mark L03 `done` only when those tests are green. Not L03.01–L03.02 as separate atoms, L01 encoding, L04 compression, L10 HMAC/AEAD, or re-filing Web Crypto or Node `crypto` as the v1 surface.

## Verify

`cargo test -p draconic-conformance --test stdlib_crypto` prints `test result: ok.` Workspace `cargo test --workspace` stays green. L03 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L03), `tests/conformance/fixtures/stdlib/crypto`, `tests/conformance/tests/stdlib_crypto.rs`, stdlib crypto surface as needed for both targets

## Links

[[s-l03]] [[ticket-81-l03-crypto-sha-256-digest-secure]]
