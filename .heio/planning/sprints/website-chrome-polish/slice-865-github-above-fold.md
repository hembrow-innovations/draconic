---
id: "slice-865-github-above-fold"
title: "GitHub above the nav fold"
kind: slice
status: met
sprint: "website-chrome-polish"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-12T07:05:00Z"
updated_at: "2026-09-12T17:32:00Z"
---

# GitHub above the nav fold

## Why

Contract chrome includes GitHub in the sticky side nav, but long always-expanded Learn and Reference lists push it below a 1280 by 720 desktop fold. Search and theme already sit above those lists. A visitor can miss the repo link.

## Done

At a 1280 by 720 desktop viewport, the GitHub control sits inside the sticky aside's visible box without scrolling that aside. It stays in the side nav with the same repo href.

## Blocked by

None.

## Non-goals

- **Tall mobile hit area**: [[slice-875-github-mobile-hit]]
- **Dropping GitHub, search, or the theme toggle**
- **Playground, vault-as-site, Start replacement**

## Oracle checklist

- [x] O1: GitHub is not ordered after the chapter lists in the sticky side nav
  CHECK: pnpm --dir website exec vitest run site-header-primary-nav
  EXPECT: Test Files  1 passed
  EVIDENCE: 2026-09-12 `pnpm --dir website exec vitest run site-header-primary-nav` → Test Files  1 passed (1)

## Pool

- [[task-866-github-above-fold]]

## See also

[[ticket-848-github-below-nav-fold]] [[ticket-646-search-below-nav-fold]] [[website-chrome-polish]] public-site.chrome:primary-nav public-site.chrome:odm-shell
