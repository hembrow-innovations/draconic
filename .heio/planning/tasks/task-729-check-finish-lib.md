---
id: "task-729-check-finish-lib"
title: "Finish check lib.rs under 1000"
kind: task
status: ready
mode: afk
blocked_by: ["task-728-check-extract-checker"]
sprint: "rust-dev-audit"
slice: "slice-709-check-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Finish check lib.rs under 1000

## Blocked by

[[task-728-check-extract-checker]]: finish that sitting first.

## Done

Every crates/draconic-check .rs file is ≤1000.

## Context

Remaining lib.rs types, helpers, and tests after Binder and Checker extracts.

## Verify

Slice O1 and O2.

scope: crates/draconic-check/src/**

## Links

[[slice-709-check-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split leftover checker lib.rs until the crate is in budget.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
lib.rs still over 1000.

**Desired behavior:**
max-loc-ok.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [ ] Slice O1 max-loc-ok
- [ ] cargo test -p draconic-check

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members

**Explain this part:**
Split by remaining feature seams, not a util.rs junk drawer over 1000.
