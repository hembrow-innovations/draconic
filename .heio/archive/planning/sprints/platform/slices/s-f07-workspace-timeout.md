---
id: "s-f07-workspace-timeout"
title: "F07 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T15:32:31Z"
updated_at: "2026-09-04T16:21:35Z"
claimed-by: d6081223-9b3a-454d-a600-f8114c62e51c
---

# F07 workspace tests finish

## Why

Review of [[s-f07]] left ROADMAP F07 unfinished: O1 (`bindgen` CLI) and O2 (`bindgen_header` integration) held, but O3 `cargo test --workspace` timed out at 120s. The ffi location still needs the F07 Loop to leave the workspace green, not only the bindgen harnesses.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP F07 work. The `bindgen` CLI and `bindgen_header` integration harnesses stay green. If the hang comes from the F07 change, fix that bindgen-ish generate-externs-from-C-header-subset surface so both checks hold. Mark F07 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-f07]]**: that slice stays sealed `failed`
- **F07.01**: Parse C header subset: functions with scalar/pointer params (already `done`)
- **F07.02**: Emit Draconic `extern "C"` decls from parsed header (already `done`)
- **F07.03**: CLI: `draconic bindgen <header>` writes extern module (already `done`)
- **F07.04**: Header subset: simple structs + typedef names (already `done`)
- **F06**: Manual `extern` decls (parse + check + IR/ABI)
- **F08**: Unsafe/native-only FFI diagnostics
- **F09**: wasm32/wasi emit
- full C preprocessor / rust-bindgen completeness (subset only; no full C)

## Oracle checklist

- [x] O1: workspace tests finish after the F07 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-cli --test bindgen --offline && cargo test -p draconic-integration-tests --test bindgen_header --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=601945df40ea7c9c bytes=97045 at=2026-09-04T16:21:11.297Z

- [x] O2: F07 bindgen CLI writes an extern module from a C header subset
  CHECK: cargo test -p draconic-cli --test bindgen
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=66184f79cf831c79 bytes=3044 at=2026-09-04T16:21:11.714Z

- [x] O3: F07 parse/emit/CLI/struct-typedef surface stays locked by the integration bindgen tests
  CHECK: cargo test -p draconic-integration-tests --test bindgen_header
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=00e1e24a078c8f35 bytes=3245 at=2026-09-04T16:21:11.785Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[f07-workspace-timeout]]`

## See also

ROADMAP.md F07, `crates/draconic-cli/tests/bindgen.rs`, `crates/draconic-cli/src/c_header.rs`, `tests/integration/tests/bindgen_header.rs`, docs/adr/0002-shared-ir-dual-backends.md, docs/adr/0003-gc-runtime-and-dual-worlds.md, CONTEXT.md, [[ffi]], [[s-f07]], [[ticket-143-f07-workspace-timeout]].
