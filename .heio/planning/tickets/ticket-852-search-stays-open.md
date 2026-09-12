---
id: "ticket-852-search-stays-open"
title: "Search query and results stay open after navigation"
kind: ticket
status: open
ticket_type: bug
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T05:58:32Z"
updated_at: "2026-09-12T05:58:32Z"
---

# Search query and results stay open after navigation

## Signal

Website swarm 2026-09-12. In-site search does not clear when you follow a hit, so stale results overlay the next article. The leftover hit can take `aria-current=page`. Escape also leaves the list open.

## Fit

Unknown until triage. Distinct from open [[ticket-821-search-body-terms]] (what the index matches) and closed [[ticket-647-search-results-clipped]] (overflow clip). This is session lifecycle.

## Notes

- Typed bindgen on `/reference-packages`, followed Reference · CLI · bindgen to `/cli#bindgen`
- Searchbox still showed bindgen; result list still visible
- `document.querySelector('[aria-current=page]')` was the leftover search hit, not the CLI side-nav link
- Small viewport: Escape left query install and the list in the snapshot
- `website/src/components/SiteSearch/SiteSearch.tsx` has local query state, no route reset, no Escape handler

## Parent

[[Public site — Contract]]
