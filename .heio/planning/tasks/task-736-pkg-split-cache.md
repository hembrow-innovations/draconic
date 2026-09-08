---
id: "task-736-pkg-split-cache"
title: "Split pkg cache.rs"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-713-pkg-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Split pkg cache.rs

## Blocked by

None.

## Done

cache.rs ≤1000 or cache_*.rs each ≤1000.

## Context

cache.rs 1277: layout, clone/fetch, checkout tests.

## Verify

cargo test -p draconic-pkg.

scope: crates/draconic-pkg/src/cache.rs and new cache_*.rs

## Links

[[slice-713-pkg-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split pkg cache.rs under 1000.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
cache.rs 1277 lines.

**Desired behavior:**
cache files ≤1000.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [ ] no cache*.rs over 1000
- [ ] cargo test -p draconic-pkg

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Editing lib.rs or resolve.rs

**Explain this part:**
Parallel with lib and resolve splits.
