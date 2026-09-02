---
id: "f08-unsafe-native-only-ffi-diagnostics"
title: "F08 unsafe/native-only FFI diagnostics"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:20:28Z"
updated_at: "2026-09-02T13:20:28Z"
---

# F08 unsafe/native-only FFI diagnostics

## Done

ROADMAP F08 is implemented test-first on both targets: `extern "C"` / FFI on js is a hard diagnostic; bad extern signatures and unsupported types emit clear spans and codes on js and native; `ffi/policy` fixtures are green and F08 is `done`.

## Context

Roadmap ID **F08** (Unsafe/native-only FFI diagnostics; JS hard-error; clear spans). F08.01–F08.02 already land js hard-error for FFI/extern and clear spans plus codes for bad extern signatures / unsupported types; this sitting unifies them as one honest FFI policy surface on both targets. Tests under `tests/conformance` fixtures `ffi/policy`. Harness `tests/conformance/tests/ffi_policy.rs`. Mark F08 `done` only when those tests are green. Not F01–F05, F07, F09, js FFI polyfill, or silent wrong code on the other backend.

## Verify

`cargo test -p draconic-conformance --test ffi_policy` prints `test result: ok.` Workspace `cargo test --workspace` stays green. F08 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (F08), `tests/conformance/fixtures/ffi/policy`, `tests/conformance/tests/ffi_policy.rs`, `crates/draconic-check`, js/native FFI diagnostic paths as needed for the parent surface

## Links

[[s-f08]] [[ticket-69-f08-unsafe-native-only-ffi-diagnostics]]
