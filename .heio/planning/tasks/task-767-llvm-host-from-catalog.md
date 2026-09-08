---
id: "task-767-llvm-host-from-catalog"
title: "LLVM host names read the catalog"
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

# LLVM host names read the catalog

## Blocked by

[[task-765-host-catalog-sync]]: catalog_sync exists.

## Done

host_fs.rs (and only fs in this sitting) classifies host call names via the catalog, not a copied string list. Dispatch still has is_host_fs_module until [[task-764-llvm-fold-host-fs]]. cargo test -p draconic-backend-llvm is green.

## Context

Stop teaching each host_* file a private name table. This sitting: host_fs only, so [[task-764-llvm-fold-host-fs]] can emit ABI calls from catalog rows. Do not fold the adapter yet.

## Verify

cargo test -p draconic-backend-llvm --offline. host_fs classify uses catalog/lookup, not a unique "readFileText" string list that can drift from HOST_APIS.

scope: crates/draconic-backend-llvm/src/host_fs.rs

## Links

[[slice-757-host-catalog]] [[task-764-llvm-fold-host-fs]]

## Agent Brief

**Category:** layout
**Summary:** host_fs name lists come from the catalog.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: one list of host names
- Contract-first: do not invent language behaviour

**Current behavior:**
host_fs.rs copies host identifier strings.

**Desired behavior:**
Those names resolve through the catalog. Classifier still exists.

**Key interfaces:**
- host_api lookup / catalog
- is_host_fs_module still present
- size-file-budget: do not add files over 1000

**Acceptance criteria:**
- [ ] host_fs host identifiers come from the catalog
- [ ] is_host_fs_module still in emit_llvm_ir_raw
- [ ] cargo test -p draconic-backend-llvm
- [ ] do not edit other host_* files

**Out of scope:**
- Folding host_fs into the walker
- Other host_* adapters
- Language behaviour or ROADMAP rows

**Explain this part:**
Do not add a new is_* adapter. Do not split host_fs.rs here (slice-712 waits on walker contract).
