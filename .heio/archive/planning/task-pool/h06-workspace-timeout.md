---
id: "h06-workspace-timeout"
title: "H06 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T16:39:15Z"
updated_at: "2026-09-04T16:47:53Z"
---

# H06 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H06 work; the host TCP conformance harness stays green.

## Context

Roadmap ID **H06** (TCP sockets (sockets-first)). Review of [[s-h06]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_tcp`) stayed green. If the hang comes from the H06 change, fix that TCP sockets (sockets-first) surface so both the workspace check and the host TCP harness hold. Mark H06 `done` only when those tests are green. Not H06.01 TCP listen bind/backlog/close/ephemeral, H06.02 TCP accept → connection handle / peer address, H06.03 TCP connect dial host:port / refused/timeout, H06.04 TCP read/write bytes / partial read / close/shutdown, H06.05 TCP loopback e2e echo, H06.06 TCP listen/accept hard-error on js, H07 async socket I/O + job queue, H08 UDP, or H00 host I/O surface policy. Do not re-open [[s-h06]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_tcp --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_tcp` still prints `test result: ok.` H06 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H06), `tests/conformance/tests/host_tcp.rs`, `tests/conformance/fixtures/host/net/tcp`, `crates/draconic-backend-llvm/src/host_tcp.rs`, `crates/draconic-runtime`, TCP sockets surface as needed to unhang workspace tests after H06

## Links

[[s-h06-workspace-timeout]] [[ticket-152-h06-workspace-timeout]] [[s-h06]]
