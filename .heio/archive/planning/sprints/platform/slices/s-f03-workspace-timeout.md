---
id: "s-f03-workspace-timeout"
title: "F03 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T15:18:39Z"
updated_at: "2026-09-04T16:06:30Z"
claimed-by: 3b2648a7-d6bf-4c3a-bec7-f3e062feb2d3
---

# F03 workspace tests finish

## Why

Review of [[s-f03]] left ROADMAP F03 unfinished: O1 (`ffi_layout`) held, but O2 `cargo test --workspace` timed out at 120s. The ffi location still needs the F03 Loop to leave the workspace green, not only the layout harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP F03 work. The `ffi_layout` harness stays green. If the hang comes from the F03 change, fix that C-compatible struct layout surface so both checks hold. Mark F03 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-f03]]**: that slice stays sealed `failed`
- **F03.01**: repr(C) struct field offsets match C ABI for scalars (already `done`)
- **F03.02**: Pass/return struct by value or pointer across FFI (already `done`)
- **F01**: `extern "C"` call out (scalar args/returns)
- **F02**: C callbacks (Draconic fn as `extern "C"` pointer)
- **F04**: Link external static lib
- **F05**: Load dynamic lib
- **F08**: Unsafe/native-only FFI diagnostics
- js FFI or a silent JS struct polyfill (native-only; other backend hard-errors)

## Oracle checklist

- [x] O1: workspace tests finish after the F03 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test ffi_layout --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=5c1d5dc7679fa7ff bytes=93759 at=2026-09-04T16:06:18.973Z

- [x] O2: F03 layout offsets and pass/return stay green on the declared native target through the ffi layout harness
  CHECK: cargo test -p draconic-conformance --test ffi_layout
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=e4f86f03ecf5ec74 bytes=3003 at=2026-09-04T16:06:20.689Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[f03-workspace-timeout]]`

## See also

ROADMAP.md F03, `tests/conformance/tests/ffi_layout.rs`, `tests/conformance/fixtures/ffi/layout`, `crates/draconic-backend-llvm`, docs/adr/0003-gc-runtime-and-dual-worlds.md, CONTEXT.md, [[ffi]], [[s-f03]], [[ticket-140-f03-workspace-timeout]].
