---
id: "ticket-810-host-name-shotgun"
title: "Host names are listed in four places"
kind: ticket
status: open
ticket_type: observation
tags: []
blocked_by: []
created_at: "2026-09-11T21:47:50Z"
updated_at: "2026-09-11T21:47:50Z"
---

# Host names are listed in four places

## Signal

Dragons audit 2026-09-12. Adding a host API still means editing Check `HOST_APIS`, LLVM `walk.rs` name arrays (`crates/draconic-backend-llvm/src/es_expr/walk.rs:73`–`:303`), JS `host_js_polyfill` (`crates/draconic-runtime/src/host_js_polyfill.rs:15`), and a per-host classify arm. `host_catalog.rs` looks up Check names only. Closed [[ticket-778-js-host-polyfill-unused]] removed a duplicate JS body array. The name lists remain.

Not the Date.now steal. That is [[ticket-807-llvm-walker-native-date-stdlib]].

## Fit

this project, later slice

## Notes

- Catalog lookup: `crates/draconic-backend-llvm/src/host_catalog.rs`.
- JS skip on missing polyfill: `crates/draconic-backend-js/src/es_polyfill.rs:107`.
- Do not mark E17.02 or E18.44 done.
