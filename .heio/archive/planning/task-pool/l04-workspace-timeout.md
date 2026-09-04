---
id: "l04-workspace-timeout"
title: "L04 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T20:06:18Z"
updated_at: "2026-09-04T20:30:25Z"
---

# L04 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP L04 work; the stdlib compression conformance tests stay green.

## Context

Roadmap ID **L04** (`Compression later: gzip/deflate byte buffers`). Review of [[s-l04]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`stdlib_compression`) stayed green. The stdlib location still needs the L04 Loop to leave the workspace green, not only the gzip/deflate byte-buffer conformance fixtures. If the hang comes from the L04 change, fix that gzip/deflate byte-buffer surface so both the workspace check and those fixtures hold. Mark L04 `done` only when those tests are green. Not L01 encoding, L03 crypto, L09 MIME, L10 HMAC/AEAD, zip/tar/brotli/zstd, a Node `zlib` identity surface, or changing the L v1 done bar to require L04. Do not re-open [[s-l04]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test stdlib_compression --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test stdlib_compression` still prints `test result: ok.` L04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L04), `tests/conformance/tests/stdlib_compression.rs`, `tests/conformance/fixtures/stdlib/compression`, stdlib compression surface as needed to unhang workspace tests after L04

## Links

[[s-l04-workspace-timeout]] [[ticket-182-l04-workspace-timeout]] [[s-l04]]
