---
id: "r05-fuzz-stress-hooks-parser-embed"
title: "R05 Fuzz/stress hooks: parser/embed/runtime entry points"
kind: task
status: completed
tags: []
created_at: "2026-09-02T14:02:32Z"
updated_at: "2026-09-05T16:39:36Z"
---

# R05 Fuzz/stress hooks: parser/embed/runtime entry points

## Done

ROADMAP R05 is implemented test-first on both targets: parser fuzz under `crates/draconic-parser` locks that garbage source does not panic at the parse entry; R05 is `done`. Embed/runtime fuzz stays the R05.02 child.

## Context

Roadmap ID **R05** (Fuzz/stress hooks: parser/embed/runtime entry points). R05.01 already lands the parser harness; this sitting unifies that designed parse-entry surface so garbage source does not panic, not folklore. Tests under `crates/draconic-parser` (`--lib fuzz`) lock the parent row. Mark R05 `done` only when those tests are green. Embed/runtime fuzz stays **R05.02**. Not R05.01 (already `done`), R04 panic/abort vs catchable exception, R06 panic backtraces, or N09 GC stress.

## Verify

`cargo test -p draconic-parser --lib fuzz` prints `test result: ok.` Workspace `cargo test --workspace` stays green. R05 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (R05), `crates/draconic-parser`

## Links

[[s-r05]] [[ticket-114-r05-fuzz-stress-hooks-parser-embed]]

## Gauntlet

- **round 1**: `cargo test -p draconic-parser --lib fuzz` — win. `test result: ok.` 5 passed. `cargo test --workspace` every crate `test result: ok.` ROADMAP R05 `done`.
