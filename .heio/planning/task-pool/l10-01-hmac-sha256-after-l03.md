---
id: "l10-01-hmac-sha256-after-l03"
title: "L10.01 HMAC-SHA256 (after L03)"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:51:45Z"
updated_at: "2026-09-02T13:51:45Z"
---

# L10.01 HMAC-SHA256 (after L03)

## Done

ROADMAP L10.01 is implemented test-first on both targets: a Program can compute HMAC-SHA256 over bytes with a key through the designed stdlib crypto surface, matching known test vectors; invalid key or message input errors rather than silently succeeding; `stdlib/crypto` fixtures lock that HMAC-SHA256 surface and L10.01 is `done`.

## Context

Roadmap ID **L10.01** (HMAC-SHA256 (after L03)). Stdlib location: honest portable libs a simple service needs. Later than the L v1 bar; this sitting is still one atomic Loop so a Program can HMAC-SHA256 a message with a key. L03.01 SHA-256 digest and L03.02 CSPRNG are already `done`; this cut does not re-land them. Tests under `tests/conformance` fixtures `stdlib/crypto`. Harness `tests/conformance/tests/stdlib_crypto.rs`. Mark L10.01 `done` only when those tests are green. Not L10 parent HMAC+AEAD remainder, L10.02 AEAD, L03 parent unify, Web Crypto or Node `crypto` identity, or changing the L v1 done bar to require L10.

## Verify

`cargo test -p draconic-conformance --test stdlib_crypto` prints `hmac_sha256` and `test result: ok.` Workspace `cargo test --workspace` stays green. L10.01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L10.01), `tests/conformance/fixtures/stdlib/crypto`, `tests/conformance/tests/stdlib_crypto.rs`, stdlib crypto surface as needed for both targets

## Links

[[s-l10-01]] [[ticket-91-l10-01-hmac-sha256-after-l03]]
