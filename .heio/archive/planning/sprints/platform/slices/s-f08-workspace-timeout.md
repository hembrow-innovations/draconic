---
id: "s-f08-workspace-timeout"
title: "F08 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T15:37:04Z"
updated_at: "2026-09-04T16:23:21Z"
claimed-by: 813924b6-34fc-4f72-904b-811de3517e6a
---

# F08 workspace tests finish

## Why

Review of [[s-f08]] left ROADMAP F08 unfinished: O1 (`ffi_policy`) held, but O2 `cargo test --workspace` timed out at 120s. The ffi location still needs the F08 Loop to leave the workspace green, not only the ffi/policy harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP F08 work. The `ffi/policy` conformance fixtures stay green. If the hang comes from the F08 change, fix that unsafe/native-only FFI diagnostics / JS hard-error / clear-spans surface so both checks hold. Mark F08 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-f08]]**: that slice stays sealed `failed`
- **F08.01**: FFI/extern on js → hard diagnostic (already `done`)
- **F08.02**: Clear spans + codes for bad extern signatures / unsupported types (already `done`)
- **F01**: `extern "C"` call out (scalar args/returns)
- **F02**: C callbacks
- **F03**: C-compatible struct layout
- **F04**: Link external static lib
- **F05**: Load dynamic lib
- **F07**: Bindgen from C headers
- **F09**: wasm32/wasi emit
- js FFI polyfill or silent wrong code on the other backend

## Oracle checklist

- [x] O1: workspace tests finish after the F08 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test ffi_policy --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=35197c8f3df7667b bytes=93979 at=2026-09-04T16:23:04.000Z

- [x] O2: F08 js hard-error and bad-extern spans/codes stay locked by the ffi/policy conformance fixtures
  CHECK: cargo test -p draconic-conformance --test ffi_policy
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=d3eab7cfca671ee6 bytes=3223 at=2026-09-04T16:23:04.440Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[f08-workspace-timeout]]`

## See also

ROADMAP.md F08, `tests/conformance/tests/ffi_policy.rs`, `tests/conformance/fixtures/ffi/policy`, `crates/draconic-check`, docs/adr/0003-gc-runtime-and-dual-worlds.md, CONTEXT.md, [[ffi]], [[s-f08]], [[ticket-144-f08-workspace-timeout]].
