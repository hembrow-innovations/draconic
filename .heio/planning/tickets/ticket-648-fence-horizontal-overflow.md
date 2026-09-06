---
id: "ticket-648-fence-horizontal-overflow"
title: "Code fences overflow the small viewport horizontally"
kind: ticket
status: open
ticket_type: bug
blocked_by: []
tags: [website, public-site]
sprint: "website-odm-match"
created_at: "2026-09-07T21:30:00Z"
updated_at: "2026-09-07T21:30:00Z"
---

# Code fences overflow the small viewport horizontally

## Signal

On `/cli` at 375 by 812, article `pre` uses `overflow-x: visible`. The first fence's `code` box extends to 461px while the viewport is 375px, so the page grows a horizontal scrollbar. `public-site.a11y:keyboard-small` says article reading works at a small viewport.

## Fit

this project, later slice. Do not execute from this ticket.

## Notes

- **Promise**: `public-site.a11y:keyboard-small`.
- **Walked**: `/cli` at 375 by 812. `document.documentElement.scrollWidth` 461. Overflowing node is `CODE` inside a fence (`scrollWidth` 429, `clientWidth` 311).
- **Parent**: [[slice-644-ui-audit]]
