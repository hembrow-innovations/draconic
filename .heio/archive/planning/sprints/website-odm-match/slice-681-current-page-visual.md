---
id: "slice-681-current-page-visual"
title: "Current page nav treatment"
kind: slice
status: met
sprint: "website-odm-match"
blocked_by:
  - slice-687-chapter-nav-touch-target
tags: [website, public-site]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-08T19:40:00Z"
---

# Current page nav treatment

## Why

The open page already gets `aria-current="page"`, but the current link stays the same muted ink as every other chapter link. ODM uses a mint current-page treatment. Token `accent-2` already exists.

## Done

The current side-nav item is visually distinct from sibling links, using an existing semantic token, while `aria-current="page"` still holds.

## Blocked by

- [[slice-687-chapter-nav-touch-target]]: current-page chrome lands on the same chapter-link variants after tap size.

## Non-goals

- **Inventing a new color token**
- **Changing which page is current**
- **Copying ODM product copy**

## Oracle checklist

- [x] O1: current-page treatment is locked on contract and tests; aria-current still holds
  CHECK: pnpm --dir website exec vitest run site-header-primary-nav learn-hub-nav reference-hub-pages
  EXPECT: Test Files  3 passed
  EVIDENCE: `Test Files  3 passed (3)` Duration 178ms; task-682 completed; commit c8f217d1

## Pool

- `[[task-682-current-page-visual]]`

## See also

[[ticket-650-current-page-no-visual]] [[website-odm-match]] public-site.chrome:primary-nav
