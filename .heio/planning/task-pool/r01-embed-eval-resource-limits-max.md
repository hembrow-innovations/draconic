---
id: "r01-embed-eval-resource-limits-max"
title: "R01 Embed/eval resource limits: max source size, alloc/time budget"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T13:30:00Z"
updated_at: "2026-09-05T08:50:00Z"
---

# R01 Embed/eval resource limits: max source size, alloc/time budget

## Blocked by

None.

## Done

ROADMAP R01 is implemented test-first on native: embed `eval` / `Function` reject oversize source, fail closed when the alloc budget is exceeded, and interrupt/fail when the time budget is exceeded; tests under `crates/draconic-embed` and `crates/draconic-runtime` are green and R01 is `done`.

## Context

Roadmap ID **R01** (`Embed/eval resource limits: max source size, alloc/time budget`). Native target. R01.01–R01.03 already land max source size reject, alloc-budget fail-closed, and time-budget interrupt; this sitting unifies them so the combined Embed + Runtime limits surface is honest. Tests under `crates/draconic-embed` and `crates/draconic-runtime` lock that combined surface. Mark R01 `done` only when those tests are green. ADR-0011 classifies R01 exhaustion as fail-closed, not a JS exception. Not R01.01–R01.03 as separate atoms, R02 permission model, R04 panic/abort vs catchable exception policy, or N07 embed `eval` / `Function` surface itself.

## Verify

`cargo test -p draconic-embed --lib` prints `test result: ok.` `cargo test -p draconic-runtime --lib` prints `test result: ok.` Workspace `cargo test --workspace` stays green. R01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (R01), `crates/draconic-embed`, `crates/draconic-runtime`

## Links

[[s-r01]] [[ticket-104-r01-embed-eval-resource-limits-max]]
