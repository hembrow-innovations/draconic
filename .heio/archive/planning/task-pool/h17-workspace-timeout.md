---
id: "h17-workspace-timeout"
title: "H17 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T18:05:40Z"
updated_at: "2026-09-04T18:17:11Z"
---

# H17 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H17 work; the http-echo and todo native-serve integration tests stay green.

## Context

Roadmap ID **H17** (Success Programs & host cutover). Review of [[s-h17]] left O3 unmet: `cargo test --workspace` timed out at 120s while O1 (`http_echo` / `host_cutover`) and O2 (`todo_server`) stayed green. If the hang comes from the H17 change, fix that Success Programs & host cutover so both the workspace check and the http-echo / todo native-serve integration tests hold. Mark H17 `done` only when those tests are green. Not H17.01 `examples/http-echo` pure Draconic native HTTP/1.1, H17.02 echo integration, H17.03 `examples/todo` C host cutover → Draconic native serve, H17.04 optional JS/Node bridge, H10 HTTP/1.1 thin helpers, P04 flagship service, or P01 fizzbuzz. Do not re-open [[s-h17]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test http_echo --test host_cutover --offline && cargo test -p draconic-integration-tests --test todo_server --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-integration-tests --test http_echo --test host_cutover` still prints `test result: ok.` `cargo test -p draconic-integration-tests --test todo_server` still prints `test result: ok.` H17 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H17), `examples/http-echo`, `examples/todo`, `tests/integration/tests/http_echo.rs`, `tests/integration/tests/todo_server.rs`, Success Programs & host cutover as needed to unhang workspace tests after H17

## Links

[[s-h17-workspace-timeout]] [[ticket-164-h17-workspace-timeout]] [[s-h17]]
