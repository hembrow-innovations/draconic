---
id: "h10-workspace-timeout"
title: "H10 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T17:28:30Z"
updated_at: "2026-09-04T17:31:37Z"
---

# H10 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H10 work; the host HTTP/1.1 conformance harness stays green.

## Context

Roadmap ID **H10** (HTTP/1.1 thin helpers). Review of [[s-h10]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_http`) stayed green. If the hang comes from the H10 change, fix that HTTP/1.1 thin-helper surface so both the workspace check and the host HTTP harness hold. Mark H10 `done` only when those tests are green. Not H10.01 HTTP/1.1 request parse: line + headers + bounded body, H10.02 HTTP/1.1 response write: status + headers + body, H10.03 HTTP/1.1 server one-shot, H10.04 HTTP/1.1 keep-alive optional, H10.05 HTTP/1.1 client on connected TCP, H10.06 chunked transfer encoding, H10.07 HTTP listen helpers hard-error on js, H06 TCP listen/accept/connect/read/write, H11 TLS, H12 WebSocket, H13 HTTP/2, or H17 Success Programs & host cutover. Do not re-open [[s-h10]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_http --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_http` still prints `test result: ok.` H10 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H10), `tests/conformance/tests/host_http.rs`, `tests/conformance/fixtures/host/http`, `crates/draconic-backend-llvm/src/host_http.rs`, `crates/draconic-backend-llvm/src/host_http_server.rs`, `crates/draconic-runtime`, HTTP/1.1 thin-helper surface as needed to unhang workspace tests after H10

## Links

[[s-h10-workspace-timeout]] [[ticket-156-h10-workspace-timeout]] [[s-h10]]
