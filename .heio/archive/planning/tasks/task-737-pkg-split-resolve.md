---
id: "task-737-pkg-split-resolve"
title: "Split pkg resolve.rs"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-713-pkg-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T23:45:00Z"
---

# Split pkg resolve.rs

## Blocked by

None.

## Done

resolve.rs ≤1000 or resolve_*.rs each ≤1000.

## Context

resolve.rs 1121: tag resolve and direct-dep pinning.

## Verify

Slice O1 after the three pkg tasks; this sitting at least resolve files ≤1000.

scope: crates/draconic-pkg/src/resolve.rs and new resolve_*.rs

## Links

[[slice-713-pkg-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split pkg resolve.rs under 1000.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
resolve.rs 1121 lines.

**Desired behavior:**
resolve files ≤1000.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] no resolve*.rs over 1000
- [x] cargo test -p draconic-pkg

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Editing lib.rs or cache.rs

**Explain this part:**
Parallel with lib and cache splits.

## Gauntlet

- **round:** 1
- **command:** cargo test -p draconic-pkg --offline
- **result:** win
- **gap:** none. 313 passed. resolve.rs 667, resolve_direct.rs 542.
