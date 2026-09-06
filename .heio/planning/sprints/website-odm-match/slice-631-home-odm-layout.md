---
id: "slice-631-home-odm-layout"
title: "ODM home layout"
kind: slice
status: met
sprint: "website-odm-match"
blocked_by:
  - slice-630-site-shell
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T23:50:00Z"
---

# ODM home layout

## Why

ODM home sits in the same two-column shell and uses kicker, lead, CTA row, numbered path steps, and a card grid. Draconic home copy stays; the bands change.

## Done

`/` keeps the locked pitch and Install plus Learn CTAs, and lays them out as kicker, hero, CTA row, path steps, and feature cards inside the site shell. Home still does not use DocsShell.

## Blocked by

- [[slice-630-site-shell]]: home is an article column in the shell, not a second chrome.

## Non-goals

- **Rewriting home pitch or CTA targets**
- **Hub card grids**: [[slice-634-hub-cards]]
- **Playground, Get started, tutorial, vault**

## Oracle checklist

- [x] O1: home keeps locked pitch and CTAs and uses kicker, CTA row, path steps, and cards inside the shell
  CHECK: pnpm --dir website exec vitest run home-hero-and-cta home-features
  EXPECT: Test Files  2 passed
  EVIDENCE: `pnpm --dir website exec vitest run home-hero-and-cta home-features` → Test Files  2 passed (2); task-638 completed in archive

## Pool

- `[[task-638-home-odm-layout]]`

## See also

[[location-589-public-site]] [[website-odm-match]] public-site.home:landing website/src/features/home/
