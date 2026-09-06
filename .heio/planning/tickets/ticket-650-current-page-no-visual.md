---
id: "ticket-650-current-page-no-visual"
title: "Current side-nav page has no visual treatment"
kind: ticket
status: open
ticket_type: observation
blocked_by: []
tags: [website, public-site]
sprint: "website-odm-match"
created_at: "2026-09-07T21:30:00Z"
updated_at: "2026-09-07T21:30:00Z"
---

# Current side-nav page has no visual treatment

## Signal

The open page gets `aria-current="page"` on the side nav, but the current link stays the same muted ink as every other chapter link (`rgb(91, 111, 134)`, weight 400). ODM uses a mint current-page treatment. Token `accent-2` exists and is used on home path numbers, not on the current nav item. No contract promise names that color.

## Fit

this project, later slice. Do not execute from this ticket.

## Notes

- **Promise**: none for current-page color. `public-site.chrome:primary-nav` and the docs-nav-groups slice only lock `aria-current`.
- **Walked**: `/` (wordmark current), `/learn` (Learn hub still muted), `/install` (Install still muted), `/reference` (Reference hub still muted), `/cli` (CLI still muted).
- **Parent**: [[slice-644-ui-audit]]
