---
id: "ticket-857-search-keyboard-live"
title: "Search results are not keyboardable or announced"
kind: ticket
status: open
ticket_type: bug
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T06:56:00Z"
updated_at: "2026-09-12T06:56:00Z"
---

# Search results are not keyboardable or announced

## Signal

Playwright audit 2026-09-12. In-site search finds Dual worlds by title, and a miss shows No matching pages, but Arrow Down and Enter stay on the field. The control is a plain search input with no combobox wiring and no live region, so keyboard and screen-reader users cannot move through hits.

## Fit

Unknown until triage. Distinct from open [[ticket-852-search-stays-open]] (query leftover after navigation) and [[ticket-821-search-body-terms]] (what the index matches). Promise `public-site.search:titles-headings` and `public-site.a11y:keyboard-small`.

## Notes

- Live: Search on `/install`
- Query Dual worlds listed Learn · Dual worlds → `/dual-worlds`
- Query zzzz-no-hit showed No matching pages
- Input has `aria-label=Search`, no `aria-expanded`, `aria-controls`, or `aria-activedescendant`
- Result list is a `ul` of links, not a listbox
- Arrow Down left focus on the searchbox; Enter did not follow the first hit
- No `aria-live` on hits or misses
- `website/src/components/SiteSearch/SiteSearch.tsx`

## Parent

[[Public site — Contract]]
