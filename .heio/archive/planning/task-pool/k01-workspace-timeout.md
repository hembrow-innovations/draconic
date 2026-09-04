---
id: "k01-workspace-timeout"
title: "K01 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T18:14:06Z"
updated_at: "2026-09-04T18:21:36Z"
---

# K01 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K01 work; the `draconic-pkg` lib tests for `draconic.toml` module path, deps, and optional path→git URL map stay green.

## Context

Roadmap ID **K01** (Manifest (`draconic.toml`): module path, deps, optional path→git URL map). Review of [[s-k01]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`draconic-pkg` lib) stayed green. If the hang comes from the K01 change, fix that manifest surface so both the workspace check and the `draconic-pkg` lib tests for `draconic.toml` module path, deps, and optional path→git URL map hold. Mark K01 `done` only when those tests are green. Not K01.01 parse `draconic.toml` own module path + deps map, K01.02 write/round-trip `draconic.toml`, K01.03 manifest schema validation + diagnostics, K01.04 optional URL map / default `https://{module_path}.git`, K02 lockfile, K03 module cache, K05 CLI `draconic get` / `draconic mod tidy`, or D02 toolchain version pin. Do not re-open [[s-k01]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-pkg --lib` still prints `test result: ok.` K01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K01), `crates/draconic-pkg`, manifest parse/write/validate/url-map surface as needed to unhang workspace tests after K01

## Links

[[s-k01-workspace-timeout]] [[ticket-165-k01-workspace-timeout]] [[s-k01]]
