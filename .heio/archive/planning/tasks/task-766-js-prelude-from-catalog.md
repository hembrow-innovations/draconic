---
id: "task-766-js-prelude-from-catalog"
title: "JS prelude reads the host catalog"
kind: task
status: completed
mode: afk
blocked_by: ["task-765-host-catalog-sync"]
sprint: "rust-dev-audit"
slice: "slice-757-host-catalog"
tags: []
created_at: "2026-09-09T17:30:00Z"
updated_at: "2026-09-09T19:00:00Z"
---

# JS prelude reads the host catalog

## Blocked by

[[task-765-host-catalog-sync]]: catalog_sync exists.

## Done

JS backend injects host polyfills from catalog names, not hand-rolled module_uses_* lists for those host functions. emit_js behaviour unchanged. cargo test -p draconic-backend-js is green.

## Context

backend-js lib.rs walks locals for readFileText and friends. Replace host name lists with catalog lookup. Keep sha256/stdlib walks if they are not HOST_APIS rows. Do not split emit.rs (that is [[task-733-js-split-emit]]).

## Verify

cargo test -p draconic-backend-js --offline. rg module_uses_fs_read in backend-js finds none, or it delegates to the catalog.

scope: crates/draconic-backend-js/src/lib.rs

## Links

[[slice-757-host-catalog]] [[slice-711-backend-js-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Host JS polyfill injection uses catalog names.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: one list of host names
- Contract-first: do not invent language behaviour

**Current behavior:**
module_uses_* duplicates host names.

**Desired behavior:**
Host prelude keys come from the catalog. Native-only names still hard-error on JS.

**Key interfaces:**
- emit_js / emit_js_full
- host_api lookup
- size-file-budget: do not grow lib.rs past a new 1250 breach; if over 1000, only replace lists

**Acceptance criteria:**
- [x] host polyfill injection uses catalog names
- [x] cargo test -p draconic-backend-js
- [x] do not edit emit.rs

## Gauntlet

- **round 1**: `cargo test -p draconic-backend-js --offline` — win — test result: ok. 65 passed. `module_uses_fs_read` gone; host prelude keys come from `host_apis()`.

**Out of scope:**
- Splitting emit.rs
- LLVM
- New host APIs
- Language behaviour or ROADMAP rows

**Explain this part:**
Stdlib crypto that is not a HOST_APIS row may keep its own module_uses_*.
