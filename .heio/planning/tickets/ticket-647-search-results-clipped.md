---
id: "ticket-647-search-results-clipped"
title: "Search result list is clipped by the sticky aside overflow"
kind: ticket
status: open
ticket_type: bug
blocked_by: []
tags: [website, public-site]
sprint: "website-odm-match"
created_at: "2026-09-07T21:30:00Z"
updated_at: "2026-09-07T21:30:00Z"
---

# Search result list is clipped by the sticky aside overflow

## Signal

Typing Dual worlds into in-site search on `/cli` produces a matching `/dual-worlds` link, but the result list is `position: absolute` inside an aside with `overflow: auto`. At 1280 by 720 the list's box is below the aside bottom (top 772, aside bottom 720), so the hit is clipped. Same clip at 375 by 812. Distinct from the field sitting below the fold: even a visible query can hide its hits.

## Fit

this project, later slice. Do not execute from this ticket.

## Notes

- **Promise**: `public-site.search:titles-headings`.
- **Walked**: `/cli` at 1280 by 720 and 375 by 812. Query Dual worlds. Result list clipped by aside overflow.
- **Parent**: [[slice-644-ui-audit]]
