---
id: "c02-workspace-timeout"
title: "C02 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T13:33:07Z"
updated_at: "2026-09-04T13:39:54Z"
---

# C02 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP C02 work; the `concurrency_channels` harness stays green.

## Context

Roadmap ID **C02** (Message-passing channels: send/recv; structured-clone or transfer policy; bounded buffer as designed). Review of [[s-c02]] left O2 unmet: `cargo test --workspace` timed out at 120s while `concurrency_channels` stayed green. If the hang comes from the C02 change, fix that channel send/recv surface so both checks hold. Mark C02 `done` only when those tests are green. Not C02.01–C02.04 as separate atoms, C01/C03–C06, or a user-facing shared JS heap across isolates. Do not re-open [[s-c02]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test concurrency_channels --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test concurrency_channels` still prints `test result: ok.` C02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (C02), `tests/conformance/fixtures/concurrency/channels`, `tests/conformance/tests/concurrency_channels.rs`, `crates/draconic-backend-llvm/src/host_channels.rs`, `crates/draconic-backend-llvm/src/host_worker_channels.rs`, channel send/recv paths as needed to unhang workspace tests after C02

## Links

[[s-c02-workspace-timeout]] [[ticket-121-c02-workspace-timeout]] [[s-c02]]
