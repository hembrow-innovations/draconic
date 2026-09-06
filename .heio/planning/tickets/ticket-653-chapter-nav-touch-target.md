---
id: "ticket-653-chapter-nav-touch-target"
title: "Grouped chapter links have a zero min-height tap target"
kind: ticket
status: open
ticket_type: observation
blocked_by: []
tags: [website, public-site]
sprint: "website-odm-match"
created_at: "2026-09-07T21:30:00Z"
updated_at: "2026-09-07T21:30:00Z"
---

# Grouped chapter links have a zero min-height tap target

## Signal

On a 375-wide walk, hub Learn / Reference / GitHub use `min-h-11` (44px). Grouped chapter and working-page links in the same side nav have `min-height: 0`. No contract names a 44px target. Small-viewport primary nav is still the walk surface.

## Fit

this project, later slice. Do not execute from this ticket.

## Notes

- **Promise**: none for tap size. Adjacent to `public-site.a11y:keyboard-small`.
- **Walked**: `/cli` at 375 by 812. Install chapter `minHeight` 0px. Learn hub `minHeight` 44px.
- **Parent**: [[slice-644-ui-audit]]
