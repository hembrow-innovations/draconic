---
id: "slice-890-meta-description"
title: "Page meta description"
kind: slice
status: met
sprint: "website-chrome-polish"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T21:10:00Z"
---

# Page meta description

## Why

Home, Learn, and Reference expose a viewport tag and a title, but no description, canonical, or Open Graph, so shares and search snippets have no page-specific summary. Browser titles are a different slice.

## Done

Each public page exposes a page-specific meta description, a canonical URL, and Open Graph tags so shares and search snippets summarize that page.

## Blocked by

None.

## Non-goals

- **Per-page document titles**: [[slice-871-per-page-titles]]
- **Theme-color, Twitter cards, og:image**
- **New marketing slogans**
- **Playground, vault-as-site**

## Oracle checklist

- [x] O1: page-specific share meta locks description, canonical, and Open Graph
  CHECK: pnpm --dir website exec vitest run meta-description
  EXPECT: Test Files  1 passed
  EVIDENCE: Test Files  1 passed (1); Tests  1 passed (1); task-891 completed in archive

## Pool

- [[task-891-meta-description]]

## See also

[[ticket-861-missing-meta-description]] [[website-chrome-polish]] [[location-589-public-site]] [[Public site purpose]]
