---
id: "task-719-parser-extract-expr"
title: "Extract parser expression seam"
kind: task
status: completed
mode: afk
blocked_by: [ "task-718-parser-extract-stmt" ]
sprint: "rust-dev-audit"
slice: "slice-706-parser-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T22:35:00Z"
---
# Extract parser expression seam

## Blocked by

[[task-718-parser-extract-stmt]]: finish that sitting first.

## Done

Expression parsing lives in sibling file modules each ≤1000. lib.rs is strictly smaller than after the stmt extract.

## Context

parse_expr around 3330 through primary, assignment, object/array, template. Extract expr*.rs. Move matching tests.

## Verify

cargo test -p draconic-parser. New expr files ≤1000.

scope: crates/draconic-parser/src/**

## Links

[[slice-706-parser-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Pull expression parsing out of parser lib.rs.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
parse_expr and precedence chain still in lib.rs after stmt extract.

**Desired behavior:**
expr*.rs ≤1000 with same-file tests.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] At least one new expr*.rs
- [x] Each new file ≤1000
- [x] lib.rs shorter than after stmt extract
- [x] cargo test -p draconic-parser

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members

**Explain this part:**
Keep Parser methods callable across modules via impl Parser in each file.

## Gauntlet

- **round**: 1
- **command**: cargo test -p draconic-parser --offline; python line counts on crates/draconic-parser/src/*.rs
- **win/lose**: win
- **gap**: none

Expression parsing moved to expr.rs, expr_ops.rs, expr_lhs.rs, expr_object.rs, expr_primary.rs. parse_class_expression stays in stmt_class.rs. Function expressions, types, import/export, and patterns stay in lib.rs for task-720/721.
