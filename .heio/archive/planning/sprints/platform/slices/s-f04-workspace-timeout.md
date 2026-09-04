---
id: "s-f04-workspace-timeout"
title: "F04 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T15:23:20Z"
updated_at: "2026-09-04T16:12:38Z"
claimed-by: 715cff43-8d25-43a6-b7f9-17c6afa028bf
---

# F04 workspace tests finish

## Why

Review of [[s-f04]] left ROADMAP F04 unfinished: O1 (`ffi_link_static` conformance) and O2 (`ffi_link_static` integration) held, but O3 `cargo test --workspace` timed out at 120s. The ffi location still needs the F04 Loop to leave the workspace green, not only the link-static harnesses.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP F04 work. The `ffi_link_static` conformance and integration harnesses stay green. If the hang comes from the F04 change, fix that link-external-static-lib (`.a`) / call-one-symbol surface so both checks hold. Mark F04 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-f04]]**: that slice stays sealed `failed`
- **F04.01**: Build links `.a`; resolve one C symbol (already `done`)
- **F04.02**: Call linked static symbol end-to-end (already `done`)
- **F05**: Link/load dynamic lib (`.so`/`.dylib`/`.dll`)
- **F02**: C callbacks (Draconic fn as `extern "C"` pointer)
- **F03**: C-compatible struct layout
- **F07**: Bindgen from C headers
- **F08**: Unsafe/native-only FFI diagnostics (js hard-error)
- js-target FFI (native-only until an explicit bridge row)

## Oracle checklist

- [x] O1: workspace tests finish after the F04 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test ffi_link_static --offline && cargo test -p draconic-integration-tests --test ffi_link_static --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=ce9c583a9a5f8fee bytes=96786 at=2026-09-04T16:12:19.069Z

- [x] O2: F04 link-static resolve and call stay green on the declared native target through the ffi/link_static conformance fixtures
  CHECK: cargo test -p draconic-conformance --test ffi_link_static
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=353ae9d46fcb6df2 bytes=3047 at=2026-09-04T16:12:19.806Z

- [x] O3: F04 native build links `.a` and calls one symbol in the integration tests
  CHECK: cargo test -p draconic-integration-tests --test ffi_link_static
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=d9e1253aec8f4c09 bytes=2983 at=2026-09-04T16:12:20.417Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[f04-workspace-timeout]]`

## See also

ROADMAP.md F04, `tests/conformance/tests/ffi_link_static.rs`, `tests/conformance/fixtures/ffi/link_static`, `tests/integration/tests/ffi_link_static.rs`, `crates/draconic-backend-llvm`, `crates/draconic-cli`, docs/adr/0002-shared-ir-dual-backends.md, docs/adr/0003-gc-runtime-and-dual-worlds.md, CONTEXT.md, [[ffi]], [[s-f04]], [[ticket-141-f04-workspace-timeout]].
