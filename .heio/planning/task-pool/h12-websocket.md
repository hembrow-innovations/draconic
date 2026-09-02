---
id: "h12-websocket"
title: "H12 WebSocket"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:30:18Z"
updated_at: "2026-09-02T13:30:18Z"
---

# H12 WebSocket

## Done

ROADMAP H12 is implemented test-first on native: complete a WebSocket handshake (HTTP/1.1 upgrade) server-side (`wsHandshakeResponse`), encode and decode RFC 6455 frames (text, binary, close, ping, pong), and dial as a client (`wsClientHandshakeRequest` / `wsClientCheckAccept` / `wsEncodeTextClient`) with loopback text echo; `host/net/ws` fixtures are green and H12 is `done`.

## Context

Roadmap ID **H12** (WebSocket). H12.01–H12.03 already land server-side HTTP/1.1 upgrade handshake, text/binary/close/ping/pong frames, and client dial + loopback text echo; this sitting unifies them as one honest WebSocket surface on native. Tests under `tests/conformance` fixtures `host/net/ws`. Harness `tests/conformance/tests/host_ws.rs`. Mark H12 `done` only when those tests are green. Not H06, H10, H11, H13, or H00.

## Verify

`cargo test -p draconic-conformance --test host_ws` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H12 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H12), `tests/conformance/fixtures/host/net/ws`, `tests/conformance/tests/host_ws.rs`, `crates/draconic-backend-llvm/src/host_ws.rs`, `crates/draconic-backend-llvm/src/host_ws_e2e.rs`, `crates/draconic-runtime`, native WebSocket paths as needed for the parent surface

## Links

[[s-h12]] [[ticket-43-h12-websocket]]
