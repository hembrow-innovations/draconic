---
id: "task-722-ast-split-lib"
title: "Split ast lib.rs types vs dump"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-707-ast-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T22:00:00Z"
---

# Split ast lib.rs types vs dump

## Blocked by

None.

## Done

lib.rs ≤1000. Node types and dump_program live on real seams. Recursive nodes stay Box.

## Context

lib.rs 2462: Program/Stmt/Expr/TypeAnn plus dump_program around 1023. Split stmt/expr/type files or dump.rs. Keep re-exports from lib.rs.

## Verify

Slice O2. lib.rs ≤1000. no new file over 1000.

scope: crates/draconic-ast/src/lib.rs and new sibling files except print.rs

## Links

[[slice-707-ast-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split ast types and dump out of lib.rs.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
One 2462-line lib.rs.

**Desired behavior:**
Feature files ≤1000, Box AST unchanged.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] lib.rs ≤1000
- [x] no new file over 1000
- [x] cargo test -p draconic-ast
- [x] recursive Stmt/Expr still Box

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Editing print.rs

**Explain this part:**
Do not introduce Rc. Public node fields may stay pub as the AST crate allows.
