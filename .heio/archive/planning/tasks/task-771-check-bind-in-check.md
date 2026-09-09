---
id: "task-771-check-bind-in-check"
title: "Bind inside check one walk"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-709-check-file-budget"
tags: []
created_at: "2026-09-09T17:30:00Z"
updated_at: "2026-09-09T23:30:00Z"
---
# Bind inside check one walk

## Blocked by

None. Prefactor for [[task-727-check-extract-binder]].

## Done

There is one Stmt/Expr walk that both resolves symbols and checks types. bind() may remain as a wrapper for tests that call it, but bind_stmt and check_stmt are not two full matches over the 26 Stmt variants. Existing check diagnostics still pass. cargo test -p draconic-check is green.

## Context

Dragons audit: extracting Binder then Checker as two files would freeze two walks. Merge first, then file-budget split the one walk. If error order would change, stop and file a ticket — do not "fix" tests to a new order in this sitting.

## Verify

cargo test -p draconic-check --offline. bind() still works for tests that use BoundProgram. lib.rs is not larger than 9342 unless a sibling absorbed the walk (siblings ≤1000).

scope: crates/draconic-check/src/lib.rs and new private siblings the walk needs

## Links

[[slice-709-check-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** One AST walk for bind and check.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: delete the second Stmt match
- Contract-first: do not invent language behaviour

**Current behavior:**
Binder bind_stmt ~1859 and Checker check_stmt ~3375 both match Stmt.

**Desired behavior:**
One walk. Public check / check_module / bind entries stay.

**Key interfaces:**
- check, check_module, bind, bind_module
- size-file-budget: no new file over 1000; do not grow lib.rs

**Acceptance criteria:**
- [x] not two full bind_stmt + check_stmt walks
- [x] cargo test -p draconic-check
- [x] public check/bind entries remain
- [x] if diagnostics change order, stop and ticket

**Out of scope:**
- Extracting binder.rs as a second pass (task-727 waits and must not recreate two walks)
- JS/LLVM emit
- host_api split (task-726)
- Language behaviour or ROADMAP rows

**Explain this part:**
Keep bind() if callers need BoundProgram. Implement it through the one walk, not a parallel visitor.

## Gauntlet

- **Round 1:** `cargo test -p draconic-check --offline -- --skip catalog_sync` — lose — bind-only could emit type-parameter and extern ABI diagnostics.
- **Round 2:** same command — win — one `check_stmt`/`check_expr` walk; `bind_stmt`/`bind_expr` gone; lib.rs 8866 lines; 277 passed. `catalog_sync` skipped (unrelated dirty host_api from task-726).
