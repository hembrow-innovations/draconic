---
id: "d04-workspace-disabled-gha"
title: "D04 workspace tests vs disabled GHA workflow"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-05T01:05:20Z"
updated_at: "2026-09-05T01:23:44Z"
---

# D04 workspace tests vs disabled GHA workflow

## Blocked by

None.

## Done

Keep `.github/workflows/release-artifact.yml.disabled` and `docs-pages.yml.disabled` as the on-disk names; retarget the D04 / D04.02 / D01.01 / website_pipeline readers at those files so `cargo test --workspace` finishes with `test result: ok.`

## Context

Roadmap ID **D04** (`Cross-compile: Linux/macOS/Windows artifacts; CI matrix documented`). Commit `97bbcc4` renamed `.github/workflows/release-artifact.yml` to `release-artifact.yml.disabled` (and `docs-pages.yml` the same way). Workspace tests now fail: `docs_ci_and_host_llvm_emit_form_one_available_matrix` panics on the live `.yml` path. D04.02 (`cross_compile_matrix`) and D01.01 (`release_artifact`) read the same file; `website_pipeline` still joins `.github/workflows/docs-pages.yml`. Desired: keep Actions disabled as `97bbcc4` did; update those integration readers so they lock the matrix/CI contract against the disabled workflow files instead of requiring live `.yml` paths. D04, D04.02, and D01.01 stay `done` on ROADMAP.md (do not reopen those rows). Not restoring live `.yml` names, not re-opening archived [[s-d04]] / [[s-d04-02]] / [[s-d01]], not the L07.02 flags hang, not D04.01 non-host triple LLVM smoke, not D05 strip/LTO, not P03 as a product row.

## Verify

`cargo test -p draconic-integration-tests --test cross_compile` prints `test result: ok.` `cargo test -p draconic-integration-tests --test cross_compile_matrix` prints `test result: ok.` `cargo test -p draconic-integration-tests --test release_artifact` prints `test result: ok.` `cargo test --workspace` prints `test result: ok.` and finishes. ROADMAP.md D04, D04.02, and D01.01 remain `done`. Live `.yml` names are not restored.

scope: `tests/integration/tests/cross_compile.rs`, `tests/integration/tests/cross_compile_matrix.rs`, `tests/integration/tests/release_artifact.rs`, `tests/integration/tests/website_pipeline.rs`, `.github/workflows/release-artifact.yml.disabled`, `.github/workflows/docs-pages.yml.disabled`

## Links

[[s-d04-workspace-disabled-gha]] [[ticket-187-workspace-disabled-gha-workflow]]
