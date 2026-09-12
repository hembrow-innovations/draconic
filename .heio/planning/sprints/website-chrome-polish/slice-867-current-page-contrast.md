---
id: "slice-867-current-page-contrast"
title: "Current-page nav contrast"
kind: slice
status: frozen
sprint: "website-chrome-polish"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-12T07:05:00Z"
updated_at: "2026-09-12T07:05:00Z"
---

# Current-page nav contrast

## Why

The open page already uses `aria-current="page"` and accent green, so it is distinct from muted siblings. That green on the light canvas is about 3:1 for body-sized nav text. Closed [[ticket-650-current-page-no-visual]] shipped color; contrast does not hold.

## Done

Current-page nav text meets 4.5:1 contrast against the canvas, stays visually distinct from muted siblings, and still uses `aria-current="page"`.

## Blocked by

None.

## Non-goals

- **Inventing a new color token name**
- **Changing which page is current**
- **Copying ODM product copy**

## Oracle checklist

- [ ] O1: current-page treatment is locked and no longer the 3:1 accent-2-on-canvas pair
  CHECK: pnpm --dir website exec vitest run site-header-primary-nav learn-hub-nav reference-hub-pages
  EXPECT: Test Files  3 passed
  EVIDENCE: pending

## Pool

- [[task-868-current-page-contrast]]

## See also

[[ticket-849-current-page-contrast]] [[ticket-650-current-page-no-visual]] [[website-chrome-polish]] public-site.chrome:current-page
