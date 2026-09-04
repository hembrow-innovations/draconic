---
id: "c03-workspace-timeout"
title: "C03 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T13:37:06Z"
updated_at: "2026-09-04T13:43:32Z"
---

# C03 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP C03 work; the `concurrency_sync` harness and `draconic-runtime` lib tests stay green.

## Context

Roadmap ID **C03** (`once` / thread-safe init; mutex only if Runtime internals need it). Review of [[s-c03]] left O3 unmet: `cargo test --workspace` timed out at 120s while `concurrency_sync` and `draconic-runtime --lib` stayed green. If the hang comes from the C03 change, fix that once / thread-safe init surface so those checks hold. Mark C03 `done` only when those tests are green. Not C03.01–C03.02 as separate atoms, C01/C02/C04–C06, a user-facing shared JS heap, or a public mutex Host API. Do not re-open [[s-c03]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test concurrency_sync --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test concurrency_sync` and `cargo test -p draconic-runtime --lib` still print `test result: ok.` C03 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (C03), `tests/conformance/fixtures/concurrency/sync`, `tests/conformance/tests/concurrency_sync.rs`, `crates/draconic-runtime/src/host_once_tests.rs`, `crates/draconic-runtime/src/host_mutex_tests.rs`, `crates/draconic-backend-llvm/src/host_once.rs`, once / Runtime-internal mutex paths as needed to unhang workspace tests after C03

## Links

[[s-c03-workspace-timeout]] [[ticket-122-c03-workspace-timeout]] [[s-c03]]
