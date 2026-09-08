---
id: "task-727-check-extract-binder"
title: "Extract Binder from check lib.rs"
kind: task
status: ready
mode: afk
blocked_by: ["task-771-check-bind-in-check"]
sprint: "rust-dev-audit"
slice: "slice-709-check-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Extract Binder from check lib.rs

## Blocked by

[[task-771-check-bind-in-check]]: one walk before file-splitting bind helpers.

## Done

Bind helpers live in private sibling files ≤1000. Do not recreate a second Stmt walk.

## Context

After bind-in-check there is one walk. Split leftover bind helpers as private modules of that walk. Do not reintroduce Binder as a second pass. lib.rs keeps check() entry.

## Verify

cargo test -p draconic-check. binder*.rs ≤1000. lib.rs shorter.

scope: crates/draconic-check/src/lib.rs and new binder*.rs

## Links

[[slice-709-check-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Move Binder out of check lib.rs.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
Binder is a private type in the 9342-line lib.rs.

**Desired behavior:**
binder*.rs ≤1000.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [ ] mod binder in lib.rs
- [ ] binder files ≤1000
- [ ] lib.rs shorter than 9342
- [ ] cargo test -p draconic-check

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Extracting Checker in this sitting

**Explain this part:**
Do not recreate bind_stmt as a full parallel walk. Do not extract Checker here. That is the next task.
