---
id: "ticket-651-badge-before-heading"
title: "Docs article paints the status badge before the heading"
kind: ticket
status: open
ticket_type: observation
blocked_by: []
tags: [website, public-site]
sprint: "website-odm-match"
created_at: "2026-09-07T21:30:00Z"
updated_at: "2026-09-07T21:30:00Z"
---

# Docs article paints the status badge before the heading

## Signal

Purpose in-scope for the docs article is a section kicker, heading, status badge, and related-link footer. On Learn and Reference pages the article children are kicker, then the shipped or not-yet Badge, then the markdown `h1`. The badge sits above the heading, not after it.

## Fit

this project, later slice. Do not execute from this ticket.

## Notes

- **Promise**: purpose docs-article order. `public-site.chrome:docs-sidebar` only requires a badge from frontmatter, not that order.
- **Walked**: `/learn`, `/install`, `/reference`, `/cli`. Same order on each: kicker, Badge, then `h1`.
- **Parent**: [[slice-644-ui-audit]]
