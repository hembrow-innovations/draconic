---
id: "task-792-fold-host-catalog-emit"
title: "One catalog-driven LLVM host emit"
kind: task
status: completed
mode: afk
blocked_by: [ "task-791-emitter-slotty-migrate" ]
sprint: "dragons-audit"
slice: "slice-784-llvm-no-fingerprint"
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T22:40:00Z"
---
# One catalog-driven LLVM host emit

## Blocked by

[[task-791-emitter-slotty-migrate]]: shared Emitter first.

## Done

Host call names classify through `lookup_host_api` (or the catalog already in Check), not a new `walk_host_*` per domain. Existing host Conformance that already passes native still passes. No new host fingerprint adapter.

## Context

~25 `walk_host_*` functions each fingerprint a module then emit a full `@main`. `host_fs` already peeks the catalog. Collapse host name classification to that catalog. Keep Runtime C ABI calls. Do not delete try_folded_walks in this sitting if ES adapters still need it; host arms should not grow.

If one sitting cannot fold every host_* file, fold classification (is_named_callee lists) into one host module and stop. File a ticket for leftover emit bodies. Do not add adapters.

## Verify

`cargo test -p draconic-backend-llvm --offline` ok. No new `fn walk_host_`.

scope: crates/draconic-backend-llvm/src/host_*.rs, es_expr.rs try_folded_walks host arms if they shrink

## Links

[[slice-784-llvm-no-fingerprint]] [[ticket-776-llvm-fingerprint-adapters]] [[slice-757-host-catalog]]

## Agent Brief

**Category:** layout
**Summary:** Host LLVM names come from the catalog, not per-file fingerprints.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless a Diagnostic message changes
- Purpose: one host classification
- Contract-first: do not invent host APIs

**Current behavior:**
Each host domain has walk_host_* plus is_named_callee lists.

**Desired behavior:**
Catalog-driven host names. Native host fixtures that already pass stay green.

**Key interfaces:**
- draconic_check::lookup_host_api / host_apis
- Runtime ABI declare/call
- size-file-budget: no new file over 1000

**Acceptance criteria:**
- [x] cargo test -p draconic-backend-llvm
- [x] no new walk_host_* 
- [x] host Conformance that was green stays green

Classification is shared in `host_catalog.rs` via `lookup_host_api`. Emit bodies stay per-domain; leftover is [[ticket-802-llvm-host-emit-bodies]].

## Gauntlet

- **round**: 1
- **command**: cargo test -p draconic-backend-llvm --offline
- **result**: win (318 passed)
- **gap**: none. Host fs/process/stdio/path/tcp Conformance still ok.

**Out of scope:**
- deleting try_folded_walks (that is [[task-793-delete-try-folded-walks]])
- annex-b private methods
- inkwell

**Explain this part:**
If a host emit body cannot move in one sitting, share classification only and ticket the rest. Do not leave a new fingerprint.
