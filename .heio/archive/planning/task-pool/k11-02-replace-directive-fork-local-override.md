---
id: "k11-02-replace-directive-fork-local-override"
title: "K11.02 `replace` directive: fork/local override"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:39:00Z"
updated_at: "2026-09-03T05:16:34Z"
---

# K11.02 `replace` directive: fork/local override

## Done

ROADMAP K11.02 is implemented test-first on the compiler target: `draconic.toml` accepts a `replace` directive mapping a module path to a fork git source or a local path; package resolve/fetch honors that override and does not silently keep the original pin; `draconic-pkg` replace tests are green and K11.02 is `done`.

## Context

Roadmap ID **K11.02** (`replace` directive: fork/local override). Post-v1 packaging on the compiler target (ADR-0009): a Program overrides a module path to a fork (different git URL / module path) or a local directory so resolve and fetch use the replacement instead of the declared dependency identity. Tests in `crates/draconic-pkg` lock parse and apply. Mark K11.02 `done` only when those tests are green. Not K11 umbrella, K11.01 private git auth, K11.03 monorepo subdir paths, K11.04 proxy/mirror, K11.05 yank, or npm-style `resolutions` / crates.io `[patch]` as the primary shape.

## Verify

`cargo test -p draconic-pkg replace` prints `test result: ok.` Workspace `cargo test --workspace` stays green. K11.02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K11.02), `crates/draconic-pkg`, `crates/draconic-pkg/src/lib.rs`

## Links

[[s-k11-02]] [[ticket-60-k11-02-replace-directive-fork-local-override]]
