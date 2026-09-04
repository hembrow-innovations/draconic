---
id: "s-l03-workspace-timeout"
title: "L03 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T19:58:27Z"
updated_at: "2026-09-04T20:29:53Z"
claimed-by: 6fb80b34-1e98-408d-a777-13211c0df314
---

# L03 workspace tests finish

## Why

Review of [[s-l03]] left ROADMAP L03 unfinished: O1 (`stdlib_crypto`) held, but O2 `cargo test --workspace` timed out at 120s. The stdlib location still needs the L03 Loop to leave the workspace green, not only the crypto conformance fixtures for SHA-256 digest and secure random bytes.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP L03 work. The stdlib crypto conformance tests stay green. If the hang comes from the L03 change, fix that SHA-256 digest and secure-random surface so both the workspace check and those fixtures hold. Mark L03 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-l03]]**: that slice stays sealed `failed`
- **L03.01**: SHA-256 digest over bytes; known test vectors (already `done`)
- **L03.02**: Secure random bytes (OS CSPRNG); length parameter (already `done`)
- **L01**: Encoding UTF-8 / Base64 / hex
- **L04**: Compression gzip/deflate
- **L10**: Crypto later HMAC + AEAD (after L03)
- Re-filing Web Crypto or Node `crypto` as the v1 surface

## Oracle checklist

- [x] O1: workspace tests finish after the L03 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test stdlib_crypto --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=ec554269531871e8 bytes=94804 at=2026-09-04T20:29:36.983Z

- [x] O2: L03 SHA-256 digest and secure-random fixtures stay locked by the stdlib crypto conformance tests
  CHECK: cargo test -p draconic-conformance --test stdlib_crypto
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=48156a184bd06b05 bytes=3234 at=2026-09-04T20:29:38.352Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[l03-workspace-timeout]]`

## See also

ROADMAP.md L03, `tests/conformance/tests/stdlib_crypto.rs`, `tests/conformance/fixtures/stdlib/crypto`, CONTEXT.md, [[stdlib]], [[s-l03]], [[ticket-181-l03-workspace-timeout]].
