---
id: "task-812-llvm-walk-file-budget"
title: "LLVM walk file budget"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "platform"
slice: "slice-811-llvm-walk-file-budget"
tags: []
created_at: "2026-09-12T00:15:00Z"
updated_at: "2026-09-12T14:00:00Z"
---

# LLVM walk file budget

## Blocked by

None.

## Done

Every `draconic-backend-llvm` Rust file is at or under 1000 lines. Walker emit selection is unchanged. Crate tests stay green.

## Context

The IR walker file is 1075 lines. AGENTS.md target is 1000, hard limit 1250. Prior file-budget work split classify/ok siblings. This sitting is size only. Do not change Date/stdlib native emit. Do not fold host names into the catalog.

## Verify

Slice O1 and O2.

scope: draconic-backend-llvm IR walker modules only

## Links

[[slice-811-llvm-walk-file-budget]] [[ticket-809-llvm-walk-over-file-budget]] [[ticket-807-llvm-walker-native-date-stdlib]] [[ticket-810-host-name-shotgun]]

## Agent Brief

**Category:** layout/refactor
**Summary:** Bring the LLVM IR walker back under the 1000-line file-budget target without changing emit selection.

**Intent (required when product behaviour changes):**
- **No product behaviour change**
- Promise ids: none
- Purpose: size-file-budget on the one IR walker
- Contract-first: do not invent emit routes

**Current behavior:**
The walker that classifies IR and steal-routes host versus ES emit lives in one file over 1000 lines. Crate tests pass. Native Date and stdlib Conformance that fail today still fail after this task.

**Desired behavior:**
Every Rust file in `draconic-backend-llvm` is ≤1000 lines. Hard limit 1250. Split along existing seams (host dispatch versus ES kind versus walk/collect), not a second fingerprint adapter. Public `emit_llvm_ir` behavior is unchanged. No new workspace crate.

**Key interfaces:**
- `emit_walk` / Seen / `es_kind` / host steal-route
- size-file-budget: no new file over 1000; prove fails over 1200
- tests stay next to the modules they lock

**Acceptance criteria:**
- [x] Slice O1 prints `max-loc-ok`
- [x] `cargo test -p draconic-backend-llvm --offline` prints `test result: ok.`
- [x] No new `walk_host_*` fingerprint adapter
- [x] No hello-stub success for unsupported IR

## Gauntlet

- **round**: 1
- **command**: O1 python loc scan; cargo test -p draconic-backend-llvm --offline
- **result**: win
- **gap**: none. walk.rs split into walk collect, host_dispatch, and es_kind. O1 max-loc-ok. O2 325 passed.

**Out of scope:**
- native Date / flags / url / compression emit ([[ticket-807-llvm-walker-native-date-stdlib]])
- replacing walker host name lists with catalog lookup ([[ticket-810-host-name-shotgun]])
- inkwell
- marking E17.02 or E18.44 done
