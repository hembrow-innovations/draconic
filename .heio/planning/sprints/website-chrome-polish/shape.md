---
id: "website-chrome-polish"
title: "Public site chrome polish"
kind: sprint
status: active
tags: [website, public-site]
created_at: "2026-09-12T07:05:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Public site chrome polish

## Grouping

Location [[location-589-public-site]]. Vertical cuts from the 2026-09-12 website swarm plus the 2026-09-12 triage promotion: favicon, GitHub reachability, current-page contrast, distinct chapter names, per-page titles, search session, unknown-URL recovery, search keyboard, fence copy announcement, Reference footer, shipped-badge contrast, share meta, and outline current section. One thin slice per sitting. Language ROADMAP.md loop is out.

## Slices in

- [[slice-863-favicon]]: origin serves a site icon. blocked_by: none
- [[slice-865-github-above-fold]]: GitHub stays in the sticky aside's visible box at desktop height. blocked_by: none
- [[slice-867-current-page-contrast]]: current-page ink meets body-text contrast on canvas. blocked_by: none
- [[slice-869-distinct-nav-labels]]: Learn and Reference chapter names are distinguishable without renaming CONTEXT terms. blocked_by: none
- [[slice-871-per-page-titles]]: tabs and history name the open page. blocked_by: none
- [[slice-873-search-session-reset]]: search query and results clear on navigate and Escape. blocked_by: none
- [[slice-875-github-mobile-hit]]: small-viewport GitHub is a compact control. blocked_by: [[slice-865-github-above-fold]]
- [[slice-880-empty-404]]: unknown URLs keep chrome with a heading and a way back. blocked_by: none
- [[slice-882-search-keyboard-live]]: search hits are keyboard-reachable and announced. blocked_by: none
- [[slice-884-copy-announcement]]: fence Copy controls are distinct and announced. blocked_by: none
- [[slice-886-reference-related-footer]]: Reference working pages have a related-link footer. blocked_by: none
- [[slice-888-shipped-badge-contrast]]: shipped chip meets light-theme contrast. blocked_by: none
- [[slice-890-meta-description]]: pages expose a page-specific share summary. blocked_by: none
- [[slice-892-outline-current-section]]: On this page marks the heading in view. blocked_by: none

## Slices out

- playground
- in-page runners
- serving docs/ vault
- replacing TanStack Start
- Next.js/VitePress/Starlight/mdBook
- rewriting CONTEXT.md Learn/Reference terms
- indexing body-only search terms ([[website-search-index]] / [[slice-894-search-body-terms]])
- language ROADMAP atoms

## Drain

`/afk-task` from unblocked chrome follow-ups. First runnable tasks are [[task-864-favicon]], [[task-866-github-above-fold]], [[task-868-current-page-contrast]], [[task-870-distinct-nav-labels]], [[task-872-per-page-titles]], [[task-874-search-session-reset]], [[task-881-empty-404]], [[task-883-search-keyboard-live]], [[task-885-copy-announcement]], [[task-887-reference-related-footer]], [[task-889-shipped-badge-contrast]], [[task-891-meta-description]], and [[task-893-outline-current-section]] (`status: ready`, `mode: afk`, empty `blocked_by`). [[task-879-github-mobile-hit]] stays ready but drain must honor `blocked_by`. Public site is `website/` TanStack Start, not `ui-components-web`.
