---
id: "h17-04-workspace-timeout"
title: "H17.04 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T17:56:31Z"
updated_at: "2026-09-04T18:13:14Z"
---

# H17.04 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H17.04 work; the host policy conformance harness stays green.

## Context

Roadmap ID **H17.04** (Optional JS/Node bridge for subset host APIs after native green). Review of [[s-h17-04]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_policy`) stayed green. If the hang comes from the H17.04 change, fix that optional JS/Node bridge subset so both the workspace check and the host policy conformance harness hold. Mark H17.04 `done` only when those tests are green. Not H17.01 `examples/http-echo` native HTTP/1.1, H17.02 echo integration, H17.03 `examples/todo` native serve, H17 parent remainder, H00 host I/O surface policy, H06.06 TCP listen/accept js hard-error, H09.03 DNS js hard-error, H10.07 HTTP listen helpers js hard-error, P04 flagship service, full Node-shaped `http` / `net` / `dgram` modules, or TLS / HTTP/2 / WebSocket on js. Do not re-open [[s-h17-04]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_policy --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_policy` still prints `test result: ok.` H17.04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H17.04), `tests/conformance/tests/host_policy.rs`, `tests/conformance/fixtures/host/policy`, `crates/draconic-check/src/host_api.rs`, optional JS/Node bridge subset as needed to unhang workspace tests after H17.04

## Links

[[s-h17-04-workspace-timeout]] [[ticket-163-h17-04-workspace-timeout]] [[s-h17-04]]
