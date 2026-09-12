---
id: "ticket-924-git-pkg-js-library-split"
title: "Git packages and JS --library are two unbridged library products"
kind: ticket
status: open
ticket_type: observation
tags: [packages, js, distribution]
blocked_by: []
created_at: "2026-09-13T12:00:00Z"
updated_at: "2026-09-13T12:00:00Z"
---

# Git packages and JS --library are two unbridged library products

## Signal

`examples/pkg-lib` is a git source checkout consumed by the Linker. `draconic build --target js --library` is a flattened Node ESM file. Draconic cannot import that artifact. Node cannot import pkg-lib without `--library`. There is no tested path that builds `examples/pkg-lib/index.drac` with `--library`, and no path that feeds a `--library` file back into the Linker. Public packages pages never mention `--library`.

## Fit

Unknown until triage. Named `--library` emit already landed ([[ticket-908-js-library-esm-export]] closed). This is the missing bridge and authoring story, not default-build always-ESM ([[ticket-913-default-js-build-no-esm]] parked) and not default/star remainder ([[ticket-914-js-library-default-export]], [[ticket-915-js-library-export-star]]).

## Notes

- K10 tests compile a consumer with `emit_js`, not `emit_js_library`.
- `--watch` passes `library` through but `watch.rs` polls only the entry file mtime, not the flattened graph.
- Default output stays `{stem}.out.js` even with `--library`. CLI tests force `-o lib.mjs`.
- No package-level js/native/portable or host-I/O contract. Native types and host I/O in a library inherit the consumer compile.
- Libraries do not ship a type surface to Node. Git consumers get types only because Linker flattens source.

## Parent

[[location-220-packages]] [[location-224-distribution]]

## What to build

An author can tell git-tag source packages from Node `--library` artifacts, and has one documented path that produces each.

## Blocked by

none
