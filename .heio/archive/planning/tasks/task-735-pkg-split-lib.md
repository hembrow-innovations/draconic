---
id: "task-735-pkg-split-lib"
title: "Split pkg lib.rs"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-713-pkg-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T22:00:00Z"
---

# Split pkg lib.rs

## Blocked by

None.

## Done

lib.rs ≤1000. K01 manifest parse/validate/write and tests live on seams.

## Context

lib.rs 1856. Keep public re-exports. Split validate/write/parse if needed.

## Verify

cargo test -p draconic-pkg. lib.rs ≤1000.

scope: crates/draconic-pkg/src/lib.rs and new sibling files not named cache.rs or resolve.rs

## Links

[[slice-713-pkg-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split pkg crate root under 1000.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
lib.rs 1856 lines.

**Desired behavior:**
lib.rs ≤1000, no new file over 1000.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] lib.rs ≤1000
- [x] cargo test -p draconic-pkg
- [x] Error enums still hand-written

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Editing cache.rs or resolve.rs

**Explain this part:**
Do not add serde. toml::Value hand-walk stays.

## Gauntlet

- **round:** 1
- **command:** cargo test -p draconic-pkg --offline
- **result:** win
- **gap:** none. 313 passed. lib.rs 196. New seams manifest.rs, parse.rs, validate.rs, write.rs all under 1000.
