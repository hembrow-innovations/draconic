---
id: "f03-workspace-timeout"
title: "F03 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T15:20:00Z"
updated_at: "2026-09-04T15:59:00Z"
---

# F03 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP F03 work; the `ffi_layout` harness stays green.

## Context

Roadmap ID **F03** (C-compatible struct layout (repr(C)-style); read/write both sides). Review of [[s-f03]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`ffi_layout`) stayed green. If the hang comes from the F03 change, fix that C-compatible struct layout surface so both the workspace check and the `ffi_layout` harness hold. Mark F03 `done` only when those tests are green. Not F03.01 repr(C) struct field offsets match C ABI for scalars, F03.02 pass/return struct by value or pointer across FFI, F01 `extern "C"` call out, F02 C callbacks, F04 link external static lib, F05 load dynamic lib, or F08 unsafe/native-only FFI diagnostics. No js FFI or a silent JS struct polyfill (native-only; other backend hard-errors). Do not re-open [[s-f03]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test ffi_layout --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test ffi_layout` still prints `test result: ok.` F03 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (F03), `tests/conformance/tests/ffi_layout.rs`, `tests/conformance/fixtures/ffi/layout`, `crates/draconic-backend-llvm`, C-compatible struct layout surface as needed to unhang workspace tests after F03

## Links

[[s-f03-workspace-timeout]] [[ticket-140-f03-workspace-timeout]] [[s-f03]]
