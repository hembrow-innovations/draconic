---
id: "s-f09-workspace-timeout"
title: "F09 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T15:43:31Z"
updated_at: "2026-09-04T16:30:02Z"
claimed-by: cb4d08be-2806-4de4-8cd5-8e8db389f3a4
---

# F09 workspace tests finish

## Why

Review of [[s-f09]] left ROADMAP F09 unfinished: O1 (`wasm32_wasi` LLVM backend) and O2 (`wasm32_wasi` integration) held, but O3 `cargo test --workspace` timed out at 120s. The ffi location still needs the F09 Loop to leave the workspace green, not only the wasm32/wasi emit+link harnesses.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP F09 work. The LLVM backend wasm32/wasi emit tests and the integration emit+link smoke stay green. If the hang comes from the F09 change, fix that wasm32/wasi emit + link smoke surface so both checks hold. Mark F09 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-f09]]**: that slice stays sealed `failed`
- **F01**: `extern "C"` call out (scalar args/returns)
- **F04**: Link external static lib (`.a`)
- **F05**: Load dynamic lib
- **F06**: Manual `extern` decls
- **F07**: Bindgen from C headers
- **F08**: Unsafe/native-only FFI diagnostics
- **D04**: linux/darwin/windows × amd64/arm64 cross-compile matrix
- a third WASM-only IR (ADR-0002 rejected that)
- full WASI libc / preview2 host, browser wasm, or wasmtime identity
- changing the F v1 done bar to require F09

## Oracle checklist

- [x] O1: workspace tests finish after the F09 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-backend-llvm wasm32_wasi --offline && cargo test -p draconic-integration-tests --test wasm32_wasi --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=58936ef936196ab4 bytes=95695 at=2026-09-04T16:29:39.853Z

- [x] O2: F09 LLVM backend emits wasm32/wasi
  CHECK: cargo test -p draconic-backend-llvm wasm32_wasi
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=9b4d4e60dada74b3 bytes=1999 at=2026-09-04T16:29:39.966Z

- [x] O3: F09 wasm32/wasi emit + link smoke is locked by the integration suite
  CHECK: cargo test -p draconic-integration-tests --test wasm32_wasi
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=d361646c0fba6b0e bytes=2940 at=2026-09-04T16:29:40.079Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[f09-workspace-timeout]]`

## See also

ROADMAP.md F09, `tests/integration`, `crates/draconic-backend-llvm`, docs/adr/0002-shared-ir-dual-backends.md, CONTEXT.md, [[ffi]], [[s-f09]], [[ticket-145-f09-workspace-timeout]].
