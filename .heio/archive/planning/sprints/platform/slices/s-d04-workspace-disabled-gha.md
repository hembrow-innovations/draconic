---
id: "s-d04-workspace-disabled-gha"
title: "D04 workspace tests vs disabled GHA workflow"
kind: slice
status: failed
sprint: "platform"
tags: []
created_at: "2026-09-05T01:03:14Z"
updated_at: "2026-09-05T01:28:20Z"
claimed-by: 660127b2-b539-4013-981a-b11903e32f04
---

# D04 workspace tests vs disabled GHA workflow

## Why

`cargo test --workspace` fails after commit `97bbcc4` renamed `.github/workflows/release-artifact.yml` to `release-artifact.yml.disabled` (and `docs-pages.yml` the same way). ROADMAP D04 still locks the cross-compile matrix through `docs_ci_and_host_llvm_emit_form_one_available_matrix`, which panics on the live `.yml` path. D04.02 (`cross_compile_matrix`) and D01.01 (`release_artifact`) read the same file. This is a distribution harness/tree mismatch, not a flags hang and not a new D04 feature row.

## Done

Keep GitHub Actions disabled as `97bbcc4` did: `.github/workflows/release-artifact.yml.disabled` and `docs-pages.yml.disabled` stay the on-disk names. Update the D04, D04.02, and D01.01 integration readers (and the docs-pages reader in `website_pipeline`) so they lock the matrix/CI contract against those disabled workflow files instead of requiring live `.yml` paths. `cargo test --workspace` finishes with `test result: ok.` Do not restore live `.yml` names. Do not reopen ROADMAP D04 / D04.02 / D01.01 (already `done`); leave those rows `done` once the tests are green.

## Blocked by

None. The ticket names no dependency.

## Non-goals

- **Re-enabling GitHub Actions**: do not rename `.yml.disabled` back to `.yml`
- **Re-opening archived [[s-d04]] / [[s-d04-02]] / [[s-d01]]**: those slices stay sealed
- **[[s-l07-02-workspace-timeout]]**: L07.02 flags hang is a different failure
- **D04.01**: non-host triple LLVM smoke
- **D05**: strip / LTO size flags
- **P03**: docs site product row; only the disabled workflow path the pipeline test reads

## Oracle checklist

- [x] O1: D04 combined matrix sitting (docs + CI + host LLVM emit) is green
  CHECK: cargo test -p draconic-integration-tests --test cross_compile
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=79065a07acdc68d3 bytes=3207 at=2026-09-05T01:25:00.809Z

- [x] O2: D04.02 matrix docs + CI jobs fixtures stay locked
  CHECK: cargo test -p draconic-integration-tests --test cross_compile_matrix
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=5ca1f3290e5c3cc2 bytes=3021 at=2026-09-05T01:25:03.676Z

- [x] O3: D01.01 release-artifact workflow reader stays locked
  CHECK: cargo test -p draconic-integration-tests --test release_artifact
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=08c89f253f2d70b2 bytes=3019 at=2026-09-05T01:25:06.986Z

- [ ] O4: workspace tests finish after the disabled-GHA harness fix
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: unmet exit=101 match=yes bytes=67805 at=2026-09-05T01:27:28.138Z
  ABANDON: cargo test --workspace exited 101 after D04/D04.02/D01.01 readers were green → ticket-189-d04-workspace-tests

## Pool

Durable links to task-pool ids. Never drop them.

- `[[d04-workspace-disabled-gha]]`

## See also

ROADMAP.md D04, D04.02, D01.01, `tests/integration/tests/cross_compile.rs`, `tests/integration/tests/cross_compile_matrix.rs`, `tests/integration/tests/release_artifact.rs`, `tests/integration/tests/website_pipeline.rs`, `.github/workflows/release-artifact.yml.disabled`, `.github/workflows/docs-pages.yml.disabled`, CONTEXT.md, [[distribution]], [[ticket-187-workspace-disabled-gha-workflow]].
