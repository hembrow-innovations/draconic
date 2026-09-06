---
id: "task-420-d03-02-same-source-pin-byte-identical"
title: "D03.02 Same source + pin → byte-identical or documented-equivalent emit"
kind: task
status: completed
mode: afk
blocked_by: []
tags: []
created_at: "2026-09-02T13:36:43Z"
updated_at: "2026-09-06T18:00:00Z"
sprint: "platform"
---

# D03.02 Same source + pin → byte-identical or documented-equivalent emit

## Done

ROADMAP D03.02 is implemented test-first on the compiler target: building the same source twice with the same toolchain pin produces emit that is byte-identical, or matches the documented equivalent where byte identity is not promised; `reproducible_emit` integration tests are green and D03.02 is `done`.

## Context

Roadmap ID **D03.02** (Same source + pin → byte-identical or documented-equivalent emit). D03.01 names timestamp and path expectations; this sitting is the compiler emit so two builds can be compared, not only described. Tests under `tests/integration` (`reproducible_emit`) lock that two-build comparison. Mark D03.02 `done` only when those tests are green. Not D03.01 docs, D03 parent remainder, D02, D01, D04, or D05.

## Verify

`cargo test -p draconic-integration-tests --test reproducible_emit` prints `test result: ok.` Workspace `cargo test --workspace` stays green. D03.02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D03.02), `tests/integration`

## Links

[[slice-246-d03-02]] [[ticket-97-d03-02-same-source-pin-byte-identical]]
