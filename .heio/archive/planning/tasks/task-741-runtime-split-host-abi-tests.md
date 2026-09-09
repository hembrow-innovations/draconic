---
id: "task-741-runtime-split-host-abi-tests"
title: "Split runtime host_abi_tests.rs"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-714-runtime-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T22:30:00Z"
---

# Split runtime host_abi_tests.rs

## Blocked by

None.

## Done

host_abi_tests.rs ≤1000 or further host_*_tests.rs files each ≤1000, still declared from lib.rs.

## Context

host_abi_tests.rs 5143 is the allowed runtime test exception but still one bag. Split by host_* seam.

## Verify

Slice O1 depends on sibling tasks; this sitting finishes the tests file family ≤1000.

scope: crates/draconic-runtime/src/host_abi_tests.rs and new host_*_tests.rs plus lib.rs mod declarations only

## Links

[[slice-714-runtime-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split host_abi_tests.rs by host seam.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
One 5143-line test module.

**Desired behavior:**
Each tests file ≤1000.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] no host_abi_tests*.rs or split host_*_tests.rs over 1000
- [x] cargo test -p draconic-runtime

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Moving these tests to crate-level tests/

**Explain this part:**
Declare new test modules from lib.rs with cfg(test).

## Gauntlet

- round 1: `cargo test -p draconic-runtime --offline` plus loc checks — win — 160 passed, host_abi_tests.rs 789, largest split host_tcp_tests.rs 922
