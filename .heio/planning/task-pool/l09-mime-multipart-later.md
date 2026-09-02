---
id: "l09-mime-multipart-later"
title: "L09 MIME multipart later (HTTP-shaped programs; after H10)"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:50:12Z"
updated_at: "2026-09-02T13:50:12Z"
---

# L09 MIME multipart later (HTTP-shaped programs; after H10)

## Done

ROADMAP L09 is implemented test-first on both targets: a Program can parse a MIME multipart body (boundary plus parts with headers and bodies) and serialize parts back into multipart text through the designed stdlib MIME surface, including a common-case round-trip; invalid or truncated multipart input errors rather than silently dropping parts; `stdlib/mime` fixtures lock that surface and L09 is `done`.

## Context

Roadmap ID **L09** (MIME multipart later (HTTP-shaped programs; after H10)). Stdlib location: honest portable libs a simple service needs. Later than the L v1 bar; this sitting is still one atomic Loop so a Program can build or read multipart bodies without leaving Draconic. Tests under `tests/conformance` fixtures `stdlib/mime`. Harness `tests/conformance/tests/stdlib_mime.rs`. Mark L09 `done` only when those tests are green. Not H10 HTTP/1.1 helpers, L08 URL/query, L04 gzip/deflate, L10 HMAC/AEAD, full email MIME (message/rfc822), a MIME type registry, Node `formidable` / `busboy` identity, or changing the L v1 done bar to require L09.

## Verify

`cargo test -p draconic-conformance --test stdlib_mime` prints `test result: ok.` Workspace `cargo test --workspace` stays green. L09 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L09), `tests/conformance/fixtures/stdlib/mime`, `tests/conformance/tests/stdlib_mime.rs`, stdlib MIME surface as needed for both targets

## Links

[[s-l09]] [[ticket-89-l09-mime-multipart-later-http-shaped]]
