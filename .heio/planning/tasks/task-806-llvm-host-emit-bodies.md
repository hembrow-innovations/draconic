---
id: "task-806-llvm-host-emit-bodies"
title: "LLVM host emit bodies"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "platform"
slice: "slice-805-llvm-host-emit-bodies"
tags: []
created_at: "2026-09-11T23:30:00Z"
updated_at: "2026-09-11T23:30:00Z"
---

# LLVM host emit bodies

## Blocked by

None.

## Done

`walk_host_*` fingerprint adapters are gone or no longer emit a full `@main`. Host calls go through `lookup_host_api` on the one IR walker. Native host Conformance that was already green stays green.

## Context

Classification already lives in `host_catalog` via `lookup_host_api`. Slice [[slice-784-llvm-no-fingerprint]] is met. Leftover `walk_host_*` emit bodies remain in the llvm backend. Frame-time search found `walk_host_*` definitions and no call sites in that crate; confirm dead leftovers versus still dispatched. Prefer deleting leftover fingerprint emit bodies once the walker covers host calls via the catalog. Do not add a new host fingerprint adapter. Keep Runtime C ABI calls. Do not absorb native `console.log` ([[ticket-773-native-console-log]]).

## Verify

Slice O1, O2, and O3.

scope: draconic-backend-llvm host_* modules, host_catalog, IR walker

## Links

[[slice-805-llvm-host-emit-bodies]] [[ticket-802-llvm-host-emit-bodies]] [[task-792-fold-host-catalog-emit]] [[ticket-773-native-console-log]]

## Agent Brief

**Category:** layout/refactor
**Summary:** Leftover `walk_host_*` fingerprint emit bodies leave the llvm crate; host calls stay catalog-driven on the one IR walker.

**Intent (required when product behaviour changes):**
- **No product behaviour change** unless a Diagnostic message must change
- Promise ids: none unless a Diagnostic message changes
- Purpose: leftover host emit on the one walker, not a new fingerprint
- Contract-first: do not invent host APIs

**Current behavior:**
Host names classify through `lookup_host_api`. About twenty-five `walk_host_*` functions still fingerprint a module and emit a full `@main`. Definitions were found in host_* modules with no call sites in draconic-backend-llvm at planning time.

**Desired behavior:**
Confirm whether those functions are dead leftovers or still dispatched. Once the one IR walker covers host calls via the catalog, delete leftover fingerprint emit bodies. No new `walk_host_*`. Runtime C ABI declare and call stays. Native host Conformance that already passes stays green.

**Key interfaces:**
- `lookup_host_api` / `host_catalog`
- Runtime C ABI declare and call
- `emit_llvm_ir` and the one IR walker
- size-file-budget: no new file over 1000

**Acceptance criteria:**
- [ ] Slice O1 prints `no-host-fingerprint`
- [ ] `cargo test -p draconic-backend-llvm --offline` prints `test result: ok.`
- [ ] Existing native host fixtures that were green stay green (slice O3)
- [ ] No new `walk_host_*`
- [ ] No new host fingerprint adapter

**Out of scope:**
- native `console.log` ([[ticket-773-native-console-log]])
- inkwell
- new host APIs
- splitting adapters only to meet 1000 lines
- hello-stub success for unsupported IR
- annex-b private methods (already restored on [[slice-784-llvm-no-fingerprint]])
- marking E17.02 or E18.44 done
