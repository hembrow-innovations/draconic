---
id: "task-728-check-extract-checker"
title: "Extract Checker from check lib.rs"
kind: task
status: completed
mode: afk
blocked_by: ["task-727-check-extract-binder"]
sprint: "rust-dev-audit"
slice: "slice-709-check-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T23:55:00Z"
---

# Extract Checker from check lib.rs

## Blocked by

[[task-727-check-extract-binder]]: finish that sitting first.

## Done

Checker lives in checker.rs (and checker_*.rs if needed), each ≤1000, with its tests.

## Context

struct Checker at about 2911. Extract type and impl.

## Verify

cargo test -p draconic-check.

scope: crates/draconic-check/src/lib.rs and new checker*.rs

## Links

[[slice-709-check-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Move Checker out of check lib.rs.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
Checker still in lib.rs after Binder extract.

**Desired behavior:**
checker*.rs ≤1000. lib.rs smaller.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] mod checker in lib.rs
- [x] checker files ≤1000
- [x] cargo test -p draconic-check

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members

**Explain this part:**
Public check/check_module/check_for_target stay on the crate root.

## Gauntlet

- **Round 1:** `cargo test -p draconic-check --offline -- --skip catalog_sync` — win — `mod checker` plus checker.rs / checker_stmt.rs / checker_expr.rs / checker_type.rs / checker_assign.rs / checker_ops.rs, each ≤1000; one `check_stmt` walk; lib.rs 4189 lines; 277 passed. `catalog_sync` skipped (unrelated dirty host_api from task-726).
