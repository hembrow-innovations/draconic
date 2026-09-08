---
id: "task-738-runtime-extract-inline-mods"
title: "Extract runtime crypto testing url file modules"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-714-runtime-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Extract runtime crypto testing url file modules

## Blocked by

None.

## Done

crypto.rs, testing.rs, and url.rs exist. lib.rs uses mod crypto; not an inline block. Re-exports unchanged.

## Context

arch-file-modules. Inline pub mod crypto around line 58, testing around 186, url around 275 in lib.rs.

## Verify

Slice O2 and O3. New files ≤1000.

scope: crates/draconic-runtime/src/lib.rs and new crypto.rs testing.rs url.rs

## Links

[[slice-714-runtime-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Move inline runtime mods onto sibling files.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
crypto/testing/url are inline modules in lib.rs.

**Desired behavior:**
Sibling files, each ≤1000.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [ ] crypto.rs testing.rs url.rs exist
- [ ] each ≤1000
- [ ] cargo test -p draconic-runtime

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Splitting abi.rs or host_abi_tests.rs

**Explain this part:**
Keep pub use so polyfill names do not move.
