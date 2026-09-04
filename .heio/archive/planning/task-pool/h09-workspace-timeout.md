---
id: "h09-workspace-timeout"
title: "H09 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T17:16:10Z"
updated_at: "2026-09-04T17:20:15Z"
---

# H09 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H09 work; the host DNS conformance harness stays green.

## Context

Roadmap ID **H09** (DNS). Review of [[s-h09]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_dns`) stayed green. If the hang comes from the H09 change, fix that DNS lookup and connect-by-name surface so both the workspace check and the host DNS harness hold. Mark H09 `done` only when those tests are green. Not H09.01 DNS lookup hostname → addresses / failure errors, H09.02 connect-by-name (H09.01 + H06.03), H09.03 DNS on js hard-error / deferred Node polyfill, H06 TCP listen/accept/connect/read/write, H08 UDP, H10 HTTP/1.1 thin helpers, or H17.04 optional JS/Node bridge for subset host APIs. Do not re-open [[s-h09]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_dns --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_dns` still prints `test result: ok.` H09 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H09), `tests/conformance/tests/host_dns.rs`, `tests/conformance/fixtures/host/net/dns`, `crates/draconic-backend-llvm/src/host_dns.rs`, `crates/draconic-runtime`, DNS lookup and connect-by-name surface as needed to unhang workspace tests after H09

## Links

[[s-h09-workspace-timeout]] [[ticket-155-h09-workspace-timeout]] [[s-h09]]
