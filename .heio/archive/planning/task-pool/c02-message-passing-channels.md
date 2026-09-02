---
id: "c02-message-passing-channels"
title: "C02 message-passing channel surface"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:06:09Z"
updated_at: "2026-09-02T14:16:16Z"
---

# C02 message-passing channel surface

## Done

ROADMAP C02 is implemented test-first on both targets: send/recv scalars and strings, clone plain objects without sharing heap refs, bounded-buffer backpressure, worker ↔ parent e2e; `concurrency/channels` fixtures are green and C02 is `done`.

## Context

Roadmap ID **C02** (Message-passing channels: send/recv; structured-clone or transfer policy; bounded buffer as designed). C02.01–C02.04 already land the per-class fixtures; this sitting unifies them as one honest channel surface on both targets. Tests under `tests/conformance` fixtures `concurrency/channels`. Harness `tests/conformance/tests/concurrency_channels.rs`. Mark C02 `done` only when those tests are green. Not C01 parent row, C03, C06, or a user-facing shared JS heap.

## Verify

`cargo test -p draconic-conformance --test concurrency_channels` prints `test result: ok.` Workspace `cargo test --workspace` stays green. C02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (C02), `tests/conformance/fixtures/concurrency/channels`, `tests/conformance/tests/concurrency_channels.rs`, `crates/draconic-backend-llvm/src/host_channels.rs`, `crates/draconic-backend-llvm/src/host_worker_channels.rs`, js/native channel paths as needed for the parent surface

## Links

[[s-c02]] [[ticket-72-c02-message-passing-channels-send-recv]]
