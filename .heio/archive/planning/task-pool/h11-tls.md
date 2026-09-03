---
id: "h11-tls"
title: "H11 TLS"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:29:48Z"
updated_at: "2026-09-03T05:16:34Z"
---

# H11 TLS

## Done

ROADMAP H11 is implemented test-first on native: wrap a TCP connection as a TLS client (`tlsClientWrap(conn, serverName, insecure)` with system trust roots when insecure=0 and skip-verify when insecure=1), wrap an accepted connection as a TLS server (`tlsServerWrap(conn, certPath, keyPath)` PEM), read/write/close via `tlsRead` / `tlsWrite` / `closeTls`, and complete HTTPS HTTP/1.1 over TLS on loopback; `host/net/tls` fixtures are green and H11 is `done`.

## Context

Roadmap ID **H11** (TLS). H11.01–H11.03 already land client wrap (trust roots / insecure-test), server wrap (PEM cert/key + handshake), and HTTPS HTTP/1.1 over TLS loopback; this sitting unifies them as one honest TLS surface on native. Tests under `tests/conformance` fixtures `host/net/tls`. Harness `tests/conformance/tests/host_tls.rs`. Mark H11 `done` only when those tests are green. Not H06, H10, H12, H13, or H00.

## Verify

`cargo test -p draconic-conformance --test host_tls` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H11 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H11), `tests/conformance/fixtures/host/net/tls`, `tests/conformance/tests/host_tls.rs`, `crates/draconic-backend-llvm/src/host_tcp.rs`, `crates/draconic-backend-llvm/src/host_http_server.rs`, `crates/draconic-runtime`, native TLS paths as needed for the parent surface

## Links

[[s-h11]] [[ticket-42-h11-tls]]
