---
id: "task-726-check-split-host-api"
title: "Split check host_api.rs"
kind: task
status: completed
mode: afk
blocked_by: ["task-765-host-catalog-sync"]
sprint: "rust-dev-audit"
slice: "slice-709-check-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T23:30:00Z"
---

# Split check host_api.rs

## Blocked by

[[task-765-host-catalog-sync]]: catalog_sync before splitting host_api.rs.

## Done

host_api.rs ≤1000. HOST_APIS and tests split like LLVM host_* files if needed.

## Context

host_api.rs 1775; product through about 881, tests from 882. Split by host groups. Keep mod host_api from lib.rs.

## Verify

cargo test -p draconic-check. No host_api*.rs over 1000.

scope: crates/draconic-check/src/host_api.rs and new host_api_*.rs or host_*.rs under check

## Links

[[slice-709-check-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split checker host_api.rs under 1000.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
One 1775-line host_api.rs.

**Desired behavior:**
Files ≤1000.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] no host api rust file over 1000
- [x] cargo test -p draconic-check

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Editing Binder/Checker in lib.rs

**Explain this part:**
Parallel with binder extract. Do not touch lib.rs Binder.

## Gauntlet

- **round 1**: `cargo test -p draconic-check --offline catalog_sync` — lose — catalog_sync still scanned only abi.rs after the runtime ABI/polyfill split.
- **round 2**: `cargo test -p draconic-check --offline` — win — test result: ok. 278 passed including catalog_sync. host_api*.rs all ≤491.
