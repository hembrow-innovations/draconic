---
id: "task-795-js-polyfill-adapter"
title: "JS emit calls host_js_polyfill"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "dragons-audit"
slice: "slice-785-js-host-polyfill"
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T12:00:00Z"
---

# JS emit calls host_js_polyfill

## Blocked by

None.

## Done

`es_polyfill.rs` injects bodies via `draconic_runtime::host_js_polyfill`. `host_js_polyfill_bodies` is gone. Slice O1 and O2 hold.

## Context

Runtime already groups catalog names onto shared JS bodies. JS emit should look up used ident names through that function. After `compile_path_for_target`, do not re-decide `availability.js` in the backend if Check already rejected native-only names. Keep native.rs hard-error for leftover native-only IR if tests require it; do not duplicate the 24-body array.

Do not walk the IR twice if one walk already exists; reuse.

## Verify

Slice O1 and O2.

scope: crates/draconic-backend-js/src/es_polyfill.rs, native.rs only if catalog import becomes unused

## Links

[[slice-785-js-host-polyfill]] [[ticket-778-js-host-polyfill-unused]]

## Agent Brief

**Category:** layout
**Summary:** Delete the JS polyfill duplicate. Use host_js_polyfill.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none
- Purpose: one host JS body adapter
- Contract-first: do not invent host APIs

**Current behavior:**
24-body array plus contains match plus host_apis filter.

**Desired behavior:**
host_js_polyfill(name). backend-js tests green.

**Key interfaces:**
- draconic_runtime::host_js_polyfill
- emit_js
- size-file-budget: no new file over 1000

**Acceptance criteria:**
- [x] slice O1 adapter
- [x] cargo test -p draconic-backend-js
- [x] no host_js_polyfill_bodies

**Out of scope:**
- LLVM host adapters
- rewriting C host
- permissions

**Explain this part:**
String contains on function source is the bug. Name lookup is the interface.

## Gauntlet

- **round**: 1
- **command**: O1 python adapter check; cargo test -p draconic-backend-js --offline
- **result**: win (O1 adapter; test result: ok. 65 passed)
- **gap**: none. `host_js_polyfill_bodies` gone. Bodies come from `host_js_polyfill(name)`. `availability.js` is not re-read. native.rs still hard-errors leftover native-only IR.
