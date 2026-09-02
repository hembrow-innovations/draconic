---
id: "h13-http-2-later-not-v1"
title: "H13 HTTP/2 (later; not v1 bar)"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:31:00Z"
updated_at: "2026-09-02T13:31:00Z"
---

# H13 HTTP/2 (later; not v1 bar)

## Done

ROADMAP H13 is implemented test-first on native: emit HTTP/2 client and server connection prefaces (`http2ClientPreface` / `http2ServerPreface` / `http2SettingsAck`), encode and parse a single-stream request and response (`http2EncodeRequest` / `http2EncodeResponse` / `http2ParseRequest` / `http2ParseResponse`), and complete a one-buffer client open plus server reply (`http2ClientOpen` / `http2ServerReply`); `host/http2` fixtures are green and H13 is `done`.

## Context

Roadmap ID **H13** (HTTP/2, later; not v1 bar). H13.01 already lands preface + single-stream request/response; this sitting unifies them as one honest HTTP/2 surface on native. Tests under `tests/conformance` fixtures `host/http2`. Harness `tests/conformance/tests/host_http2.rs`. Mark H13 `done` only when those tests are green. Not H13.01 as a separate atom, H06, H10, H11, H12, H00, or multiplexed streams / push / a full HTTP/2 stack beyond the single-stream helpers.

## Verify

`cargo test -p draconic-conformance --test host_http2` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H13 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H13), `tests/conformance/fixtures/host/http2`, `tests/conformance/tests/host_http2.rs`, `crates/draconic-backend-llvm/src/host_http2.rs`, `crates/draconic-runtime`, native HTTP/2 paths as needed for the parent surface

## Links

[[s-h13]] [[ticket-44-h13-http-2-later-not-v1]]
