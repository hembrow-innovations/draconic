---
id: "h08-workspace-timeout"
title: "H08 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T16:54:50Z"
updated_at: "2026-09-04T17:05:47Z"
---

# H08 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H08 work; the host UDP conformance harness stays green.

## Context

Roadmap ID **H08** (UDP). Review of [[s-h08]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_udp`) stayed green. If the hang comes from the H08 change, fix that UDP bind/sendto/recvfrom/close and loopback surface so both the workspace check and the host UDP harness hold. Mark H08 `done` only when those tests are green. Not H08.01 UDP bind / sendto / recvfrom / close, H08.02 UDP loopback e2e, H06 TCP listen/accept/connect/read/write, H07 async socket I/O + job queue, H09 DNS, H10 HTTP/1.1 thin helpers, or H00 host I/O surface policy. Do not re-open [[s-h08]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_udp --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_udp` still prints `test result: ok.` H08 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H08), `tests/conformance/tests/host_udp.rs`, `tests/conformance/fixtures/host/net/udp`, `crates/draconic-backend-llvm/src/host_udp.rs`, `crates/draconic-runtime`, UDP surface as needed to unhang workspace tests after H08

## Links

[[s-h08-workspace-timeout]] [[ticket-154-h08-workspace-timeout]] [[s-h08]]
