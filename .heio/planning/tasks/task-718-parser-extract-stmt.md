---
id: "task-718-parser-extract-stmt"
title: "Extract parser statement seam"
kind: task
status: ready
mode: afk
blocked_by: ["task-770-parser-context"]
sprint: "rust-dev-audit"
slice: "slice-706-parser-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Extract parser statement seam

## Blocked by

[[task-770-parser-context]]: one context value before extracting stmt files.

## Done

Statement parsing lives in sibling file modules, each ≤1000 lines. lib.rs is strictly smaller. parse/parse_module still in lib.rs.

## Context

Parser impl in lib.rs: parse_stmt around 334 through control-flow, using, function/class decls before import/export. Extract stmt (and further stmt_* files if one file would exceed 1000). Move matching tests from the bottom tests module. Parser struct stays in lib.rs unless a later task moves it.

## Verify

cargo test -p draconic-parser. New files ≤1000. lib.rs line count dropped.

scope: crates/draconic-parser/src/**

## Links

[[slice-706-parser-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Pull statement parsing out of parser lib.rs.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
parse_stmt and friends live in the 10k-line lib.rs.

**Desired behavior:**
stmt*.rs files ≤1000 with same-file tests. No new file over 1250.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [ ] At least one new stmt*.rs
- [ ] Each new file ≤1000 lines
- [ ] lib.rs shorter than 10483
- [ ] cargo test -p draconic-parser

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members

**Explain this part:**
Sibling files are `impl Parser` in extra files, not a new public parse interface. If statement parsing itself exceeds 1000, split for/class/function in the same sitting.
