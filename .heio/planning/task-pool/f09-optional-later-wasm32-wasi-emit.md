---
id: "f09-optional-later-wasm32-wasi-emit"
title: "F09 Optional later: wasm32/wasi emit + link smoke"
kind: task
status: ready
tags: []
created_at: "2026-09-02T22:12:55Z"
updated_at: "2026-09-02T22:12:55Z"
---

# F09 Optional later: wasm32/wasi emit + link smoke

## Done

ROADMAP F09 is implemented test-first on native: the LLVM backend emits for wasm32/wasi from the shared IR, a link smoke produces a linked wasm artifact (not a full WASI runtime), tests under `tests/integration` and `crates/draconic-backend-llvm` lock that emit+link surface, and F09 is `done`.

## Context

Roadmap ID **F09** (Optional later: wasm32/wasi emit + link smoke). F09 is later than the F v1 bar; this sitting is one atomic Loop so a Program can emit for wasm32/wasi from the shared IR (ADR-0002: no WASM-only fork) and link that artifact without leaving Draconic. Tests under `tests/integration` and `crates/draconic-backend-llvm`. Harnesses `cargo test -p draconic-backend-llvm wasm32_wasi` and `cargo test -p draconic-integration-tests --test wasm32_wasi`. Mark F09 `done` only when those tests are green. Not F01, F04, F05, F06, F08, D04, a third WASM-only IR, full WASI libc / preview2 host, browser wasm, wasmtime identity, or changing the F v1 done bar to require F09.

## Verify

`cargo test -p draconic-backend-llvm wasm32_wasi` prints `test result: ok.` `cargo test -p draconic-integration-tests --test wasm32_wasi` prints `test result: ok.` Workspace `cargo test --workspace` stays green. F09 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (F09), `tests/integration`, `crates/draconic-backend-llvm`

## Links

[[s-f09]] [[ticket-70-f09-optional-later-wasm32-wasi-emit]]
