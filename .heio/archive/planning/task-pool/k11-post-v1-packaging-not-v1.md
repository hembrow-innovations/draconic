---
id: "k11-post-v1-packaging-not-v1"
title: "K11 Post-v1 packaging (not v1 bar)"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T13:22:50Z"
updated_at: "2026-09-04T14:25:34Z"
---

# K11 Post-v1 packaging (not v1 bar)

## Blocked by

None.

## Done

ROADMAP K11 is implemented test-first on the compiler target: the v1 packaging surface does not silently ship private git auth, `replace` fork/local override, multi-module monorepo subdir paths, module proxy/mirror, or yank/retract as v1 features; those remain later children; `draconic-pkg` k11 tests are green and K11 is `done`.

## Context

Roadmap ID **K11** (`Post-v1 packaging (not v1 bar)`). ADR-0009 git-backed packages; K v1 done bar is K01–K08 + K09.02. K11.01–K11.05 already land private git auth, `replace`, monorepo subdir paths, proxy/mirror, and yank as later children; this sitting is the parent row that those ops are honestly later, not silent v1. Tests in `crates/draconic-pkg` lock that later-not-v1 classification. Mark K11 `done` only when those tests are green. Not K11.01–K11.05 as separate atoms, K01–K10, or an npm-compatible registry / crates.io clone as v1 primary.

## Verify

`cargo test -p draconic-pkg k11` prints `test result: ok.` Workspace `cargo test --workspace` stays green. K11 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K11), `crates/draconic-pkg`

## Links

[[s-k11]] [[ticket-58-k11-post-v1-packaging-not-v1]]
