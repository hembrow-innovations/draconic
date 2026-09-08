---
id: "task-725-linker-err-codes"
title: "Attach codes::* on linker diagnostics"
kind: task
status: ready
mode: afk
blocked_by: ["task-724-linker-split-lib"]
sprint: "rust-dev-audit"
slice: "slice-708-linker-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Attach codes::* on linker diagnostics

## Blocked by

[[task-724-linker-split-lib]]: finish that sitting first.

## Done

Every linker Diagnostic::new used as a compiler failure has with_code. Reuse existing codes when they match; add codes in draconic-diagnostics only when no code exists.

## Context

err-codes. Failed module read, undeclared export, relative-specifier were uncoded in the pre-split file. After split, grep the crate.

## Verify

Slice O2 and O3.

scope: crates/draconic-linker/src/** and crates/draconic-diagnostics/src/lib.rs only if a new code is required

## Links

[[slice-708-linker-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Give linker failures stable diagnostic codes.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
Diagnostic::new without with_code.

**Desired behavior:**
linker-codes-ok on the crate.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [ ] Slice O3 linker-codes-ok
- [ ] cargo test -p draconic-linker
- [ ] No anyhow

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members

**Explain this part:**
Do not change which failures are hard errors. Only attach codes.
