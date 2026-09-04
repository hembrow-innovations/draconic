---
id: "f09-workspace-timeout"
title: "F09 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T15:45:03Z"
updated_at: "2026-09-04T16:24:50Z"
---

# F09 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP F09 work; the LLVM backend wasm32/wasi emit tests and the integration emit+link smoke stay green.

## Context

Roadmap ID **F09** (Optional later: wasm32/wasi emit + link smoke). Review of [[s-f09]] left O3 unmet: `cargo test --workspace` timed out at 120s while O1 (`wasm32_wasi` LLVM backend) and O2 (`wasm32_wasi` integration) stayed green. If the hang comes from the F09 change, fix that wasm32/wasi emit + link smoke surface so both the workspace check and the emit+link harnesses hold. Mark F09 `done` only when those tests are green. Not F01 `extern "C"` call out, F04 Link external static lib, F05 Load dynamic lib, F06 Manual `extern` decls, F07 Bindgen from C headers, F08 Unsafe/native-only FFI diagnostics, or D04 linux/darwin/windows × amd64/arm64 cross-compile matrix. No third WASM-only IR (ADR-0002), full WASI libc / preview2 host, browser wasm, or wasmtime identity. Do not re-open [[s-f09]]. Do not change the F v1 done bar to require F09.

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-backend-llvm wasm32_wasi --offline && cargo test -p draconic-integration-tests --test wasm32_wasi --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-backend-llvm wasm32_wasi` and `cargo test -p draconic-integration-tests --test wasm32_wasi` still print `test result: ok.` F09 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (F09), `tests/integration`, `crates/draconic-backend-llvm`, wasm32/wasi emit + link smoke surface as needed to unhang workspace tests after F09

## Links

[[s-f09-workspace-timeout]] [[ticket-145-f09-workspace-timeout]] [[s-f09]]
