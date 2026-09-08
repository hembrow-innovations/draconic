---
id: "task-724-linker-split-lib"
title: "Split linker lib.rs by load/export/json seams"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-708-linker-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Split linker lib.rs by load/export/json seams

## Blocked by

None.

## Done

Every crates/draconic-linker .rs file is ≤1000. Public API link_entry and link_entry_with_packages stay on the crate root.

## Context

Single 6004-line lib.rs holding load, export resolution, namespace synthesis, dynamic-import rewrite, rename, JSON modules, and tests from about 4924.

## Verify

Slice O1 and O2.

scope: crates/draconic-linker/src/**

## Links

[[slice-708-linker-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split the linker by feature seam.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
One 6004-line file.

**Desired behavior:**
Feature files ≤1000, thin lib.rs re-exports.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [ ] Slice O1 max-loc-ok
- [ ] cargo test -p draconic-linker
- [ ] link_entry still public

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Skipping err-codes; that is the next task

**Explain this part:**
If one file would exceed 1000 after extract, split further in this sitting. Never leave a new file over 1250.
