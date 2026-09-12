---
id: "ticket-856-empty-404"
title: "Unknown URLs show a generic empty Not Found"
kind: ticket
status: open
ticket_type: bug
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T06:56:00Z"
updated_at: "2026-09-12T06:56:00Z"
---

# Unknown URLs show a generic empty Not Found

## Signal

Playwright audit 2026-09-12. A missing path keeps site chrome but the main column is only the words Not Found. No heading, no way back, no current-page nav. The console warns that the router has no notFoundComponent.

## Fit

Unknown until triage. Visitor recovery on [[location-589-public-site]], not a Learn or Reference copy change.

## Notes

- Live: `http://localhost:3000/no-such-page`
- HTTP status 404, document title still Draconic
- Main text is a paragraph, not a heading
- Console: TanStack Router defaultNotFoundComponent warning on `__root__`
- Side nav and footer still render; no `aria-current=page`

## Parent

[[Public site — Contract]]
