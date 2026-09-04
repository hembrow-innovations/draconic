---
id: "s-f05-workspace-timeout"
title: "F05 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T15:26:57Z"
updated_at: "2026-09-04T16:14:55Z"
claimed-by: 631432a2-2322-4dbd-b43c-b34eb384ab5a
---

# F05 workspace tests finish

## Why

Review of [[s-f05]] left ROADMAP F05 unfinished: O1 (`ffi_link_dynamic` conformance) and O2 (`ffi_link_dynamic` integration) held, but O3 `cargo test --workspace` timed out at 120s. The ffi location still needs the F05 Loop to leave the workspace green, not only the link-dynamic harnesses.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP F05 work. The `ffi_link_dynamic` conformance and integration harnesses stay green. If the hang comes from the F05 change, fix that link/load-dynamic-lib (`.so`/`.dylib`/`.dll`) / call-one-symbol surface so both checks hold. Mark F05 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-f05]]**: that slice stays sealed `failed`
- **F05.01**: Load dynamic lib at link or runtime; resolve one symbol (already `done`)
- **F05.02**: Call dynamic symbol; missing lib → typed error (already `done`)
- **F04**: Link external static lib (`.a`)
- **F02**: C callbacks (Draconic fn as `extern "C"` pointer)
- **F03**: C-compatible struct layout
- **F07**: Bindgen from C headers
- **F08**: Unsafe/native-only FFI diagnostics (js hard-error)
- js-target FFI (native-only until an explicit bridge row)

## Oracle checklist

- [x] O1: workspace tests finish after the F05 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test ffi_link_dynamic --offline && cargo test -p draconic-integration-tests --test ffi_link_dynamic --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=abeec5b187b64ef7 bytes=96855 at=2026-09-04T16:14:18.254Z

- [x] O2: F05 link-dynamic resolve and call stay green on the declared native target through the ffi/link_dynamic conformance fixtures
  CHECK: cargo test -p draconic-conformance --test ffi_link_dynamic
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=54cfdf2b89e0cfbf bytes=3053 at=2026-09-04T16:14:22.367Z

- [x] O3: F05 native build loads a shared lib and calls one symbol in the integration tests
  CHECK: cargo test -p draconic-integration-tests --test ffi_link_dynamic
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=5b3b6631c0fa06fd bytes=3046 at=2026-09-04T16:14:25.518Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[f05-workspace-timeout]]`

## See also

ROADMAP.md F05, `tests/conformance/tests/ffi_link_dynamic.rs`, `tests/conformance/fixtures/ffi/link_dynamic`, `tests/integration/tests/ffi_link_dynamic.rs`, `crates/draconic-backend-llvm`, `crates/draconic-cli`, docs/adr/0002-shared-ir-dual-backends.md, docs/adr/0003-gc-runtime-and-dual-worlds.md, CONTEXT.md, [[ffi]], [[s-f05]], [[ticket-142-f05-workspace-timeout]].
