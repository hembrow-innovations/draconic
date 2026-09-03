---
id: "k11-05-yank-retract-when-advisory-source"
title: "K11.05 Yank/retract when advisory source configured"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:41:00Z"
updated_at: "2026-09-03T05:16:34Z"
---

# K11.05 Yank/retract when advisory source configured

## Done

ROADMAP K11.05 is implemented test-first on the compiler target: with an advisory source configured, package resolve/fetch hard-fails a yanked or retracted version and does not pin it; with no advisory source configured, yank is not invented as a silent v1 check; `draconic-pkg` yank tests are green and K11.05 is `done`.

## Context

Roadmap ID **K11.05** (Yank/retract when advisory source configured). Post-v1 packaging on the compiler target (ADR-0009): when an advisory source is configured, a yanked or retracted version is refused at resolve/fetch instead of silently installing. Tests in `crates/draconic-pkg` lock that refuse path. Mark K11.05 `done` only when those tests are green. Not K11 umbrella, K11.01 private git auth, K11.02 replace, K11.03 monorepo subdir paths, K11.04 proxy/mirror, K08 lock-hash integrity, or npm unpublish / crates.io yank as the primary shape.

## Verify

`cargo test -p draconic-pkg yank` prints `test result: ok.` Workspace `cargo test --workspace` stays green. K11.05 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K11.05), `crates/draconic-pkg`

## Links

[[s-k11-05]] [[ticket-63-k11-05-yank-retract-when-advisory-source]]
