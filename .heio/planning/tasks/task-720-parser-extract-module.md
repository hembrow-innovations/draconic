---
id: "task-720-parser-extract-module"
title: "Extract parser module and type seams"
kind: task
status: ready
mode: afk
blocked_by: ["task-719-parser-extract-expr"]
sprint: "rust-dev-audit"
slice: "slice-706-parser-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Extract parser module and type seams

## Blocked by

[[task-719-parser-extract-expr]]: finish that sitting first.

## Done

import/export, type annotations, and binding patterns live in sibling files each ≤1000.

## Context

parse_import around 2462, parse_export around 2688, parse_type around 2143, parse_binding_pattern around 3153. Extract module/type/pattern files. Move matching tests.

## Verify

cargo test -p draconic-parser.

scope: crates/draconic-parser/src/**

## Links

[[slice-706-parser-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Pull import/export/type/pattern parsing out of lib.rs.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
Those methods still sit in lib.rs after expr extract.

**Desired behavior:**
New files ≤1000. lib.rs smaller.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [ ] New files for module and/or type and/or pattern
- [ ] Each new file ≤1000
- [ ] cargo test -p draconic-parser

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members

**Explain this part:**
Do not change Module vs Script policy. Frontend owns that.
