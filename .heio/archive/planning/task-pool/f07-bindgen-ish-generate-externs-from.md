---
id: "f07-bindgen-ish-generate-externs-from"
title: "F07 bindgen-ish generate externs from C header subset"
kind: task
status: completed
tags: []
created_at: "2026-09-02T20:56:18Z"
updated_at: "2026-09-02T21:10:00Z"
---

# F07 bindgen-ish generate externs from C header subset

## Done

ROADMAP F07 is implemented test-first on the compiler target: `draconic bindgen <header>` parses a C header subset (scalar/pointer functions, simple structs, typedef names), emits Draconic `extern "C"` decls, and writes an extern module; bindgen CLI and integration tests are green and F07 is `done`.

## Context

Roadmap ID **F07** (Bindgen-ish: generate externs from C header subset). F07.01–F07.04 already land parse, emit, CLI write, and simple struct/typedef names; this sitting unifies them as one honest `draconic bindgen` surface. Tests under `tests/integration` and `crates/draconic-cli`. Mark F07 `done` only when those tests are green. Not F07.01–F07.04 as separate rows, F06, F08, F09, or full C preprocessor / rust-bindgen completeness.

## Verify

`cargo test -p draconic-cli --test bindgen` prints `test result: ok.` `cargo test -p draconic-integration-tests --test bindgen_header` prints `test result: ok.` Workspace `cargo test --workspace` stays green. F07 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (F07), `crates/draconic-cli/tests/bindgen.rs`, `crates/draconic-cli/src/c_header.rs`, `tests/integration/tests/bindgen_header.rs`

## Links

[[s-f07]] [[ticket-68-f07-bindgen-ish-generate-externs-from]]
