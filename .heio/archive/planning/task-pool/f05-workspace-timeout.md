---
id: "f05-workspace-timeout"
title: "F05 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T15:28:42Z"
updated_at: "2026-09-04T16:12:00Z"
---

# F05 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP F05 work; the `ffi_link_dynamic` conformance and integration harnesses stay green.

## Context

Roadmap ID **F05** (Link/load dynamic lib (`.so`/`.dylib`/`.dll`); call one symbol). Review of [[s-f05]] left O3 unmet: `cargo test --workspace` timed out at 120s while O1 (`ffi_link_dynamic` conformance) and O2 (`ffi_link_dynamic` integration) stayed green. If the hang comes from the F05 change, fix that link/load-dynamic-lib (`.so`/`.dylib`/`.dll`) / call-one-symbol surface so both the workspace check and the `ffi_link_dynamic` harnesses hold. Mark F05 `done` only when those tests are green. Not F05.01 Load dynamic lib at link or runtime; resolve one symbol, F05.02 Call dynamic symbol; missing lib → typed error, F04 Link external static lib, F02 C callbacks, F03 C-compatible struct layout, F07 Bindgen from C headers, or F08 Unsafe/native-only FFI diagnostics. No js-target FFI (native-only until an explicit bridge row). Do not re-open [[s-f05]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test ffi_link_dynamic --offline && cargo test -p draconic-integration-tests --test ffi_link_dynamic --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test ffi_link_dynamic` and `cargo test -p draconic-integration-tests --test ffi_link_dynamic` still print `test result: ok.` F05 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (F05), `tests/conformance/tests/ffi_link_dynamic.rs`, `tests/conformance/fixtures/ffi/link_dynamic`, `tests/integration/tests/ffi_link_dynamic.rs`, `crates/draconic-backend-llvm`, `crates/draconic-cli`, link/load-dynamic-lib / call-one-symbol surface as needed to unhang workspace tests after F05

## Links

[[s-f05-workspace-timeout]] [[ticket-142-f05-workspace-timeout]] [[s-f05]]
