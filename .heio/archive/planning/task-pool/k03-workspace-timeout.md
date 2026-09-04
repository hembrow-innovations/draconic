---
id: "k03-workspace-timeout"
title: "K03 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T18:28:05Z"
updated_at: "2026-09-04T18:38:00Z"
---

# K03 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K03 work; the `draconic-pkg` cache tests for module cache layout keyed by module path + commit OID, git clone/fetch into the VCS store, checkout of a pinned OID, and SHA-256 over the canonical package tree stay green.

## Context

Roadmap ID **K03** (Module cache: layout, git clone/fetch, checkout by OID). Review of [[s-k03]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`draconic-pkg` cache) stayed green. If the hang comes from the K03 change, fix that module cache surface so both the workspace check and the `draconic-pkg` cache tests for module cache layout keyed by module path + commit OID, git clone/fetch into the VCS store, checkout of a pinned OID, and SHA-256 over the canonical package tree hold. Mark K03 `done` only when those tests are green. Not K03.01 cache layout keyed by module path + commit OID, K03.02 git clone/fetch into cache, K03.03 checkout pinned OID / cache hit skips network, K03.04 content hash SHA-256 over canonical package tree, K01 manifest (`draconic.toml`), K02 lockfile (`draconic.lock`) parse/write surface, K04 version resolve, or K08 integrity verify. Do not re-open [[s-k03]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline cache` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-pkg cache` still prints `test result: ok.` K03 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K03), `crates/draconic-pkg`, `crates/draconic-pkg/src/cache.rs`, `crates/draconic-pkg/src/hash.rs`, module cache layout / git clone-fetch / OID checkout surface as needed to unhang workspace tests after K03

## Links

[[s-k03-workspace-timeout]] [[ticket-167-k03-workspace-timeout]] [[s-k03]]
