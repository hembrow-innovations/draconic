---
id: "task-730-ir-dump-pub-crate"
title: "Make dump_module pub(crate)"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-710-ir-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T08:45:00Z"
---
# Make dump_module pub(crate)

## Blocked by

None.

## Done

dump_module is pub(crate). In-crate tests still compile.

## Context

vis-pub-crate. dump_module around 6763 is pub, only used in this crate's tests.

## Verify

Slice O2 and O3.

scope: crates/draconic-ir/src/lib.rs (dump_module and callers in this crate only)

## Links

[[slice-710-ir-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Hide dump_module from the public IR surface.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
pub fn dump_module.

**Desired behavior:**
pub(crate) fn dump_module.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] O3 matches pub(crate) fn dump_module
- [x] cargo test -p draconic-ir

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Splitting lower in this sitting

**Explain this part:**
Do not delete dump_module. Tests need it.
