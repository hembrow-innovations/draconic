---
id: "h11-workspace-timeout"
title: "H11 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T17:19:58Z"
updated_at: "2026-09-04T17:37:07Z"
---

# H11 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H11 work; the host TLS conformance harness stays green.

## Context

Roadmap ID **H11** (TLS). Review of [[s-h11]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_tls`) stayed green. If the hang comes from the H11 change, fix that TLS client/server wrap and HTTPS loopback surface so both the workspace check and the host TLS harness hold. Mark H11 `done` only when those tests are green. Not H11.01 TLS client wrap trust roots / insecure-test, H11.02 TLS server wrap cert/key handshake, H11.03 HTTPS HTTP/1.1 over TLS loopback, H06 TCP listen/accept/connect/read/write, H10 HTTP/1.1 thin helpers (plaintext), H12 WebSocket, H13 HTTP/2, or H00 host I/O surface policy. Do not re-open [[s-h11]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_tls --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_tls` still prints `test result: ok.` H11 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H11), `tests/conformance/tests/host_tls.rs`, `tests/conformance/fixtures/host/net/tls`, `crates/draconic-backend-llvm/src/host_tcp.rs`, `crates/draconic-backend-llvm/src/host_http_server.rs`, `crates/draconic-runtime`, TLS client/server wrap and HTTPS loopback surface as needed to unhang workspace tests after H11

## Links

[[s-h11-workspace-timeout]] [[ticket-157-h11-workspace-timeout]] [[s-h11]]
