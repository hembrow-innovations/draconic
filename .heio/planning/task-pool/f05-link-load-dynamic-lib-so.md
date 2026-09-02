---
id: "f05-link-load-dynamic-lib-so"
title: "F05 Link/load dynamic lib (`.so`/`.dylib`/`.dll`); call one symbol"
kind: task
status: ready
tags: []
created_at: "2026-09-02T22:10:45Z"
updated_at: "2026-09-02T22:10:45Z"
---

# F05 Link/load dynamic lib (`.so`/`.dylib`/`.dll`); call one symbol

## Done

ROADMAP F05 is implemented test-first on native: a Program loads an external dynamic library (`.so`/`.dylib`/`.dll`), resolves one C symbol, calling that symbol observes the C return value end-to-end, and a missing lib is a typed error; `ffi/link_dynamic` conformance and integration tests are green and F05 is `done`.

## Context

Roadmap ID **F05** (Link/load dynamic lib (`.so`/`.dylib`/`.dll`); call one symbol). F05.01–F05.02 already land load/resolve-one-symbol and call-with-typed-missing-lib error; this sitting unifies them as one honest link-dynamic / call-one-symbol surface on native. Tests under `tests/conformance` fixtures `ffi/link_dynamic` and `tests/integration`. Harnesses `tests/conformance/tests/ffi_link_dynamic.rs` and `tests/integration/tests/ffi_link_dynamic.rs`. Mark F05 `done` only when those tests are green. Not F05.01, F05.02, F04, F02, F03, F07, F08, F09, or js-target FFI (native-only until an explicit bridge row).

## Verify

`cargo test -p draconic-conformance --test ffi_link_dynamic` prints `test result: ok.` `cargo test -p draconic-integration-tests --test ffi_link_dynamic` prints `test result: ok.` Workspace `cargo test --workspace` stays green. F05 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (F05), `tests/conformance/fixtures/ffi/link_dynamic`, `tests/conformance/tests/ffi_link_dynamic.rs`, `tests/integration/tests/ffi_link_dynamic.rs`, `crates/draconic-backend-llvm`, `crates/draconic-cli`, native dynamic-link / FFI paths as needed for the parent surface

## Links

[[s-f05]] [[ticket-67-f05-link-load-dynamic-lib-so]]
