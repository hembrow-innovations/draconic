---
id: "slice-633-docs-nav-groups"
title: "ODM docs nav groups"
kind: slice
status: frozen
sprint: "website-odm-match"
blocked_by:
  - slice-630-site-shell
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T18:00:00Z"
---

# ODM docs nav groups

## Why

ODM docs are one left nav with labeled groups and aria-current on the open page, not a top bar plus a second unlabeled list. Learn and Reference page lists belong in that side nav.

## Done

The site side nav groups Learn pages and Reference pages in hub order, marks the current page with aria-current, and still exposes wordmark, Learn, Reference, and GitHub. Section sidebar plus badge still hold.

## Blocked by

- [[slice-630-site-shell]]: groups hang off the site side nav.

## Non-goals

- **Hub card grids**: [[slice-634-hub-cards]]
- **Changing Learn or Reference chapter order**
- **Playground**

## Oracle checklist

- [ ] O1: side nav groups list Learn and Reference pages with aria-current; docs sidebar and badge still hold
  CHECK: pnpm --dir website exec vitest run learn-hub-nav reference-hub-pages docs-shell
  EXPECT: Test Files  3 passed
  EVIDENCE: pending

## Pool

- `[[task-640-docs-nav-groups]]`

## See also

[[location-589-public-site]] [[website-odm-match]] public-site.nav:learn-reference-status public-site.chrome:docs-sidebar website/src/features/learn/ website/src/features/reference/
