---
id: "task-744-cli-split-c-header"
title: "Split cli c_header.rs"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-715-cli-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Split cli c_header.rs

## Blocked by

None.

## Done

c_header.rs ≤1000. Same-file tests from 764 move with any new seam.

## Context

c_header.rs 1021, just over the target.

## Verify

cargo test -p draconic-cli. c_header.rs ≤1000.

scope: crates/draconic-cli/src/c_header.rs and new sibling if required

## Links

[[slice-715-cli-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Trim or split c_header.rs under 1000.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
1021 lines including tests.

**Desired behavior:**
≤1000.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [ ] c_header.rs ≤1000
- [ ] cargo test -p draconic-cli

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members

**Explain this part:**
A small extract of tests-with-seam is enough. Do not rewrite bindgen behaviour.
