---
id: "f08-workspace-timeout"
title: "F08 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T15:39:33Z"
updated_at: "2026-09-04T16:19:07Z"
---

# F08 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP F08 work; the `ffi/policy` conformance fixtures stay green.

## Context

Roadmap ID **F08** (Unsafe/native-only FFI diagnostics; JS hard-error; clear spans). Review of [[s-f08]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`ffi_policy` conformance) stayed green. If the hang comes from the F08 change, fix that unsafe/native-only FFI diagnostics / JS hard-error / clear-spans surface so both the workspace check and the `ffi/policy` harnesses hold. Mark F08 `done` only when those tests are green. Not F08.01 FFI/extern on js → hard diagnostic, F08.02 Clear spans + codes for bad extern signatures / unsupported types, F01 `extern "C"` call out, F02 C callbacks, F03 C-compatible struct layout, F04 Link external static lib, F05 Load dynamic lib, F07 Bindgen from C headers, or F09 wasm32/wasi emit. No js FFI polyfill or silent wrong code on the other backend. Do not re-open [[s-f08]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test ffi_policy --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test ffi_policy` still prints `test result: ok.` F08 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (F08), `tests/conformance/tests/ffi_policy.rs`, `tests/conformance/fixtures/ffi/policy`, `crates/draconic-check`, unsafe/native-only FFI diagnostics / JS hard-error / clear-spans surface as needed to unhang workspace tests after F08

## Links

[[s-f08-workspace-timeout]] [[ticket-144-f08-workspace-timeout]] [[s-f08]]
