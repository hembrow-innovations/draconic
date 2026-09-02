---
id: "k03-module-cache-layout-git-clone"
title: "K03 Module cache: layout, git clone/fetch, checkout by OID"
kind: task
status: ready
tags: []
created_at: "2026-09-02T22:24:55Z"
updated_at: "2026-09-02T22:24:55Z"
---

# K03 Module cache: layout, git clone/fetch, checkout by OID

## Done

ROADMAP K03 is implemented test-first on the compiler target: a Program's package graph stores git-backed modules in cache (layout keyed by module path + commit OID, clone/fetch into the VCS store, checkout of a pinned OID with cache hits skipping the network, SHA-256 over the canonical package tree); `draconic-pkg` cache tests are green and K03 is `done`.

## Context

Roadmap ID **K03** (Module cache: layout, git clone/fetch, checkout by OID). K03.01–K03.04 already land layout, clone/fetch, OID checkout, and tree SHA-256; this sitting unifies them as one honest module cache surface on the compiler target. Tests in `crates/draconic-pkg`. Harness `cargo test -p draconic-pkg cache`. Mark K03 `done` only when those tests are green. Not K03.01–K03.04 as separate atoms, K01, K02, K04, or K08.

## Verify

`cargo test -p draconic-pkg cache` prints `test result: ok.` Workspace `cargo test --workspace` stays green. K03 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K03), `crates/draconic-pkg`, `crates/draconic-pkg/src/cache.rs`, `crates/draconic-pkg/src/hash.rs`

## Links

[[s-k03]] [[ticket-52-k03-module-cache-layout-git-clone]]
