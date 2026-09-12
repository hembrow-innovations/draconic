---
id: "js-library-esm"
title: "JS library ESM emit"
kind: sprint
status: active
tags: [js, linker, distribution]
created_at: "2026-09-12T11:05:00Z"
updated_at: "2026-09-12T18:20:00Z"
---

# JS library ESM emit

## Grouping

Location [[location-224-distribution]]. Promoted from [[ticket-908-js-library-esm-export]] then [[ticket-914-js-library-default-export]] and [[ticket-915-js-library-export-star]]. Named exports, default export, and `export *` on flattened JS emit so Node can import a library artifact. Default `draconic run` stays a script. Language ROADMAP.md loop is out.

## Slices in

- [[slice-909-entry-export-table]]: entry named-export names survive flatten as IR metadata. blocked_by: none. met
- [[slice-911-js-library-named-esm]]: opt-in JS library emit exposes those names to Node. blocked_by: [[slice-909-entry-export-table]]. met
- [[slice-916-js-library-default-export]]: opt-in JS library emit exposes authored `export default`. blocked_by: none
- [[slice-918-js-library-export-star]]: opt-in JS library emit exposes `export *` names. blocked_by: [[slice-916-js-library-default-export]]

## Slices out

- always-ESM / changing `draconic run` to `node --input-type=module` ([[ticket-913-default-js-build-no-esm]] parked)
- keeping `export` statements through check and lower
- multi-file ESM emit without flatten
- npm as the package story ([[0009-go-style-git-packages]], [[location-220-packages]])
- consumer copy or hand-wrap of JS emit
- website chrome
- language ROADMAP atoms

## Drain

AFK. First claim is [[task-917-js-library-default-export]]. [[task-919-js-library-export-star]] waits on it.
