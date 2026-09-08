---
id: "task-716-lexer-split-lib"
title: "Split lexer lib.rs by feature seam"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-705-lexer-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T21:30:00Z"
---

# Split lexer lib.rs by feature seam

## Blocked by

None.

## Done

crates/draconic-lexer/src/lib.rs is ≤1000 lines. New sibling .rs files are each ≤1000. Public tokenize API unchanged.

## Context

lib.rs is 2926 lines: JsString, TokenKind, Lexer, scan, and tests from about 1832. regexp.rs already exists. Split by seam (token kinds, js string, scan) with tests moving into the new files. Do not grow regexp.rs past 1000.

## Verify

Slice O1 and O2. Count lines on every new .rs.

scope: crates/draconic-lexer/src/** except changing regexp Diagnostic return types

## Links

[[slice-705-lexer-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split lexer lib.rs under the 1000-line target.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
One lib.rs at 2926 lines.

**Desired behavior:**
Feature files plus a thin lib.rs. Target ≤1000, never leave a file over 1250.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] lib.rs ≤1000
- [x] no new lexer .rs file over 1000
- [x] cargo test -p draconic-lexer
- [x] No crate-level tests/ for these units

## Gauntlet

- **round**: 1
- **command**: python3 O1 max-loc on crates/draconic-lexer; cargo test -p draconic-lexer --offline
- **win/lose**: win
- **gap**: none

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members

**Explain this part:**
Count whole files including tests.
