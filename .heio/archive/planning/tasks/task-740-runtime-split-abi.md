---
id: "task-740-runtime-split-abi"
title: "Split runtime abi.rs"
kind: task
status: completed
mode: afk
blocked_by: ["task-768-abi-polyfills-keyed"]
sprint: "rust-dev-audit"
slice: "slice-714-runtime-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T21:00:00Z"
---

# Split runtime abi.rs

## Blocked by

[[task-768-abi-polyfills-keyed]]: polyfills keyed by catalog name before splitting abi.rs.

## Done

abi.rs ≤1000. LLVM ABI catalog vs process/worker/cancel/timer/stdio/path/fs JS polyfills live on host_* or abi_* seams.

## Context

abi.rs 3270 mixes catalog and JS polyfills.

## Verify

cargo test -p draconic-runtime. abi*.rs ≤1000.

scope: crates/draconic-runtime/src/abi.rs and new abi_* or host polyfill files under runtime

## Links

[[slice-714-runtime-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split runtime abi.rs under 1000.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
One 3270-line abi.rs.

**Desired behavior:**
Files ≤1000.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] no abi*.rs over 1000
- [x] cargo test -p draconic-runtime

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Editing lib.rs inline mods

**Explain this part:**
Parallel with extract/split of lib.rs as long as you do not edit lib.rs in this sitting.

## Gauntlet

- **round 1**: `cargo test -p draconic-runtime --offline` — win. 163 passed. abi*.rs all ≤1000 (abi.rs 746). No critic gap.
