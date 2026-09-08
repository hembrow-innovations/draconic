---
id: "task-743-cli-split-extract"
title: "Split cli extract.rs and tests/extract.rs"
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

# Split cli extract.rs and tests/extract.rs

## Blocked by

None.

## Done

src/extract.rs ≤1000 and tests/extract.rs ≤1000, splitting tests/extract_*.rs if needed.

## Context

extract.rs 1141. tests/extract.rs 2343 integration tests.

## Verify

cargo test -p draconic-cli.

scope: crates/draconic-cli/src/extract.rs and crates/draconic-cli/tests/extract*.rs

## Links

[[slice-715-cli-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split extract implementation and its integration tests.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
Two extract files over budget.

**Desired behavior:**
Each extract*.rs ≤1000.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [ ] src/extract.rs ≤1000
- [ ] no tests/extract*.rs over 1000
- [ ] cargo test -p draconic-cli

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Editing main.rs

**Explain this part:**
Integration tests may split under tests/. Do not invent crate-level unit tests that reach private helpers.
