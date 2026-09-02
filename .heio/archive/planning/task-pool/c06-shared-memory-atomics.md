---
id: "c06-shared-memory-atomics"
title: "C06 shared-memory atomics surface"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:13:03Z"
updated_at: "2026-09-03T01:30:00Z"
---

# C06 shared-memory atomics surface

## Done

ROADMAP C06 is implemented test-first on native: a Program can allocate a shared integer memory buffer visible to a worker isolate, then atomic load, store, add, compare-exchange, wait, and notify on that buffer without sharing a JS heap; the js target hard-errors (native-only); `concurrency/atomics` fixtures are green and C06 is `done`.

## Context

Roadmap ID **C06** (Optional later: shared-memory atomics (advanced; not v1 bar)). Worker isolates already exist (C01) and do not share a JS heap; this sitting is the opt-in integer buffer those isolates can share with atomic ops on native. Tests under `tests/conformance` fixtures `concurrency/atomics`. Harness `tests/conformance/tests/concurrency_atomics.rs`. Mark C06 `done` only when those tests are green. Not C01–C05, a shared JS heap, full ECMA-262 `SharedArrayBuffer` / `Atomics` Test262 allowlist (E19 / S02), or Node `worker_threads` SharedArrayBuffer identity.

## Verify

`cargo test -p draconic-conformance --test concurrency_atomics` prints `test result: ok.` Workspace `cargo test --workspace` stays green. C06 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (C06), `tests/conformance/fixtures/concurrency/atomics`, `tests/conformance/tests/concurrency_atomics.rs`, `crates/draconic-runtime`, `crates/draconic-backend-llvm`, `crates/draconic-check/src/host_api.rs`, native shared-buffer / atomics paths as needed for the surface

## Links

[[s-c06]] [[ticket-76-c06-optional-later-shared-memory-atomics]]
