---
id: "f07-workspace-timeout"
title: "F07 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T15:33:58Z"
updated_at: "2026-09-04T16:15:17Z"
---

# F07 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP F07 work; the `bindgen` CLI and `bindgen_header` integration harnesses stay green.

## Context

Roadmap ID **F07** (Bindgen-ish: generate externs from C header subset). Review of [[s-f07]] left O3 unmet: `cargo test --workspace` timed out at 120s while O1 (`bindgen` CLI) and O2 (`bindgen_header` integration) stayed green. If the hang comes from the F07 change, fix that bindgen-ish generate-externs-from-C-header-subset surface so both the workspace check and the bindgen harnesses hold. Mark F07 `done` only when those tests are green. Not F07.01 Parse C header subset: functions with scalar/pointer params, F07.02 Emit Draconic `extern "C"` decls from parsed header, F07.03 CLI: `draconic bindgen <header>` writes extern module, F07.04 Header subset: simple structs + typedef names, F06 Manual `extern` decls, F08 Unsafe/native-only FFI diagnostics, or F09 wasm32/wasi emit. No full C preprocessor / rust-bindgen completeness (subset only). Do not re-open [[s-f07]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-cli --test bindgen --offline && cargo test -p draconic-integration-tests --test bindgen_header --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-cli --test bindgen` and `cargo test -p draconic-integration-tests --test bindgen_header` still print `test result: ok.` F07 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (F07), `crates/draconic-cli/tests/bindgen.rs`, `crates/draconic-cli/src/c_header.rs`, `tests/integration/tests/bindgen_header.rs`, bindgen-ish generate-externs-from-C-header-subset surface as needed to unhang workspace tests after F07

## Links

[[s-f07-workspace-timeout]] [[ticket-143-f07-workspace-timeout]] [[s-f07]]
