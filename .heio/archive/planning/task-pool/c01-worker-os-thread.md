---
id: "c01-worker-os-thread"
title: "C01 worker / OS-thread isolate surface"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:05:01Z"
updated_at: "2026-09-02T14:16:31Z"
---

# C01 worker / OS-thread isolate surface

## Done

ROADMAP C01 is implemented test-first on both targets: spawn isolate from module path or fn entry, join for result/error, terminate without sharing a JS heap, native OS-thread backing; `concurrency/workers` fixtures are green and C01 is `done`.

## Context

Roadmap ID **C01** (Worker / OS thread: spawn isolate running module/fn; join/terminate; no shared JS heap by default). C01.01–C01.04 already land the per-class fixtures; this sitting unifies them as one honest worker surface on both targets. Tests under `tests/conformance` fixtures `concurrency/workers`. Harness `tests/conformance/tests/concurrency_workers.rs`. Mark C01 `done` only when those tests are green. Not C02–C06, Node/Web Worker API identity, or a shared JS heap.

## Verify

`cargo test -p draconic-conformance --test concurrency_workers` prints `test result: ok.` Workspace `cargo test --workspace` stays green. C01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (C01), `tests/conformance/fixtures/concurrency/workers`, `tests/conformance/tests/concurrency_workers.rs`, `crates/draconic-runtime`, js/native worker isolate paths as needed for the parent surface

## Links

[[s-c01]] [[ticket-71-c01-worker-os-thread-spawn-isolate]]
