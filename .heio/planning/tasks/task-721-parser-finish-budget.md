---
id: "task-721-parser-finish-budget"
title: "Finish parser until every file ≤1000"
kind: task
status: ready
mode: afk
blocked_by: ["task-720-parser-extract-module"]
sprint: "rust-dev-audit"
slice: "slice-706-parser-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Finish parser until every file ≤1000

## Blocked by

[[task-720-parser-extract-module]]: finish that sitting first.

## Done

Every crates/draconic-parser .rs file is ≤1000 including lib.rs and tests in those files.

## Context

Whatever remains in lib.rs (helpers from about 5439, leftover impl, tests) must split by seam until the crate oracle holds. fuzz.rs stays.

## Verify

Slice O1 and O2.

scope: crates/draconic-parser/src/**

## Links

[[slice-706-parser-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Drain remaining parser overage until the crate is in budget.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
lib.rs still over 1000 after prior extracts.

**Desired behavior:**
max-loc-ok on the crate.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [ ] Slice O1 max-loc-ok
- [ ] cargo test -p draconic-parser
- [ ] parse and parse_module still public in lib.rs

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members

**Explain this part:**
Helpers that are only used by one seam belong with that seam, not a junk drawer over 1000.
