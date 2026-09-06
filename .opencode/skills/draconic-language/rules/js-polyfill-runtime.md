---
title: JS polyfills live in Runtime
impact: HIGH
impactDescription: Duplicated polyfill strings drift from the native ABI
tags: [js, runtime, polyfill]
---

## JS polyfills live in Runtime

Node bridges for host names are strings exported from `draconic-runtime`. The JS backend prepends them when IR references those names.

**Incorrect:** a second copy of `sha256` / TCP helpers inside `draconic-backend-js`.

**Correct:** `draconic_runtime::sha256_js_polyfill()` (and the matching `tcp_js_polyfill`, …). Prepend from the backend; keep the text next to the C ABI.

**Notes.** See `dual-host-globals`. Native path links `libdraconic_rt.a`.
