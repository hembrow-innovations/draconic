---
id: "l08-url-query-parse-serialize"
title: "L08 URL / query parse + serialize"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:49:03Z"
updated_at: "2026-09-02T13:49:03Z"
---

# L08 URL / query parse + serialize

## Done

ROADMAP L08 is implemented test-first on both targets: a Program can parse a URL into scheme/host/path/query/hash and parse/serialize query text through the designed URL surface, including common round-trips; `stdlib/url` fixtures lock that combined surface and L08 is `done`.

## Context

Roadmap ID **L08** (URL / query parse + serialize). Stdlib location: honest portable libs a simple service needs. L08.01 and L08.02 already land `parseUrl` (scheme/host/path/query/hash) and `parseQuery` / `serializeQuery` with common-case round-trips; this sitting unifies those ops as one URL library. Tests under `tests/conformance` fixtures `stdlib/url`. Harness `tests/conformance/tests/stdlib_url.rs`. Mark L08 `done` only when those tests are green. Not L08.01, L08.02, L09 MIME multipart, H10 HTTP/1.1 helpers, WHATWG URL identity, or a Node `url` clone.

## Verify

`cargo test -p draconic-conformance --test stdlib_url` prints `test result: ok.` Workspace `cargo test --workspace` stays green. L08 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L08), `tests/conformance/fixtures/stdlib/url`, `tests/conformance/tests/stdlib_url.rs`, stdlib URL surface as needed for both targets

## Links

[[s-l08]] [[ticket-88-l08-url-query-parse-serialize]]
