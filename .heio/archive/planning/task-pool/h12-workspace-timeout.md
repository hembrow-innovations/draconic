---
id: "h12-workspace-timeout"
title: "H12 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T17:24:31Z"
updated_at: "2026-09-04T17:48:30Z"
---

# H12 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H12 work; the host WebSocket conformance harness stays green.

## Context

Roadmap ID **H12** (WebSocket). Review of [[s-h12]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_ws`) stayed green. If the hang comes from the H12 change, fix that WebSocket handshake, frames, and client echo surface so both the workspace check and the host WebSocket harness hold. Mark H12 `done` only when those tests are green. Not H12.01 WebSocket handshake (HTTP/1.1 upgrade) server-side, H12.02 WebSocket frames: text/binary; close; ping/pong, H12.03 WebSocket client dial + echo e2e, H06 TCP listen/accept/connect/read/write, H10 HTTP/1.1 thin helpers (plaintext), H11 TLS, H13 HTTP/2, or H00 host I/O surface policy. Do not re-open [[s-h12]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_ws --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_ws` still prints `test result: ok.` H12 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H12), `tests/conformance/tests/host_ws.rs`, `tests/conformance/fixtures/host/net/ws`, `crates/draconic-backend-llvm/src/host_ws.rs`, `crates/draconic-backend-llvm/src/host_ws_e2e.rs`, `crates/draconic-runtime`, WebSocket handshake, frames, and client echo surface as needed to unhang workspace tests after H12

## Links

[[s-h12-workspace-timeout]] [[ticket-158-h12-workspace-timeout]] [[s-h12]]
