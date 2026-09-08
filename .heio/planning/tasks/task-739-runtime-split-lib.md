---
id: "task-739-runtime-split-lib"
title: "Split runtime lib.rs"
kind: task
status: ready
mode: afk
blocked_by: ["task-738-runtime-extract-inline-mods"]
sprint: "rust-dev-audit"
slice: "slice-714-runtime-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Split runtime lib.rs

## Blocked by

[[task-738-runtime-extract-inline-mods]]: finish that sitting first.

## Done

lib.rs ≤1000. Remaining clang archive / GC tests / helpers live on seams.

## Context

lib.rs 3897 after inline mods still owns crypto leftovers, testing, URL, clang archive build, and about 3000 lines of GC tests.

## Verify

cargo test -p draconic-runtime. lib.rs ≤1000.

scope: crates/draconic-runtime/src/lib.rs and new sibling files except abi.rs and host_abi_tests.rs

## Links

[[slice-714-runtime-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split runtime lib.rs under 1000.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
lib.rs still thousands of lines after extracting inline mods.

**Desired behavior:**
lib.rs ≤1000.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [ ] lib.rs ≤1000
- [ ] cargo test -p draconic-runtime

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Editing abi.rs or host_abi_tests.rs

**Explain this part:**
Runtime may keep split *_tests.rs declared from lib.rs.
