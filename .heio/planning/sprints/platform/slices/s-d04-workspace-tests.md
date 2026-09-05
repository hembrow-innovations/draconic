---
id: "s-d04-workspace-tests"
title: "D04 workspace tests pass"
kind: slice
status: active
sprint: "platform"
tags: []
created_at: "2026-09-05T01:29:39Z"
updated_at: "2026-09-05T01:29:39Z"
claimed-by: 0c0f76f4-edcd-49fd-8def-b8c42031ab1c
---

# D04 workspace tests pass

## Why

Review of [[s-d04-workspace-disabled-gha]] left ROADMAP D04 unfinished as a workspace gate: O1 (`cross_compile`), O2 (`cross_compile_matrix`), and O3 (`release_artifact`) held against `.yml.disabled`, but O4 `cargo test --workspace` exited 101. The distribution location still needs the D04 Loop to leave the workspace green, not only the disabled-GHA matrix and release-artifact readers.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D04 work. The D04, D04.02, and D01.01 integration readers stay green against `.github/workflows/release-artifact.yml.disabled` and `docs-pages.yml.disabled`. If the failure comes from the D04 harness change, fix that matrix/CI contract so the workspace check and those readers hold. Do not restore live `.yml` names. Do not reopen ROADMAP D04 / D04.02 / D01.01 (already `done`); leave those rows `done` once the tests are green.

## Blocked by

None. The ticket names no dependency.

## Non-goals

- **Re-enabling GitHub Actions**: do not rename `.yml.disabled` back to `.yml`
- **Re-opening [[s-d04-workspace-disabled-gha]]**: that slice stays sealed `failed`
- **Re-opening archived [[s-d04]] / [[s-d04-02]] / [[s-d01]]**: those slices stay sealed
- **[[s-l07-02-workspace-tests]]**: L07.02 flags workspace fail is a different failure
- **D04.01**: non-host triple LLVM smoke
- **D05**: strip / LTO size flags
- **P03**: docs site product row; only the disabled workflow path the pipeline test reads

## Oracle checklist

- [ ] O1: workspace tests pass after the D04 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: pending

- [ ] O2: D04 combined matrix sitting (docs + CI + host LLVM emit) stays green
  CHECK: cargo test -p draconic-integration-tests --test cross_compile
  EXPECT: test result: ok.
  EVIDENCE: pending

- [ ] O3: D04.02 matrix docs + CI jobs fixtures stay locked
  CHECK: cargo test -p draconic-integration-tests --test cross_compile_matrix
  EXPECT: test result: ok.
  EVIDENCE: pending

- [ ] O4: D01.01 release-artifact workflow reader stays locked
  CHECK: cargo test -p draconic-integration-tests --test release_artifact
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

Durable links to task-pool ids. Never drop them.

- `[[d04-workspace-tests]]`

## See also

ROADMAP.md D04, D04.02, D01.01, `tests/integration/tests/cross_compile.rs`, `tests/integration/tests/cross_compile_matrix.rs`, `tests/integration/tests/release_artifact.rs`, `tests/integration/tests/website_pipeline.rs`, `.github/workflows/release-artifact.yml.disabled`, `.github/workflows/docs-pages.yml.disabled`, CONTEXT.md, [[distribution]], [[s-d04-workspace-disabled-gha]], [[ticket-189-d04-workspace-tests]].
