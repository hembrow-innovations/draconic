---
id: "d04-workspace-tests"
title: "D04 workspace tests pass"
kind: task
status: ready
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-05T01:31:54Z"
updated_at: "2026-09-05T01:31:54Z"
---

# D04 workspace tests pass

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D04 work; D04, D04.02, and D01.01 readers stay green against `.yml.disabled`.

## Context

Roadmap ID **D04** (`Cross-compile matrix: linux/darwin/windows × amd64/arm64 (as available)`). Review of [[s-d04-workspace-disabled-gha]] left O4 unmet: `cargo test --workspace` exited 101 while O1 (`cross_compile`), O2 (`cross_compile_matrix`), and O3 (`release_artifact`) stayed green against `.github/workflows/release-artifact.yml.disabled` and `docs-pages.yml.disabled`. The distribution location still needs the D04 Loop to leave the workspace green, not only the disabled-GHA matrix and release-artifact readers. If the failure comes from the D04 harness change, fix that matrix/CI contract so the workspace check and those readers hold. Do not restore live `.yml` names. Do not reopen ROADMAP D04 / D04.02 / D01.01 (already `done`); leave those rows `done` once the tests are green. Not re-opening [[s-d04-workspace-disabled-gha]] or archived [[s-d04]] / [[s-d04-02]] / [[s-d01]]. Not [[s-l07-02-workspace-tests]] (L07.02 flags workspace fail is a different failure). Not D04.01 non-host triple LLVM smoke, not D05 strip/LTO, not P03 as a product row.

## Verify

`cargo test --workspace` prints `test result: ok.` and finishes with exit 0. `cargo test -p draconic-integration-tests --test cross_compile` still prints `test result: ok.` `cargo test -p draconic-integration-tests --test cross_compile_matrix` still prints `test result: ok.` `cargo test -p draconic-integration-tests --test release_artifact` still prints `test result: ok.` ROADMAP.md D04, D04.02, and D01.01 remain `done`. Live `.yml` names are not restored.

scope: `tests/integration/tests/cross_compile.rs`, `tests/integration/tests/cross_compile_matrix.rs`, `tests/integration/tests/release_artifact.rs`, `tests/integration/tests/website_pipeline.rs`, `.github/workflows/release-artifact.yml.disabled`, `.github/workflows/docs-pages.yml.disabled`, plus whatever D04 harness change is required so workspace tests pass without restoring live `.yml` names

## Links

[[s-d04-workspace-tests]] [[ticket-189-d04-workspace-tests]] [[s-d04-workspace-disabled-gha]]
