---
id: "task-734-js-pointer-codes"
title: "Code JS pointer native_only diagnostics"
kind: task
status: completed
mode: afk
blocked_by: ["task-733-js-split-emit"]
sprint: "rust-dev-audit"
slice: "slice-711-backend-js-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T22:00:00Z"
---

# Code JS pointer native_only diagnostics

## Blocked by

[[task-733-js-split-emit]]: finish that sitting first.

## Done

Pointer native-only Diagnostic path calls with_code. Host/extern paths keep existing codes.

## Context

err-codes. native_only_diag in pre-split lib.rs used Diagnostic::new with no with_code for pointers.

## Verify

Slice O2 and O3.

scope: crates/draconic-backend-js/src/** and diagnostics codes only if a new code is required

## Links

[[slice-711-backend-js-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Attach a diagnostic code on JS pointer native-only failures.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
Pointer failures uncoded.

**Desired behavior:**
with_code on that path. Still a hard error, not JS emit.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] O3 sees with_code near native_only
- [x] cargo test -p draconic-backend-js

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Emitting JS for pointers

**Explain this part:**
Reuse HOST_API_UNSUPPORTED / EXTERN_UNSUPPORTED patterns already in this crate.

## Gauntlet

- **round 1**: `rg -n "native_only_diag|with_code" crates/draconic-backend-js/src`; `cargo test -p draconic-backend-js --offline` — **win**. native_only_diag calls with_code(POINTER_UNSUPPORTED). test result: ok. 64 passed. Host/extern paths unchanged. Promise `language.dual-worlds:native-only-hard-error-on-js` still hard-errors.
