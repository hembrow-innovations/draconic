---
id: "h00-workspace-timeout"
title: "H00 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T15:47:55Z"
updated_at: "2026-09-04T16:28:20Z"
---

# H00 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H00 work; the host policy conformance harness and the runtime crate tests stay green.

## Context

Roadmap ID **H00** (Host I/O surface policy: module/global shape, error model, js hard-error vs polyfill matrix). Review of [[s-h00]] left O3 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_policy`) and O2 (`draconic-runtime` lib) stayed green. If the hang comes from the H00 change, fix that host I/O surface policy so both the workspace check and the host policy / Runtime ABI harnesses hold. Mark H00 `done` only when those tests are green. Not H00.01 host API registry and js unsupported hard diagnostic, H00.02 Runtime ABI scaffold, H00.03 I/O bytes boundary, H01–H16 concrete host ops, H17.04 optional JS/Node bridge, or R02 permission grant/deny. Do not re-open [[s-h00]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_policy --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_policy` and `cargo test -p draconic-runtime --lib` still print `test result: ok.` H00 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H00), `tests/conformance/tests/host_policy.rs`, `tests/conformance/fixtures/host/policy`, `crates/draconic-runtime`, `crates/draconic-check/src/host_api.rs`, host I/O surface policy as needed to unhang workspace tests after H00

## Links

[[s-h00-workspace-timeout]] [[ticket-146-h00-workspace-timeout]] [[s-h00]]
