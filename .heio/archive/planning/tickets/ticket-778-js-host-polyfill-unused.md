---
id: "ticket-778-js-host-polyfill-unused"
title: "JS emit ignores host_js_polyfill"
kind: ticket
status: closed
ticket_type: observation
tags: []
blocked_by: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T06:57:16Z"
---
# JS emit ignores host_js_polyfill

## Signal

Dragons audit 2026-09-10. Runtime `host_js_polyfill(name)` maps catalog names to JS bodies. `draconic-backend-js` `es_polyfill.rs` rebuilds a 24-body array and matches with `contains("function {name}(")`. It also walks `draconic_check::host_apis()` for `availability.js`. LLVM `host_fs` peeks `lookup_host_api` to classify callees.

## Fit

Promoted to [[slice-785-js-host-polyfill]].

## Notes

- Check owns availability. Runtime owns JS bodies. Backends should consume IR plus that adapter.
- [[slice-757-host-catalog]] is met. The JS emit path did not switch to the name lookup.

## Parent

[[slice-785-js-host-polyfill]]
