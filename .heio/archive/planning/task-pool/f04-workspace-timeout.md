---
id: "f04-workspace-timeout"
title: "F04 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T15:24:30Z"
updated_at: "2026-09-04T16:07:16Z"
---

# F04 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP F04 work; the `ffi_link_static` conformance and integration harnesses stay green.

## Context

Roadmap ID **F04** (Link external static lib (`.a`); call one symbol). Review of [[s-f04]] left O3 unmet: `cargo test --workspace` timed out at 120s while O1 (`ffi_link_static` conformance) and O2 (`ffi_link_static` integration) stayed green. If the hang comes from the F04 change, fix that link-external-static-lib (`.a`) / call-one-symbol surface so both the workspace check and the `ffi_link_static` harnesses hold. Mark F04 `done` only when those tests are green. Not F04.01 Build links `.a`; resolve one C symbol, F04.02 Call linked static symbol end-to-end, F05 Link/load dynamic lib, F02 C callbacks, F03 C-compatible struct layout, F07 Bindgen from C headers, or F08 Unsafe/native-only FFI diagnostics. No js-target FFI (native-only until an explicit bridge row). Do not re-open [[s-f04]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test ffi_link_static --offline && cargo test -p draconic-integration-tests --test ffi_link_static --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test ffi_link_static` and `cargo test -p draconic-integration-tests --test ffi_link_static` still print `test result: ok.` F04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (F04), `tests/conformance/tests/ffi_link_static.rs`, `tests/conformance/fixtures/ffi/link_static`, `tests/integration/tests/ffi_link_static.rs`, `crates/draconic-backend-llvm`, `crates/draconic-cli`, link-external-static-lib / call-one-symbol surface as needed to unhang workspace tests after F04

## Links

[[s-f04-workspace-timeout]] [[ticket-141-f04-workspace-timeout]] [[s-f04]]
