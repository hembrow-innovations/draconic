---
id: "task-732-ir-split-lower"
title: "Split IR lower until every file ≤1000"
kind: task
status: ready
mode: afk
blocked_by: ["task-731-ir-extract-types"]
sprint: "rust-dev-audit"
slice: "slice-710-ir-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Split IR lower until every file ≤1000

## Blocked by

[[task-731-ir-extract-types]]: finish that sitting first.

## Done

Every crates/draconic-ir .rs file is ≤1000. lower remains the public lowering entry.

## Context

lower, class desugar, eval rewrite, private, patterns, dump helper, tests around 7760. Split by those seams.

## Verify

Slice O1 and O2.

scope: crates/draconic-ir/src/**

## Links

[[slice-710-ir-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split lowering until the IR crate is in budget.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
lower still over budget after types extract.

**Desired behavior:**
max-loc-ok. No process-global lower state.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [ ] Slice O1 max-loc-ok
- [ ] pub fn lower still the entry
- [ ] cargo test -p draconic-ir

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Process-global lower state

**Explain this part:**
Keep LowerCtx owned by one lower call.
