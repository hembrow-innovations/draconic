---
id: "s02-expand-test262-allowlist-promote-first"
title: "S02 Expand Test262 allowlist / promote first failure cluster (see **E19.02**)"
kind: task
status: ready
tags: []
created_at: "2026-09-02T14:03:49Z"
updated_at: "2026-09-02T14:03:49Z"
---

# S02 Expand Test262 allowlist / promote first failure cluster (see **E19.02**)

## Done

ROADMAP S02 is implemented test-first on the js target: the Test262 curated allowlist expands after baseline triage and the first failure cluster is promoted (see E19.02); tests under `tests/test262` (allowlist + baseline-report) lock that expansion and S02 is `done`.

## Context

Roadmap ID **S02** (Expand Test262 allowlist / promote first failure cluster (see **E19.02**)). S-track pointer to E19.02: S01/E19.01 already land the staged harness + curated allowlist; E19.02 already promoted the first failure cluster after baseline triage (language/types + early gaps). This sitting makes that expansion honest on the S row — no duplicate harness work. Tests under `tests/test262` (allowlist + baseline-report). Harness crate `draconic-test262`. Mark S02 `done` only when those tests are green. Not S01 / E19.01, E19.03+, E17.02 / E18.44, or native Test262 in v1.

## Verify

`cargo test -p draconic-test262 allowlist_loads_and_has_entries` prints `allowlist_loads_and_has_entries`. Workspace `cargo test --workspace` stays green. S02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (S02), `tests/test262`, `tests/test262/allowlist.txt`, `tests/test262/baseline-report.md`

## Links

[[s-s02]] [[ticket-119-s02-expand-test262-allowlist-promote-first]]
