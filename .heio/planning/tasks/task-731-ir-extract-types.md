---
id: "task-731-ir-extract-types"
title: "Extract IR Module types from lib.rs"
kind: task
status: ready
mode: afk
blocked_by: ["task-730-ir-dump-pub-crate"]
sprint: "rust-dev-audit"
slice: "slice-710-ir-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Extract IR Module types from lib.rs

## Blocked by

[[task-730-ir-dump-pub-crate]]: finish that sitting first.

## Done

Module, locals, stmt/expr enums live in type files each ≤1000. lib.rs smaller. lower still public.

## Context

Types from Module around 101 through the enum soup before lower around 563. Split module.rs / stmt.rs / expr.rs as needed so no new file exceeds 1000.

## Verify

cargo test -p draconic-ir.

scope: crates/draconic-ir/src/**

## Links

[[slice-710-ir-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Move IR types out of the giant lib.rs.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
Types and lower share one 8474-line file.

**Desired behavior:**
Type files ≤1000. Re-export from lib.rs so backends keep compiling.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [ ] new type module files ≤1000
- [ ] lib.rs shorter
- [ ] cargo test -p draconic-ir

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members

**Explain this part:**
Do not change IR shape. Re-exports must keep downstream crates compiling.
