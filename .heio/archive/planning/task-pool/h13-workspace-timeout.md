---
id: "h13-workspace-timeout"
title: "H13 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T17:32:44Z"
updated_at: "2026-09-04T17:53:18Z"
---

# H13 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H13 work; the host HTTP/2 conformance harness stays green.

## Context

Roadmap ID **H13** (HTTP/2). Review of [[s-h13]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_http2`) stayed green. If the hang comes from the H13 change, fix that HTTP/2 preface and single-stream request/response surface so both the workspace check and the host HTTP/2 harness hold. Mark H13 `done` only when those tests are green. Not H13.01 HTTP/2 preface + single stream request/response, H06 TCP listen/accept/connect/read/write, H10 HTTP/1.1 thin helpers (plaintext), H11 TLS, H12 WebSocket, H00 host I/O surface policy, multiplexed streams, push, or a full HTTP/2 stack beyond the single-stream helpers. Do not re-open [[s-h13]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_http2 --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_http2` still prints `test result: ok.` H13 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H13), `tests/conformance/tests/host_http2.rs`, `tests/conformance/fixtures/host/http2`, `crates/draconic-backend-llvm/src/host_http2.rs`, `crates/draconic-runtime`, HTTP/2 preface and single-stream request/response surface as needed to unhang workspace tests after H13

## Links

[[s-h13-workspace-timeout]] [[ticket-159-h13-workspace-timeout]] [[s-h13]]
