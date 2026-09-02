---
id: "k01-manifest-draconic-toml-module-path"
title: "K01 Manifest (`draconic.toml`): module path, deps, optional path→git URL map"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:33:45Z"
updated_at: "2026-09-02T13:33:45Z"
---

# K01 Manifest (`draconic.toml`): module path, deps, optional path→git URL map

## Done

ROADMAP K01 is implemented test-first on the compiler: a Program's `draconic.toml` parses own module path plus deps, round-trips with stable order, rejects invalid schema with diagnostics, and resolves git URLs from the optional map or default derivation; `draconic-pkg` lib tests are green and K01 is `done`.

## Context

Roadmap ID **K01** (Manifest (`draconic.toml`): module path, deps, optional path→git URL map). K01.01–K01.04 already land parse, stable write/round-trip, schema diagnostics, and default `https://{module_path}.git`; this sitting unifies them as one honest manifest surface on the compiler. Tests in `crates/draconic-pkg`. Mark K01 `done` only when those tests are green. Not K01.01–K01.04 as separate atoms, K02, K03, K05, or D02.

## Verify

`cargo test -p draconic-pkg --lib` prints `test result: ok.` Workspace `cargo test --workspace` stays green. K01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K01), `crates/draconic-pkg`, `crates/draconic-pkg/src/lib.rs`

## Links

[[s-k01]] [[ticket-50-k01-manifest-draconic-toml-module-path]]
