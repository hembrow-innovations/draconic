---
id: "h14-workspace-timeout"
title: "H14 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T17:36:35Z"
updated_at: "2026-09-04T17:57:08Z"
---

# H14 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H14 work; the host process signal conformance harness and the runtime crate signal tests stay green.

## Context

Roadmap ID **H14** (Signals). Review of [[s-h14]] left O3 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_process signal`) and O2 (`host_signal`) stayed green. If the hang comes from the H14 change, fix that SIGINT/SIGTERM watch, ignore, and restore-default surface so both the workspace check and the signal tests hold. Mark H14 `done` only when those tests are green. Not H14.01 Signal watch SIGINT/SIGTERM → handler/job, H14.02 Signal ignore / restore default, H01 process args, env, exit, H15 subprocess, H16 OS misc, H00 host I/O surface policy, or js signal APIs / a Node polyfill. Do not re-open [[s-h14]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_process --offline signal && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_process signal` still prints `test result: ok.` `cargo test -p draconic-runtime host_signal` still prints `test result: ok.` H14 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H14), `tests/conformance/tests/host_process.rs`, `tests/conformance/fixtures/host/process`, `crates/draconic-backend-llvm/src/host_signals.rs`, `crates/draconic-runtime`, SIGINT/SIGTERM watch, ignore, and restore-default surface as needed to unhang workspace tests after H14

## Links

[[s-h14-workspace-timeout]] [[ticket-160-h14-workspace-timeout]] [[s-h14]]
