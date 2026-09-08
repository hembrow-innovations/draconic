---
id: "task-723-ast-split-print"
title: "Split ast print.rs"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-707-ast-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Split ast print.rs

## Blocked by

None.

## Done

print.rs ≤1000 or replaced by print_*.rs files each ≤1000. print_program still public.

## Context

print.rs is 1703 lines, all printer. Split by stmt vs expr vs types. Tests print_simple_let and print_block_indent move with the seam.

## Verify

cargo test -p draconic-ast. No print*.rs over 1000.

scope: crates/draconic-ast/src/print.rs and new print_*.rs

## Links

[[slice-707-ast-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split the AST printer under 1000 lines.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
print.rs 1703 lines.

**Desired behavior:**
Printer files ≤1000.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [ ] no print*.rs over 1000
- [ ] print_program still public
- [ ] cargo test -p draconic-ast

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Rewriting dump_program

**Explain this part:**
Parallel with the lib.rs split; do not edit lib.rs types in this sitting.
