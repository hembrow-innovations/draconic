---
id: "s-f02-workspace-timeout"
title: "F02 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T15:15:04Z"
updated_at: "2026-09-04T16:04:19Z"
claimed-by: b25f82bd-74ea-4384-a775-0ddd65bb1c10
---

# F02 workspace tests finish

## Why

Review of [[s-f02]] left ROADMAP F02 unfinished: O1 (`ffi_callback`) held, but O2 `cargo test --workspace` timed out at 120s. The ffi location still needs the F02 Loop to leave the workspace green, not only the callback harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP F02 work. The `ffi_callback` harness stays green. If the hang comes from the F02 change, fix that C-callback surface so both checks hold. Mark F02 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-f02]]**: that slice stays sealed `failed`
- **F02.01**: Export Draconic fn as C function pointer (already `done`)
- **F02.02**: Host invokes callback with scalar args; return value observed (already `done`)
- **F01**: `extern "C"` call out (scalar args/returns)
- **F03**: C-compatible struct layout
- **F04**: Link external static lib
- **F05**: Load dynamic lib
- **F08**: Unsafe/native-only FFI diagnostics
- js FFI or a silent JS callback polyfill (native-only; other backend hard-errors)

## Oracle checklist

- [x] O1: workspace tests finish after the F02 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test ffi_callback --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=b484eba5e6351b30 bytes=93777 at=2026-09-04T16:03:58.401Z

- [x] O2: F02 export-as-fnptr and host scalar invoke stay green on the declared native target through the ffi callback harness
  CHECK: cargo test -p draconic-conformance --test ffi_callback
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=94aca049ea0ef751 bytes=3021 at=2026-09-04T16:03:59.083Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[f02-workspace-timeout]]`

## See also

ROADMAP.md F02, `tests/conformance/tests/ffi_callback.rs`, `tests/conformance/fixtures/ffi/callback`, `crates/draconic-backend-llvm`, docs/adr/0003-gc-runtime-and-dual-worlds.md, CONTEXT.md, [[ffi]], [[s-f02]], [[ticket-139-f02-workspace-timeout]].
