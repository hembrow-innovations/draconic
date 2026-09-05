---
id: "l10-crypto-later-hmac-aead-after"
title: "L10 Crypto later: HMAC + AEAD (after L03)"
kind: task
status: completed
blocked-by: ["l10-01-hmac-sha256-after-l03", "l10-02-aead-encrypt-decrypt-after-l03"]
tags: []
created_at: "2026-09-02T22:29:53Z"
updated_at: "2026-09-05T18:09:16Z"
---

# L10 Crypto later: HMAC + AEAD (after L03)

## Blocked by

[[l10-01-hmac-sha256-after-l03]] and [[l10-02-aead-encrypt-decrypt-after-l03]]. L03 already landed. Parent remainder waits until HMAC-SHA256 and AEAD child atoms land so Build does not duplicate those Loops.

## Done

ROADMAP L10 is implemented test-first on both targets: a Program can HMAC-SHA256 a message with a key and AEAD encrypt/decrypt bytes (algorithm as designed) through the designed stdlib crypto surface; invalid key, nonce, or ciphertext errors rather than silently succeeding; `stdlib/crypto` fixtures lock that combined surface and L10 is `done`.

## Context

Roadmap ID **L10** (Crypto later: HMAC + AEAD (after L03)). Stdlib location: honest portable libs a simple service needs. Later than the L v1 bar; L10.01 and L10.02 land the per-class HMAC-SHA256 and AEAD fixtures; this sitting unifies them as one later crypto library after L03 SHA-256 digest. L03.01 SHA-256 digest and L03.02 CSPRNG are already `done`; this cut does not re-land them. Tests under `tests/conformance` fixtures `stdlib/crypto`. Harness `tests/conformance/tests/stdlib_crypto.rs`. Mark L10 `done` only when those tests are green. Not L10.01, L10.02, L03 parent unify, Web Crypto or Node `crypto` identity, or changing the L v1 done bar to require L10.

## Verify

`cargo test -p draconic-conformance --test stdlib_crypto` prints `hmac_sha256`, `aead`, and `test result: ok.` Workspace `cargo test --workspace` stays green. L10 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L10), `tests/conformance/fixtures/stdlib/crypto`, `tests/conformance/tests/stdlib_crypto.rs`, stdlib crypto surface as needed for both targets

## Links

[[s-l10]] [[ticket-90-l10-crypto-later-hmac-aead-after]]

## Gauntlet

- **round 1**: `cargo test -p draconic-conformance --test stdlib_crypto` — win (`hmac_sha256`, `aead`, `test result: ok.`). `cargo test --workspace` — win (`test result: ok.`). Combined `stdlib/crypto/hmac_aead` fixture locks HMAC-SHA256 + AEAD in one Program. Gap: none.
