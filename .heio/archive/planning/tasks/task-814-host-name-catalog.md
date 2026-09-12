---
id: "task-814-host-name-catalog"
title: "LLVM walker host names from catalog"
kind: task
status: completed
mode: afk
blocked_by: ["task-812-llvm-walk-file-budget"]
sprint: "platform"
slice: "slice-813-host-name-catalog"
tags: []
created_at: "2026-09-12T00:15:00Z"
updated_at: "2026-09-12T19:10:00Z"
---

# LLVM walker host names from catalog

## Blocked by

[[task-812-llvm-walk-file-budget]]: walker must already be under file budget.

## Done

Walker host dispatch uses catalog lookup, not a parallel host-name string cascade. Native host Conformance that was already green stays green.

## Context

`lookup_host_api` already classifies host callee names. The walker still steal-routes with `host_has` string lists. Check `HOST_APIS` stays the language name owner. JS polyfill bodies stay keyed by catalog names. Do not absorb Date.now / stdlib emit.

## Verify

Slice O1, O2, O3, and O4.

scope: draconic-backend-llvm walker host dispatch and host_catalog; read-only Check catalog

## Links

[[slice-813-host-name-catalog]] [[ticket-810-host-name-shotgun]] [[task-812-llvm-walk-file-budget]] [[task-792-fold-host-catalog-emit]]

## Agent Brief

**Category:** layout/refactor
**Summary:** LLVM walker host steal-routes go through `lookup_host_api`, not a second name table.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- **No product behaviour change** unless a Diagnostic message must change
- Promise ids: none unless a Diagnostic message changes
- Purpose: one catalog of host names; stop shotgun copies
- Contract-first: do not invent host APIs

**Current behavior:**
Host names classify through `lookup_host_api`. The walker still decides which `emit_host_*` to call by matching collected host names against literal string lists (`host_has(&[...])`). Adding a host API means editing Check and those lists.

**Desired behavior:**
Walker host dispatch uses catalog lookup (and existing `emit_host_*` modules). The `host_has(&[` string-list cascade is gone. Check remains owner of `HOST_APIS`. `catalog_sync` stays green. JS polyfill bodies are not rewritten this sitting. Native host fixtures that already pass stay green. File budget still holds.

**Key interfaces:**
- `lookup_host_api` / `host_apis` / `HostApiEntry`
- `catalog_callee` / walker host dispatch / existing `emit_host_*`
- size-file-budget: no file over 1000

**Acceptance criteria:**
- [x] Slice O1 prints `catalog-route`
- [x] `cargo test -p draconic-backend-llvm --offline` prints `test result: ok.`
- [x] `cargo test -p draconic-check --offline catalog_sync` prints `test result: ok.`
- [x] Existing native host fixtures stay green (slice O4)
- [x] No new host API
- [x] No new `walk_host_*` fingerprint adapter

## Gauntlet

- **round**: 1
- **command**: O1 catalog-route scan; cargo test -p draconic-backend-llvm --offline; cargo test -p draconic-check --offline catalog_sync; host_fs/process/stdio/path/tcp conformance
- **result**: win
- **gap**: none. Dispatch uses catalog note prefixes via lookup_host_api. O1 catalog-route. O2 326 passed. O3 catalog_sync ok. O4 host fixtures ok.

**Out of scope:**
- native Date / flags / url / compression emit ([[ticket-807-llvm-walker-native-date-stdlib]])
- rewriting JS polyfill bodies
- inkwell
- hello-stub success for unsupported IR
- marking E17.02 or E18.44 done
