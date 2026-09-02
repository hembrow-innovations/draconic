---
id: "d03-01-document-reproducibility-expectations"
title: "D03.01 document reproducibility expectations (timestamps, paths)"
kind: task
status: completed
tags: []
created_at: "2026-09-02T17:19:05Z"
updated_at: "2026-09-02T17:25:00Z"
---

# D03.01 document reproducibility expectations (timestamps, paths)

## Done

ROADMAP D03.01 is implemented test-first on the compiler target: docs name timestamp and path reproducibility expectations, `reproducibility_expectations` integration tests lock that contract, and D03.01 is `done`.

## Context

Roadmap ID **D03.01** (Document reproducibility expectations (timestamps, paths)). Distribution honesty starts with words: docs name what same source plus pin promises for timestamps and embedded paths, so operators can tell whether two artifacts should match before D03.02 locks emit identity. Integration tests under `tests/integration` lock that those docs exist and cover timestamps and paths. Harness `cargo test -p draconic-integration-tests --test reproducibility_expectations`. Mark D03.01 `done` only when those tests are green. Not D03.02 emit identity, D03 parent remainder, D02 toolchain pin, or D01 release binaries.

## Verify

`cargo test -p draconic-integration-tests --test reproducibility_expectations` prints `test result: ok.` Workspace `cargo test --workspace` stays green. D03.01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D03.01), `tests/integration`, `website/install.md`

## Links

[[s-d03-01]] [[ticket-96-d03-01-document-reproducibility-expectations-timestamps-paths]]
