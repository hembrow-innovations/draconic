---
id: "f02-workspace-timeout"
title: "F02 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T15:16:35Z"
updated_at: "2026-09-04T15:58:36Z"
---

# F02 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP F02 work; the `ffi_callback` harness stays green.

## Context

Roadmap ID **F02** (C callbacks: Draconic fn as `extern "C"` pointer; host invokes). Review of [[s-f02]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`ffi_callback`) stayed green. If the hang comes from the F02 change, fix that C-callback surface so both the workspace check and the `ffi_callback` harness hold. Mark F02 `done` only when those tests are green. Not F02.01 export Draconic fn as C function pointer, F02.02 host invoke with scalar args, F01 `extern "C"` call out, F03 C-compatible struct layout, F04 link external static lib, F05 load dynamic lib, or F08 unsafe/native-only FFI diagnostics. No js FFI or a silent JS callback polyfill (native-only; other backend hard-errors). Do not re-open [[s-f02]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test ffi_callback --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test ffi_callback` still prints `test result: ok.` F02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (F02), `tests/conformance/tests/ffi_callback.rs`, `tests/conformance/fixtures/ffi/callback`, `crates/draconic-backend-llvm`, C-callback surface as needed to unhang workspace tests after F02

## Links

[[s-f02-workspace-timeout]] [[ticket-139-f02-workspace-timeout]] [[s-f02]]
