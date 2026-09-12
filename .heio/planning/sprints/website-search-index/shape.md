---
id: "website-search-index"
title: "Public site search index"
kind: sprint
status: active
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Public site search index

## Grouping

Location [[location-589-public-site]]. Body-term indexing for the in-site finder. Chrome polish stays in [[website-chrome-polish]]. Language ROADMAP.md loop is out.

## Slices in

- [[slice-894-search-body-terms]]: visitors find Learn and Reference pages by teaching-body terms, not only title or heading. blocked_by: none

## Slices out

- search keyboard and live region ([[slice-882-search-keyboard-live]])
- search session reset ([[slice-873-search-session-reset]])
- playground
- serving docs/ vault
- fuzz ranking
- language ROADMAP atoms

## Drain

`/afk-task` from [[task-895-search-body-terms]] (`status: ready`, `mode: afk`, empty `blocked_by`). Public site is `website/` TanStack Start, not `ui-components-web`.
