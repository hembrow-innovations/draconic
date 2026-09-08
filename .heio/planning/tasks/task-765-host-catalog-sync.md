---
id: "task-765-host-catalog-sync"
title: "Host catalog sync test"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-757-host-catalog"
tags: []
created_at: "2026-09-09T17:30:00Z"
updated_at: "2026-09-09T17:30:00Z"
---

# Host catalog sync test

## Blocked by

None.

## Done

A test named catalog_sync (or host_catalog_sync) fails if a HOST_APIS name with native availability lacks a matching Runtime ABI symbol, or a BOTH name lacks a JS polyfill export. cargo test -p draconic-check --offline catalog_sync is green today.

## Context

Expand. Do not move polyfill bodies yet. Checker stays owner of language names and availability. Runtime stays owner of C/LLVM shapes. The test is the seam so later tasks cannot drift.

## Verify

cargo test -p draconic-check --offline catalog_sync. Slice O1 EXPECT test result: ok.

scope: crates/draconic-check/src/host_api.rs, crates/draconic-runtime/src/abi.rs (read/assert only unless a missing row is a one-line AbiFn stub demanded by an existing name)

## Links

[[slice-757-host-catalog]]

## Agent Brief

**Category:** layout
**Summary:** Lock host names to ABI and JS polyfill rows.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: one catalog of host names; stop shotgun copies
- Contract-first: do not invent language behaviour

**Current behavior:**
HOST_APIS, abi.rs HOST_* , and JS polyfills can drift.

**Desired behavior:**
catalog_sync fails on drift. No new host function in this sitting.

**Key interfaces:**
- host_api::HOST_APIS / lookup
- draconic_runtime::abi HOST_SYMBOLS or AbiFn names
- size-file-budget: do not push host_api.rs over a new 1250 if already over; add the test next to existing host_api tests

**Acceptance criteria:**
- [ ] cargo test -p draconic-check --offline catalog_sync
- [ ] test asserts native/BOTH names have ABI symbols
- [ ] test asserts BOTH names have JS polyfill coverage
- [ ] no new workspace crate

**Out of scope:**
- Splitting host_api.rs (task-726 waits)
- Splitting abi.rs (task-740 waits)
- LLVM walker
- Language behaviour or ROADMAP rows

**Explain this part:**
If a known name is missing an ABI row, add the mapping test expectation only when the symbol already exists. Do not implement new host ops.
