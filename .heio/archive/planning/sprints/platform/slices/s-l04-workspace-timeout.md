---
id: "s-l04-workspace-timeout"
title: "L04 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T20:04:25Z"
updated_at: "2026-09-04T20:32:10Z"
claimed-by: 78e8c9bb-6b4f-4302-9fec-d1a7f2d20527
---

# L04 workspace tests finish

## Why

Review of [[s-l04]] left ROADMAP L04 unfinished: O1 (`stdlib_compression`) held, but O2 `cargo test --workspace` timed out at 120s. The stdlib location still needs the L04 Loop to leave the workspace green, not only the gzip/deflate byte-buffer conformance fixtures.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP L04 work. The stdlib compression conformance tests stay green. If the hang comes from the L04 change, fix that gzip/deflate byte-buffer surface so both the workspace check and those fixtures hold. Mark L04 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-l04]]**: that slice stays sealed `failed`
- **L01**: Encoding UTF-8 / Base64 / hex
- **L03**: Crypto SHA-256 / CSPRNG
- **L09**: MIME multipart later
- **L10**: HMAC / AEAD later
- zip/tar archives, brotli, zstd, or a Node `zlib` identity surface
- changing the L v1 done bar to require L04

## Oracle checklist

- [x] O1: workspace tests finish after the L04 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test stdlib_compression --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=64efe0d5e72f4f19 bytes=94708 at=2026-09-04T20:31:54.997Z

- [x] O2: L04 gzip/deflate byte-buffer fixtures stay locked by the stdlib compression conformance tests
  CHECK: cargo test -p draconic-conformance --test stdlib_compression
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=c341f7f7f28e190d bytes=3138 at=2026-09-04T20:31:55.873Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[l04-workspace-timeout]]`

## See also

ROADMAP.md L04, `tests/conformance/tests/stdlib_compression.rs`, `tests/conformance/fixtures/stdlib/compression`, CONTEXT.md, [[stdlib]], [[s-l04]], [[ticket-182-l04-workspace-timeout]].
