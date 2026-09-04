---
id: "c06-workspace-timeout"
title: "C06 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T13:55:58Z"
updated_at: "2026-09-04T14:04:33Z"
---

# C06 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP C06 work; the `concurrency_atomics` harness stays green.

## Context

Roadmap ID **C06** (Optional later: shared-memory atomics (advanced; not v1 bar)). Review of [[s-c06]] left O2 unmet: `cargo test --workspace` timed out at 120s while `concurrency_atomics` stayed green. If the hang comes from the C06 change, fix that shared-memory atomics surface so both checks hold. Mark C06 `done` only when those tests are green. Not C01–C05, full ECMA-262 `SharedArrayBuffer` / `Atomics` Test262 allowlist (E19 / S02), Node `worker_threads` SharedArrayBuffer identity, or a shared JS heap. Do not re-open [[s-c06]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test concurrency_atomics --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test concurrency_atomics` still prints `test result: ok.` C06 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (C06), `tests/conformance/tests/concurrency_atomics.rs`, `tests/conformance/fixtures/concurrency/atomics`, `crates/draconic-runtime`, `crates/draconic-backend-llvm`, `crates/draconic-check/src/host_api.rs`, shared-memory atomics paths as needed to unhang workspace tests after C06

## Links

[[s-c06-workspace-timeout]] [[ticket-125-c06-workspace-timeout]] [[s-c06]]
