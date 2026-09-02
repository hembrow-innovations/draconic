---
id: "d03-reproducible-builds"
title: "D03 reproducible builds surface"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:12:00Z"
updated_at: "2026-09-03T02:40:00Z"
---

# D03 reproducible builds surface

## Done

ROADMAP D03 is implemented test-first on the compiler: same source plus the same toolchain pin yields artifacts that match the documented equivalence contract (byte-identical where promised); `reproducible_builds` integration tests are green and D03 is `done`.

## Context

Roadmap ID **D03** (Reproducible builds: same source + pin → documented-equivalent artifacts). D03.01 documents timestamp and path expectations and D03.02 is the emit-identity cut; this sitting unifies them as one honest same-source-plus-pin surface on the compiler. Tests under `tests/integration` (`reproducible_builds`). Mark D03 `done` only when those tests are green. Not D01, D02, D04, D05, or the D03.01 / D03.02 child rows.

## Verify

`cargo test -p draconic-integration-tests --test reproducible_builds` prints `test result: ok.` Workspace `cargo test --workspace` stays green. D03 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D03), `tests/integration/tests/reproducible_builds.rs`, compiler emit / pin paths as needed for the parent surface

## Links

[[s-d03]] [[ticket-95-d03-reproducible-builds-same-source-pin]]
