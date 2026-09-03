---
id: "h17-native-host-cutover"
title: "H17 Success Programs & host cutover"
kind: task
status: completed
tags: []
created_at: "2026-09-02T22:24:44Z"
updated_at: "2026-09-03T05:16:34Z"
---

# H17 Success Programs & host cutover

## Done

ROADMAP H17 is implemented test-first on native: `examples/http-echo` is a pure Draconic native HTTP/1.1 server (no C host), integration starts echo, issues a client request, asserts status/body, and shuts down, and `examples/todo` serves from a Draconic native static host (no C host); http_echo and todo_server integration tests are green and H17 is `done`.

## Context

Roadmap ID **H17** (Success Programs & host cutover). H17.01–H17.03 already land `examples/http-echo` as pure Draconic native HTTP/1.1, the start-echo / client-request / assert / shutdown integration, and `examples/todo` C host cutover to Draconic native serve; this sitting unifies them as one honest native host cutover. Tests under `examples/http-echo`, `examples/todo`, `tests/integration/tests/http_echo.rs`, and `tests/integration/tests/todo_server.rs`. Mark H17 `done` only when those tests are green. Not H17.01, H17.02, H17.03 as separate atoms, H17.04, H10, P04, or P01.

## Verify

`cargo test -p draconic-integration-tests --test http_echo` prints `test result: ok.` `cargo test -p draconic-integration-tests --test todo_server` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H17 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H17), `examples/http-echo`, `examples/todo`, `tests/integration/tests/http_echo.rs`, `tests/integration/tests/todo_server.rs`, docs/adr/0008-host-io-sockets-first-http.md

## Links

[[s-h17]] [[ticket-48-h17-success-programs-host-cutover]]
