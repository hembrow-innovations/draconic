---
id: "task-768-abi-polyfills-keyed"
title: "Key Runtime JS polyfills by catalog name"
kind: task
status: ready
mode: afk
blocked_by: ["task-765-host-catalog-sync"]
sprint: "rust-dev-audit"
slice: "slice-757-host-catalog"
tags: []
created_at: "2026-09-09T17:30:00Z"
updated_at: "2026-09-09T17:30:00Z"
---

# Key Runtime JS polyfills by catalog name

## Blocked by

[[task-765-host-catalog-sync]]: catalog_sync exists.

## Done

JS polyfill functions for host names are reachable by catalog name (map or match on the same strings HOST_APIS uses). abi.rs may still be over 1000. cargo test -p draconic-runtime is green. catalog_sync still passes.

## Context

Prefactor [[task-740-runtime-split-abi]]. Split later by catalog groups, not by inventing a second name list. Do not rewrite C. Do not `pub use abi::*` new dump types.

## Verify

cargo test -p draconic-runtime --offline. cargo test -p draconic-check --offline catalog_sync.

scope: crates/draconic-runtime/src/abi.rs and existing polyfill modules it already calls

## Links

[[slice-757-host-catalog]] [[task-740-runtime-split-abi]]

## Agent Brief

**Category:** layout
**Summary:** Polyfills keyed by host catalog names before abi.rs split.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: abi split cuts one table
- Contract-first: do not invent language behaviour

**Current behavior:**
Polyfill fns are ad-hoc names in abi.rs.

**Desired behavior:**
A caller can ask for the polyfill body by catalog host name.

**Key interfaces:**
- existing fs_read_js_polyfill and siblings, plus a name-keyed lookup
- size-file-budget: do not grow abi.rs; prefer moving polyfills into siblings if that keeps files ≤1000, but finishing the 1000 budget is task-740

**Acceptance criteria:**
- [ ] lookup by catalog name returns host JS polyfills
- [ ] cargo test -p draconic-runtime
- [ ] catalog_sync still green
- [ ] no new workspace crate

**Out of scope:**
- Finishing abi.rs ≤1000 (task-740)
- C host implementation
- Language behaviour or ROADMAP rows

**Explain this part:**
If you must move polyfill bodies to new files to avoid growing abi.rs, that is allowed and helps task-740. Do not change LLVM declare strings in this sitting unless a move requires a re-export.
