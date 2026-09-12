---
id: "ticket-906-search-chrome-source-greps"
title: "Search keyboard, announce, and session tests grep source"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
sprint: "website-chrome-polish"
created_at: "2026-09-12T10:47:00Z"
updated_at: "2026-09-12T10:47:00Z"
---

# Search keyboard, announce, and session tests grep source

## Signal

Website chrome polish review 2026-09-12 after [[slice-882-search-keyboard-live]] and [[slice-873-search-session-reset]] were marked met. `public-site.search:keyboard-live` and `public-site.search:session` are locked by token greps in `website/src/tests/search.test.ts`. Vitest does not mount SiteSearch, press Arrow Down or Escape, or follow a hit. A no-op handler still passes.

## Fit

Unknown until triage. Follow-up after met [[slice-882-search-keyboard-live]] / [[task-883-search-keyboard-live]] and [[slice-873-search-session-reset]] / [[task-874-search-session-reset]]. Distinct from promoted [[ticket-857-search-keyboard-live]] and [[ticket-852-search-stays-open]] (missing combobox and leftover hits). Index matching is a different lock; [[ticket-902-stale-vault-search-misses]] stays its own observation.

## Notes

- `website/src/tests/search.test.ts` greps `ArrowDown`, `ArrowUp`, `Enter`, `aria-live`, `setQuery`, and Escape
- `not.toContain("aria-current")` does not lock leftover TanStack Link hits stealing `aria-current="page"`; SiteSearch never wrote that attribute
- Index queries (Dual worlds, body terms, vault-only phrases) actually run through `querySearchIndex`
- Following a hit that is already the current pathname and hash may not fire the location effect, so the list can stay open
- Website tests do not use testing-library

## Parent

[[slice-882-search-keyboard-live]]
