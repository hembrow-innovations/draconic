---
id: "ticket-187-workspace-disabled-gha-workflow"
title: "Workspace tests fail: release-artifact.yml renamed .disabled"
kind: ticket
status: closed
labels: bug
tags: []
sprint: platform
slice: "s-d04-workspace-disabled-gha"
created_at: "2026-09-05T00:59:29Z"
updated_at: "2026-09-05T05:08:49Z"
intent: fix
---

# Workspace tests fail: release-artifact.yml renamed .disabled

## Signal

`cargo test --workspace --offline` finishes (does not hang) and fails in `-p draconic-integration-tests --test cross_compile`:

- **test**: `docs_ci_and_host_llvm_emit_form_one_available_matrix`
- **panic**: missing `.github/workflows/release-artifact.yml` (D04)
- **tree**: commit `97bbcc4` renamed that workflow to `release-artifact.yml.disabled` (and `docs-pages.yml` the same way)
- **other readers**: `tests/integration/tests/cross_compile_matrix.rs`, `tests/integration/tests/release_artifact.rs`

L07.02 flags conformance stays green. This is not a flags hang.

## Fit

This project, later slice. Not [[s-l07-02-workspace-timeout]]. Do not re-enable GitHub Actions as a silent side effect of an L07.02 Loop.

## Notes

- **ROADMAP**: D04 / D04.02 name the CI workflow as the matrix lock
- **intent**: keep Actions disabled as `97bbcc4` did, or restore the `.yml` names so workspace is green — pick one, do not do both by accident
