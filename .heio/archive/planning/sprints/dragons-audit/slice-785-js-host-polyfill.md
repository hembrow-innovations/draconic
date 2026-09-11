---
id: "slice-785-js-host-polyfill"
title: "JS emit uses host_js_polyfill"
kind: slice
status: met
sprint: "dragons-audit"
blocked_by: []
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T12:10:00Z"
---

# JS emit uses host_js_polyfill

## Why

Runtime already maps catalog name to JS body. JS emit duplicates that with a 24-body array and a string contains match, and re-reads Check availability.

## Done

`emit_js` injects host polyfills only through `draconic_runtime::host_js_polyfill`. `host_js_polyfill_bodies` is gone. JS backend tests stay green.

## Blocked by

None.

## Non-goals

- **rewriting draconic_rt_host.c**
- **deny-by-default permissions**
- **LLVM host adapters** (that is [[slice-784-llvm-no-fingerprint]])

## Oracle checklist

- [x] O1: no host_js_polyfill_bodies
  CHECK: python3 -c 'from pathlib import Path; t=Path("crates/draconic-backend-js/src/es_polyfill.rs").read_text(); print("adapter" if "fn host_js_polyfill_bodies" not in t and "host_js_polyfill(" in t else "still-dup")'
  EXPECT: adapter
  EVIDENCE: adapter (slice re-run after task-795)
- [x] O2: js backend tests green
  CHECK: cargo test -p draconic-backend-js --offline
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 65 passed (slice re-run after task-795)

## Pool

- [[task-795-js-polyfill-adapter]]

## See also

[[ticket-778-js-host-polyfill-unused]], [[slice-757-host-catalog]], crates/draconic-runtime/src/host_js_polyfill.rs, crates/draconic-backend-js/src/es_polyfill.rs
