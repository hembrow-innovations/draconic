---
id: "l04-compression-later-gzip-deflate"
title: "L04 Compression later: gzip/deflate byte buffers"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:45:47Z"
updated_at: "2026-09-02T13:45:47Z"
---

# L04 Compression later: gzip/deflate byte buffers

## Done

ROADMAP L04 is implemented test-first on both targets: a Program can gzip and deflate byte buffers (compress and decompress) through the designed stdlib compression surface, invalid or truncated input errors rather than silently corrupting, `stdlib/compression` fixtures are green, and L04 is `done`.

## Context

Roadmap ID **L04** (`Compression later: gzip/deflate byte buffers`). Stdlib location: honest portable libs a simple service needs. Later than the L v1 bar; this sitting is still one atomic Loop so a Program can round-trip gzip and deflate buffers without leaving Draconic. Tests under `tests/conformance` fixtures `stdlib/compression`. Harness `tests/conformance/tests/stdlib_compression.rs`. Mark L04 `done` only when those tests are green. Not L01 encoding, L03 crypto, L09 MIME, L10 HMAC/AEAD, zip/tar/brotli/zstd, a Node `zlib` identity surface, or changing the L v1 done bar to require L04.

## Verify

`cargo test -p draconic-conformance --test stdlib_compression` prints `test result: ok.` Workspace `cargo test --workspace` stays green. L04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L04), `tests/conformance/fixtures/stdlib/compression`, `tests/conformance/tests/stdlib_compression.rs`, stdlib compression surface as needed for both targets

## Links

[[s-l04]] [[ticket-82-l04-compression-later-gzip-deflate-byte]]
