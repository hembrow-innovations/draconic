---
id: "ticket-646-search-below-nav-fold"
title: "Search and theme toggle sit below the sticky side-nav fold"
kind: ticket
status: open
ticket_type: bug
blocked_by: []
tags: [website, public-site]
sprint: "website-odm-match"
created_at: "2026-09-07T21:30:00Z"
updated_at: "2026-09-07T21:30:00Z"
---

# Search and theme toggle sit below the sticky side-nav fold

## Signal

On a 1280 by 720 desktop walk of `/cli`, the sticky side nav is 856px tall. Search sits at about 724px and the theme toggle follows it, so both are off-screen until the aside is scrolled. `public-site.search:titles-headings` and purpose both keep search in that nav. The grouped Learn and Reference lists fill the pane first.

## Fit

this project, later slice. Do not execute from this ticket.

## Notes

- **Promise**: `public-site.search:titles-headings`. Purpose also keeps the theme toggle in the side nav.
- **Walked**: `/cli` at 1280 by 720. Aside `scrollHeight` 856, `clientHeight` 720, search `getBoundingClientRect().top` 724 (not visible).
- **Parent**: [[slice-644-ui-audit]]
