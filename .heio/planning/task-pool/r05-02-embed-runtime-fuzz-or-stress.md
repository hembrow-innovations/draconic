---
id: "r05-02-embed-runtime-fuzz-or-stress"
title: "R05.02 Embed/runtime fuzz or stress hooks"
kind: task
status: ready
tags: []
created_at: "2026-09-02T14:01:00Z"
updated_at: "2026-09-02T14:01:00Z"
---

# R05.02 Embed/runtime fuzz or stress hooks

## Done

ROADMAP R05.02 is implemented test-first on the native target: Embed and Runtime expose a fuzz or stress entry (cargo-fuzz or designed harness) that treats Ok and Err as success and panics/aborts as failure; tests under `crates/draconic-runtime` fuzz and `crates/draconic-embed` lock that empty, valid, invalid, and binary garbage input does not panic; R05.02 is `done`.

## Context

Roadmap ID **R05.02** (Embed/runtime fuzz or stress hooks). R05.01 already lands the parser fuzz entry in `crates/draconic-parser`; this sitting mirrors that designed surface on compiler-in-runtime and native Runtime so garbage at embed `eval` / Runtime entry does not panic. Tests under `crates/draconic-runtime` (`--lib fuzz`) and `crates/draconic-embed` (`--lib fuzz`) lock empty, valid, invalid, and binary garbage input. Mark R05.02 `done` only when those tests are green. Not R05 parent remainder, R05.01 parser fuzz (already `done`), R01 embed/eval resource limits, R04 panic/abort vs catchable exception, R06 panic backtraces, N07 embed `eval` / `Function` surface, or N09 GC stress.

## Verify

`cargo test -p draconic-runtime --lib fuzz` prints `test result: ok.` `cargo test -p draconic-embed --lib fuzz` prints `test result: ok.` Workspace `cargo test --workspace` stays green. R05.02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (R05.02), `crates/draconic-runtime`, `crates/draconic-embed`

## Links

[[s-r05-02]] [[ticket-115-r05-02-embed-runtime-fuzz-or-stress]]
