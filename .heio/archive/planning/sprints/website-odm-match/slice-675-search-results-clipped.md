---
id: "slice-675-search-results-clipped"
title: "Unclipped search results"
kind: slice
status: met
sprint: "website-odm-match"
blocked_by:
  - slice-673-search-below-nav-fold
tags: [website, public-site]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-07T23:50:00Z"
---
# Unclipped search results

## Why

A Dual worlds query already finds `/dual-worlds`, but the result list is clipped by the sticky aside's overflow. Finding a page is not done if the hit cannot be seen or activated.

## Done

Typing Dual worlds into in-site search shows the matching result without clipping by the sticky aside, at both 1280 by 720 and 375 by 812.

## Blocked by

- [[slice-673-search-below-nav-fold]]: pin the field first so result layout is not fighting an off-screen query.

## Non-goals

- **Moving search out of the side nav**
- **Indexing vault notes or body-only phrases**
- **Playground**

## Oracle checklist

- [x] O1: Dual worlds search still hits `/dual-worlds` and the result list is not `absolute` inside overflowing aside clip
  CHECK: pnpm --dir website exec vitest run search
  EXPECT: Test Files  1 passed
  EVIDENCE: vitest search — Test Files  1 passed (1); Dual worlds hits `/dual-worlds`; result list is not `absolute` inside overflowing aside clip

## Pool

- `[[task-676-search-results-clipped]]`

## See also

[[ticket-647-search-results-clipped]] [[slice-673-search-below-nav-fold]] [[website-odm-match]] public-site.search:titles-headings
