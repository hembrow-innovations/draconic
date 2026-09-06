---
id: "ticket-652-chapter-nav-no-focus-ring"
title: "Grouped chapter links have no focus-visible ring"
kind: ticket
status: open
ticket_type: bug
blocked_by: []
tags: [website, public-site]
sprint: "website-odm-match"
created_at: "2026-09-07T21:30:00Z"
updated_at: "2026-09-07T21:30:00Z"
---

# Grouped chapter links have no focus-visible ring

## Signal

Learn and Reference chapter links in the side nav have no `focus-visible` token ring. Hub Learn, Reference, GitHub, search, theme toggle, and skip do. Those chapter lists are now part of primary nav. `public-site.a11y:keyboard-small` says primary nav works with keyboard.

## Fit

this project, later slice. Do not execute from this ticket.

## Notes

- **Promise**: `public-site.a11y:keyboard-small`.
- **Walked**: `/`, `/learn`, `/install`, `/reference`, `/cli`. Install chapter link classes: `font-body text-body text-muted no-underline hover:text-ink` (no ring). Learn hub link classes include `focus-visible:ring-2 focus-visible:ring-accent`.
- **Parent**: [[slice-644-ui-audit]]
