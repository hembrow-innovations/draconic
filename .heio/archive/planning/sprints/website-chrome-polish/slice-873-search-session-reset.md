---
id: "slice-873-search-session-reset"
title: "Search session reset"
kind: slice
status: met
sprint: "website-chrome-polish"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-12T07:05:00Z"
updated_at: "2026-09-12T18:10:00Z"
---

# Search session reset

## Why

In-site search does not clear when a visitor follows a hit, so stale results overlay the next article and a leftover hit can take `aria-current="page"`. Escape also leaves the list open. This is session lifecycle, not what the index matches.

## Done

Following a search hit or pressing Escape clears the query and result list. `aria-current="page"` on the destination is the side-nav current page, not a leftover hit.

## Blocked by

None.

## Non-goals

- **Body-term indexing**: [[ticket-821-search-body-terms]]
- **Result-list clipping**: closed [[ticket-647-search-results-clipped]]
- **Moving search out of the side nav**

## Oracle checklist

- [x] O1: search query and results reset on navigation and Escape
  CHECK: pnpm --dir website exec vitest run search
  EXPECT: Test Files  1 passed
  EVIDENCE: pnpm --dir website exec vitest run search → Test Files  1 passed (1). task-874 archived.

## Pool

- [[task-874-search-session-reset]]

## See also

[[ticket-852-search-stays-open]] [[ticket-821-search-body-terms]] [[website-chrome-polish]] public-site.search:titles-headings public-site.chrome:current-page
