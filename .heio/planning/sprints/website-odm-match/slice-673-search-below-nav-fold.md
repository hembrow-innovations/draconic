---
id: "slice-673-search-below-nav-fold"
title: "Search above the nav fold"
kind: slice
status: frozen
sprint: "website-odm-match"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-07T23:15:00Z"
---

# Search above the nav fold

## Why

In-site search and the theme toggle live in the sticky side nav, but the grouped chapter lists fill the pane first. A visitor at a normal desktop height should not have to scroll the aside to type a query.

## Done

At a 1280 by 720 desktop viewport, the search field and theme toggle sit inside the sticky aside's visible box without scrolling that aside. They stay in the side nav.

## Blocked by

None.

## Non-goals

- **Unclipping the result list**: [[slice-675-search-results-clipped]]
- **Dropping search or the theme toggle**
- **Playground, vault-as-site, Start replacement**

## Oracle checklist

- [ ] O1: search and theme toggle remain in the sticky side nav and are not ordered after the chapter lists
  CHECK: pnpm --dir website exec vitest run site-header-primary-nav search
  EXPECT: Test Files  2 passed
  EVIDENCE: pending

## Pool

- `[[task-674-search-below-nav-fold]]`

## See also

[[ticket-646-search-below-nav-fold]] [[website-odm-match]] [[location-589-public-site]] public-site.search:titles-headings public-site.chrome:odm-shell
