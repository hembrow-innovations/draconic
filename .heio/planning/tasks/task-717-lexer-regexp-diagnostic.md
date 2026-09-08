---
id: "task-717-lexer-regexp-diagnostic"
title: "Regexp validators return Diagnostic"
kind: task
status: ready
mode: afk
blocked_by: ["task-716-lexer-split-lib"]
sprint: "rust-dev-audit"
slice: "slice-705-lexer-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Regexp validators return Diagnostic

## Blocked by

[[task-716-lexer-split-lib]]: finish that sitting first.

## Done

validate_regexp_literal and validate_regexp_flags return Result<(), Diagnostic>.

## Context

err-diagnostic: compiler-path helpers must not return String. These two fns are in regexp.rs and re-exported from lib.rs. Callers must compile.

## Verify

Slice O2 and O3.

scope: crates/draconic-lexer/src/regexp.rs and call sites that name these fns

## Links

[[slice-705-lexer-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Replace String errors on regexp validators with Diagnostic.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
Result<(), String> on validate_regexp_literal / validate_regexp_flags.

**Desired behavior:**
Result<(), Diagnostic>. No anyhow.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [ ] Both fns return Result<(), Diagnostic>
- [ ] No Result<(), String> on those fns
- [ ] cargo test -p draconic-lexer

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Inventing a second Diagnostic type

**Explain this part:**
Keep the re-export. Do not change regexp syntax rules.
