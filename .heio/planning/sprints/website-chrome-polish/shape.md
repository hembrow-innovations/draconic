---
id: "website-chrome-polish"
title: "Public site chrome polish"
kind: sprint
status: active
tags: [website, public-site]
created_at: "2026-09-12T07:05:00Z"
updated_at: "2026-09-12T07:05:00Z"
---

# Public site chrome polish

## Grouping

Location [[location-589-public-site]]. Vertical cuts from the 2026-09-12 website swarm: favicon, GitHub reachability, current-page contrast, distinct chapter names, per-page titles, and search session. One thin slice per sitting. Language ROADMAP.md loop is out.

## Slices in

- [[slice-863-favicon]]: origin serves a site icon. blocked_by: none
- [[slice-865-github-above-fold]]: GitHub stays in the sticky aside's visible box at desktop height. blocked_by: none
- [[slice-867-current-page-contrast]]: current-page ink meets body-text contrast on canvas. blocked_by: none
- [[slice-869-distinct-nav-labels]]: Learn and Reference chapter names are distinguishable without renaming CONTEXT terms. blocked_by: none
- [[slice-871-per-page-titles]]: tabs and history name the open page. blocked_by: none
- [[slice-873-search-session-reset]]: search query and results clear on navigate and Escape. blocked_by: none
- [[slice-875-github-mobile-hit]]: small-viewport GitHub is a compact control. blocked_by: [[slice-865-github-above-fold]]

## Slices out

- playground
- in-page runners
- serving docs/ vault
- replacing TanStack Start
- Next.js/VitePress/Starlight/mdBook
- rewriting CONTEXT.md Learn/Reference terms
- indexing body-only search terms ([[ticket-821-search-body-terms]])
- language ROADMAP atoms

## Drain

`/afk-task` from unblocked chrome follow-ups. First runnable tasks are [[task-864-favicon]], [[task-866-github-above-fold]], [[task-868-current-page-contrast]], [[task-870-distinct-nav-labels]], [[task-872-per-page-titles]], and [[task-874-search-session-reset]] (`status: ready`, `mode: afk`, empty `blocked_by`). [[task-879-github-mobile-hit]] stays ready but drain must honor `blocked_by`. Public site is `website/` TanStack Start, not `ui-components-web`.
