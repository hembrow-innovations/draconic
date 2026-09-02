---
id: "f04-link-external-static-lib"
title: "F04 link external static lib surface"
kind: task
status: completed
tags: []
created_at: "2026-09-03T06:28:00Z"
updated_at: "2026-09-02T20:40:00Z"
---

# F04 link external static lib surface

## Done

ROADMAP F04 is implemented test-first on native: a Program links an external static archive (`.a`), resolves one C symbol, and calling that symbol observes the C return value; `ffi/link_static` fixtures are green and F04 is `done`.

## Context

Roadmap ID **F04** (Link external static lib (`.a`); call one symbol). F04.01–F04.02 already land resolve-one-symbol and call-end-to-end; this sitting unifies them as one honest link-static / call-one-symbol surface. Tests under `tests/conformance` fixtures `ffi/link_static` and `tests/integration`. Harness `tests/conformance/tests/ffi_link_static.rs`. Mark F04 `done` only when those tests are green. Not F04.01, F04.02, F05, F02, F03, F07, F08, or js-target FFI.

## Verify

`cargo test -p draconic-conformance --test ffi_link_static` prints `test result: ok.` `cargo test -p draconic-integration-tests --test ffi_link_static` prints `test result: ok.` Workspace `cargo test --workspace` stays green. F04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (F04), `tests/conformance/fixtures/ffi/link_static`, `tests/conformance/tests/ffi_link_static.rs`, `tests/integration/tests/ffi_link_static.rs`, `crates/draconic-backend-llvm`, `crates/draconic-cli`

## Links

[[s-f04]] [[ticket-66-f04-link-external-static-lib-a]]
