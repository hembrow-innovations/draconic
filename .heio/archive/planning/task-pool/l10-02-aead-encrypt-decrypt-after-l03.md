---
id: "l10-02-aead-encrypt-decrypt-after-l03"
title: "L10.02 AEAD encrypt/decrypt (after L03; algorithm as designed)"
kind: task
status: completed
tags: []
created_at: "2026-09-02T22:29:04Z"
updated_at: "2026-09-05T13:46:42Z"
---

# L10.02 AEAD encrypt/decrypt (after L03; algorithm as designed)

## Done

ROADMAP L10.02 is implemented test-first on both targets: a Program can AEAD encrypt plaintext with a key and nonce and decrypt the matching ciphertext back to the original bytes through the designed stdlib crypto surface; invalid key, nonce, or ciphertext errors rather than silently succeeding; `stdlib/crypto` fixtures lock that AEAD surface and L10.02 is `done`.

## Context

Roadmap ID **L10.02** (AEAD encrypt/decrypt (after L03; algorithm as designed)). Stdlib location: honest portable libs a simple service needs. Later than the L v1 bar; this sitting is still one atomic Loop so a Program can AEAD encrypt and decrypt bytes (algorithm as designed). L03.01 SHA-256 digest and L03.02 CSPRNG are already `done`; this cut does not re-land them. Tests under `tests/conformance` fixtures `stdlib/crypto`. Harness `tests/conformance/tests/stdlib_crypto.rs`. Mark L10.02 `done` only when those tests are green. Not L10 parent HMAC+AEAD remainder, L10.01 HMAC-SHA256, L03 parent unify, Web Crypto or Node `crypto` identity, or changing the L v1 done bar to require L10.

## Verify

`cargo test -p draconic-conformance --test stdlib_crypto` prints `aead` and `test result: ok.` Workspace `cargo test --workspace` stays green. L10.02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L10.02), `tests/conformance/fixtures/stdlib/crypto`, `tests/conformance/tests/stdlib_crypto.rs`, stdlib crypto surface as needed for both targets

## Links

[[s-l10-02]] [[ticket-92-l10-02-aead-encrypt-decrypt-after-l03]]

## Gauntlet

- **Round 1**: `cargo test -p draconic-conformance --test stdlib_crypto` and `cargo test --workspace` — win. O1 printed `aead` with `test result: ok.`; O2 printed `test result: ok.` Workspace stayed green. Done line holds.
