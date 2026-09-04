---
id: "c01-workspace-timeout"
title: "C01 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T13:28:32Z"
updated_at: "2026-09-04T13:35:36Z"
---

# C01 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP C01 work; the `concurrency_workers` harness stays green.

## Context

Roadmap ID **C01** (Worker / OS thread: spawn isolate running module/fn; join/terminate; no shared JS heap by default). Review of [[s-c01]] left O2 unmet: `cargo test --workspace` timed out at 120s while `concurrency_workers` stayed green. If the hang comes from the C01 change, fix that worker/isolate surface so both checks hold. Mark C01 `done` only when those tests are green. Not C01.01–C01.04 as separate atoms, C02–C06, Node/Web Worker API identity, or a shared JS heap. Do not re-open [[s-c01]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test concurrency_workers --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test concurrency_workers` still prints `test result: ok.` C01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (C01), `tests/conformance/fixtures/concurrency/workers`, `tests/conformance/tests/concurrency_workers.rs`, `crates/draconic-runtime`, worker/isolate paths as needed to unhang workspace tests after C01

## Links

[[s-c01-workspace-timeout]] [[ticket-120-c01-workspace-timeout]] [[s-c01]]
