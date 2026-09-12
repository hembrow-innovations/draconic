---
id: "slice-882-search-keyboard-live"
title: "Search keyboard and live region"
kind: slice
status: frozen
sprint: "website-chrome-polish"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Search keyboard and live region

## Why

Hits exist, but Arrow Down and Enter stay on the field, so keyboard and screen-reader users cannot move through results. Chrome a11y, not index widening.

## Done

Search results are keyboard-reachable and announced, Dual worlds still hits, and a miss still shows No matching pages.

## Blocked by

None.

## Non-goals

- **Body-term indexing**: [[slice-894-search-body-terms]]
- **Search session reset**: [[slice-873-search-session-reset]]
- **Fence Copy announcements**: [[slice-884-copy-announcement]]
- **Moving search out of the nav**

## Oracle checklist

- [ ] O1: search keyboard and live region lock
  CHECK: pnpm --dir website exec vitest run search
  EXPECT: Test Files  1 passed
  EVIDENCE: pending

## Pool

- [[task-883-search-keyboard-live]]

## See also

[[ticket-857-search-keyboard-live]] [[website-chrome-polish]] [[location-589-public-site]] [[Public site — Contract]]
