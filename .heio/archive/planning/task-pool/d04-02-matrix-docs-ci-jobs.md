---
id: "d04-02-matrix-docs-ci-jobs"
title: "D04.02 matrix docs + CI jobs for available OS/arch pairs"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:13:00Z"
updated_at: "2026-09-02T13:13:00Z"
---

# D04.02 matrix docs + CI jobs for available OS/arch pairs

## Done

ROADMAP D04.02 is implemented test-first on the native target: docs list the available OS/arch pairs, CI jobs cover those pairs, `cross_compile_matrix` integration tests lock both contracts, and D04.02 is `done`.

## Context

Roadmap ID **D04.02** (Matrix docs + CI jobs for available OS/arch pairs). D01.01 already ships a host-triple artifact; this sitting makes the matrix honest for available linux/darwin/windows × amd64/arm64 pairs. Docs name those pairs. CI jobs exist for them. Integration tests under `tests/integration` lock both the docs contract and the workflow jobs. Harness `cargo test -p draconic-integration-tests --test cross_compile_matrix`. Mark D04.02 `done` only when those tests are green. Not D04.01 non-host LLVM smoke, D04 parent remainder for unavailable pairs, D05 strip/LTO, or a re-litigation of D01.01 host-only staging.

## Verify

`cargo test -p draconic-integration-tests --test cross_compile_matrix` prints `test result: ok.` Workspace `cargo test --workspace` stays green. D04.02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D04.02), `tests/integration`, `.github/workflows`, `website/install.md`

## Links

[[s-d04-02]] [[ticket-100-d04-02-matrix-docs-ci-jobs-for]]
