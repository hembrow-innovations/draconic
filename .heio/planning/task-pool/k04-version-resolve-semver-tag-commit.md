---
id: "k04-version-resolve-semver-tag-commit"
title: "K04 Version resolve: semver tag → commit OID; fail closed"
kind: task
status: ready
tags: []
created_at: "2026-09-02T22:28:28Z"
updated_at: "2026-09-02T22:28:28Z"
---

# K04 Version resolve: semver tag → commit OID; fail closed

## Done

ROADMAP K04 is implemented test-first on the compiler target: a Program's package graph resolves a version req against git tags to the highest matching semver commit OID, no match / non-semver-only / empty tags fail closed with a diagnostic, and a direct-deps set resolves to lock pins (v1: direct only); `draconic-pkg` resolve tests are green and K04 is `done`.

## Context

Roadmap ID **K04** (Version resolve: semver tag → commit OID; fail closed). K04.01–K04.03 already land highest matching semver, fail-closed diagnostics, and direct-deps → lock pins (v1: direct only); this sitting unifies them as one honest semver-tag → OID resolve surface on the compiler target. Tests in `crates/draconic-pkg`. Harness `cargo test -p draconic-pkg resolve`. Mark K04 `done` only when those tests are green. Not K04.01–K04.03 as separate atoms, K03, K05, K02, or K08.

## Verify

`cargo test -p draconic-pkg resolve` prints `test result: ok.` Workspace `cargo test --workspace` stays green. K04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K04), `crates/draconic-pkg`, `crates/draconic-pkg/src/resolve.rs`

## Links

[[s-k04]] [[ticket-53-k04-version-resolve-semver-tag-commit]]
