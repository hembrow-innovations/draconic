---
id: "ticket-849-current-page-contrast"
title: "Current-page nav green fails contrast"
kind: ticket
status: open
ticket_type: bug
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T05:58:32Z"
updated_at: "2026-09-12T05:58:32Z"
---

# Current-page nav green fails contrast

## Signal

Website swarm 2026-09-12. The open page is marked `aria-current=page` and uses accent green, which is distinct from muted siblings, but that green on the light canvas is about 3:1 for body-sized nav text.

## Fit

Unknown until triage. Follow-on to closed [[ticket-650-current-page-no-visual]]: current-page color exists; contrast does not hold. Promise `public-site.chrome:current-page`.

## Notes

- Live: `/` wordmark, `/learn` Learn hub, `/reference` Reference hub
- Current color `rgb(47, 158, 111)` on canvas `rgb(245, 248, 252)`
- Inactive Learn and Reference use muted `rgb(91, 111, 134)`
- Token is `accent-2` on `aria-[current=page]` in `website/src/components/SiteHeader/SiteHeader.variants.ts`

## Parent

[[Public site — Contract]]
