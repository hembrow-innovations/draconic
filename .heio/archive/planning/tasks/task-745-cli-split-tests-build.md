---
id: "task-745-cli-split-tests-build"
title: "Split cli tests/build.rs"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-715-cli-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T22:20:00Z"
---

# Split cli tests/build.rs

## Blocked by

None.

## Done

tests/build.rs ≤1000 or tests/build_*.rs each ≤1000.

## Context

tests/build.rs 1227 binary integration tests.

## Verify

Slice O1 after sibling CLI tasks; this sitting finishes build integration tests ≤1000.

scope: crates/draconic-cli/tests/build.rs and new tests/build_*.rs

## Links

[[slice-715-cli-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split CLI build integration tests under 1000.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
1227-line tests/build.rs.

**Desired behavior:**
Each file ≤1000.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] no tests/build*.rs over 1000
- [x] cargo test -p draconic-cli

## Gauntlet

- **round 1**: `cargo test -p draconic-cli --offline` — win. build.rs 250, build_link.rs 283, build_packages.rs 786; all CLI tests ok.

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members

**Explain this part:**
Keep spawning CARGO_BIN_EXE_draconic. Do not use --lib as a language oracle.
