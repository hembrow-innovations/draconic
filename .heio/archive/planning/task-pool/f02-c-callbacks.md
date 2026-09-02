---
id: "f02-c-callbacks"
title: "F02 C callbacks surface"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:18:30Z"
updated_at: "2026-09-02T20:09:50Z"
---

# F02 C callbacks surface

## Done

ROADMAP F02 is implemented test-first on native: a Program can pass a Draconic fn as an `extern "C"` pointer and the host can invoke it with scalar args so the return value is observed; `ffi/callback` fixtures are green and F02 is `done`.

## Context

Roadmap ID **F02** (C callbacks: Draconic fn as `extern "C"` pointer; host invokes). F02.01–F02.02 already land export of a Draconic fn as a C function pointer and host invoke with scalar args so the return is observed; this sitting unifies them as one honest native callback surface. Tests under `tests/conformance` fixtures `ffi/callback`. Harness `tests/conformance/tests/ffi_callback.rs`. Mark F02 `done` only when those tests are green. Not F01, F03–F05, F08, js FFI, or a silent JS callback polyfill (native-only; other backend hard-errors).

## Verify

`cargo test -p draconic-conformance --test ffi_callback` prints `test result: ok.` Workspace `cargo test --workspace` stays green. F02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (F02), `tests/conformance/fixtures/ffi/callback`, `tests/conformance/tests/ffi_callback.rs`, `crates/draconic-backend-llvm`, native callback / extern-C pointer paths as needed for the parent surface

## Links

[[s-f02]] [[ticket-64-f02-c-callbacks-draconic-fn-as]]
