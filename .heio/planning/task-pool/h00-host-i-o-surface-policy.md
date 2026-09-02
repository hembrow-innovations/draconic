---
id: "h00-host-i-o-surface-policy"
title: "H00 Host I/O surface policy: module/global shape, error model, js hard-error vs polyfill matrix"
kind: task
status: ready
tags: []
created_at: "2026-09-02T22:14:18Z"
updated_at: "2026-09-02T22:14:18Z"
---

# H00 Host I/O surface policy: module/global shape, error model, js hard-error vs polyfill matrix

## Done

ROADMAP H00 is implemented test-first on both targets: Host APIs have a locked module/global shape, native host failures use the designed error model, and JS-unavailable host APIs hard-error (no silent polyfill) until an explicit bridge row; `host/policy` conformance and runtime crate tests are green and H00 is `done`.

## Context

Roadmap ID **H00** (Host I/O surface policy: module/global shape, error model, js hard-error vs polyfill matrix). H00.01–H00.03 already land the registry, Runtime ABI scaffold, and bytes boundary; this sitting unifies them as one honest policy on both targets. Tests under `tests/conformance/host/policy` and `crates/draconic-runtime`. Harness `tests/conformance/tests/host_policy.rs`. Mark H00 `done` only when those tests are green. Not H00.01, H00.02, H00.03, H01–H16, H17.04, or R02.

## Verify

`cargo test -p draconic-conformance --test host_policy` prints `test result: ok.` `cargo test -p draconic-runtime --lib` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H00 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H00), `tests/conformance/fixtures/host/policy`, `tests/conformance/tests/host_policy.rs`, `crates/draconic-runtime`, `crates/draconic-check/src/host_api.rs`, host policy / Runtime ABI paths as needed for the parent surface

## Links

[[s-h00]] [[ticket-31-h00-host-i-o-surface-policy]]
