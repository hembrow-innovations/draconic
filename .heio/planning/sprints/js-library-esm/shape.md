---
id: "js-library-esm"
title: "JS library ESM emit"
kind: sprint
status: active
tags: [js, linker, distribution]
created_at: "2026-09-12T11:05:00Z"
updated_at: "2026-09-12T11:05:00Z"
---

# JS library ESM emit

## Grouping

Location [[location-224-distribution]]. Promoted from [[ticket-908-js-library-esm-export]]. Named exports on flattened JS emit so Node can `import { name } from` a library artifact. Default `draconic run` stays a script. Language ROADMAP.md loop is out.

## Slices in

- [[slice-909-entry-export-table]]: entry named-export names survive flatten as IR metadata. blocked_by: none
- [[slice-911-js-library-named-esm]]: opt-in JS library emit exposes those names to Node. blocked_by: [[slice-909-entry-export-table]]

## Slices out

- default export and `export *`
- always-ESM / changing `draconic run` to `node --input-type=module`
- keeping `export` statements through check and lower
- multi-file ESM emit without flatten
- npm as the package story ([[0009-go-style-git-packages]], [[location-220-packages]])
- consumer copy or hand-wrap of JS emit
- website chrome
- language ROADMAP atoms

## Drain

AFK. First claim is [[task-910-entry-export-table]]. [[task-912-js-library-named-esm]] waits on it.
