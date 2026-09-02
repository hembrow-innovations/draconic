---
id: "f03-c-compatible-struct-layout"
title: "F03 C-compatible struct layout (repr(C)-style); read/write both sides"
kind: task
status: ready
tags: []
created_at: "2026-09-02T22:09:29Z"
updated_at: "2026-09-02T22:09:29Z"
---

# F03 C-compatible struct layout (repr(C)-style); read/write both sides

## Done

ROADMAP F03 is implemented test-first on native: a Program can lay out a native struct matching C ABI offsets, read and write fields from both the Draconic and C sides, and pass or return that struct by value or pointer across FFI; `ffi/layout` fixtures are green and F03 is `done`.

## Context

Roadmap ID **F03** (C-compatible struct layout (repr(C)-style); read/write both sides). F03.01–F03.02 already land field offsets matching the C ABI for scalars and pass/return by value or pointer; this sitting unifies them as one honest read/write-both-sides layout surface on native. Tests under `tests/conformance` fixtures `ffi/layout`. Harness `tests/conformance/tests/ffi_layout.rs`. Mark F03 `done` only when those tests are green. Not F01, F02, F04, F05, F07, F08, js FFI, or a silent JS struct polyfill (native-only; other backend hard-errors).

## Verify

`cargo test -p draconic-conformance --test ffi_layout` prints `test result: ok.` Workspace `cargo test --workspace` stays green. F03 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (F03), `tests/conformance/fixtures/ffi/layout`, `tests/conformance/tests/ffi_layout.rs`, `crates/draconic-backend-llvm`, native struct layout / FFI paths as needed for the parent surface

## Links

[[s-f03]] [[ticket-65-f03-c-compatible-struct-layout-repr]]
