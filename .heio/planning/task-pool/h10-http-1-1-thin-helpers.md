---
id: "h10-http-1-1-thin-helpers"
title: "H10 HTTP/1.1 thin helpers (plaintext) on sockets"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:29:14Z"
updated_at: "2026-09-02T13:29:14Z"
---

# H10 HTTP/1.1 thin helpers (plaintext) on sockets

## Done

ROADMAP H10 is implemented test-first on native: parse an HTTP/1.1 request (line, headers, Content-Length body), write a status + headers + body response, run a one-shot server, optionally keep-alive two requests on one connection, issue a client request and read the response on connected TCP, encode/decode Transfer-Encoding: chunked, and hard-error JS HTTP listen helpers; `host/http` fixtures are green and H10 is `done`.

## Context

Roadmap ID **H10** (HTTP/1.1 thin helpers, plaintext, on sockets). H10.01–H10.07 already land request parse, response write, one-shot server, optional keep-alive, client on connected TCP, chunked transfer encoding, and js listen-helper hard-error; this sitting unifies them as one honest HTTP/1.1 helper surface on native. Tests under `tests/conformance` fixtures `host/http`. Harness `tests/conformance/tests/host_http.rs`. Mark H10 `done` only when those tests are green. Not H06, H11, H12, H13, H17, or H00.

## Verify

`cargo test -p draconic-conformance --test host_http` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H10 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H10), `tests/conformance/fixtures/host/http`, `tests/conformance/tests/host_http.rs`, `crates/draconic-backend-llvm/src/host_http.rs`, `crates/draconic-backend-llvm/src/host_http_server.rs`, `crates/draconic-runtime`, js/native HTTP helper paths as needed for the parent surface

## Links

[[s-h10]] [[ticket-41-h10-http-1-1-thin-helpers]]
