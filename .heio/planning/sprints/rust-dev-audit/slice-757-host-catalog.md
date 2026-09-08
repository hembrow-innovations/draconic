---
id: "slice-757-host-catalog"
title: "Host catalog one table"
kind: slice
status: frozen
sprint: "rust-dev-audit"
blocked_by: []
tags: []
created_at: "2026-09-09T17:30:00Z"
updated_at: "2026-09-09T17:30:00Z"
---

# Host catalog one table

## Why

Dragons audit: adding one host function is shotgun surgery across Checker names, C ABI, LLVM declares, JS polyfills, and LLVM classifiers. Prefactor [[slice-714-runtime-file-budget]] abi split and [[slice-709-check-file-budget]] host_api split so those cuts split one table, not five copies.

## Done

Language host names and availability live in one catalog. A sync test fails if a catalog name lacks the Runtime ABI row and JS polyfill its availability requires. JS prelude injection and LLVM host name lists read that catalog. Frontend compile/check can pass CompileTarget. cargo test -p draconic-check, draconic-runtime, draconic-backend-js, and draconic-frontend are green.

## Blocked by

None. [[task-740-runtime-split-abi]] and [[task-726-check-split-host-api]] wait on this slice's tasks.

## Non-goals

- **a new workspace crate**
- **rewriting draconic_rt_host.c**
- **deny-by-default permissions**
- **LLVM walker** (that is [[slice-756-llvm-one-walker]])

## Oracle checklist

- [ ] O1: catalog sync test green
  CHECK: cargo test -p draconic-check --offline catalog_sync
  EXPECT: test result: ok.
  EVIDENCE: pending
- [ ] O2: check runtime js frontend tests green
  CHECK: cargo test -p draconic-check -p draconic-runtime -p draconic-backend-js -p draconic-frontend --offline
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

Durable links to task ids. Never drop them.

- [[task-765-host-catalog-sync]]
- [[task-766-js-prelude-from-catalog]]
- [[task-767-llvm-host-from-catalog]]
- [[task-768-abi-polyfills-keyed]]
- [[task-769-frontend-compile-target]]

## See also

[[slice-709-check-file-budget]], [[slice-714-runtime-file-budget]], [[slice-711-backend-js-file-budget]], ADR-0008, crates/draconic-check/src/host_api.rs, crates/draconic-runtime/src/abi.rs
