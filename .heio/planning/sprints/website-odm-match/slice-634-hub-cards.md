---
id: "slice-634-hub-cards"
title: "ODM hub cards"
kind: slice
status: frozen
sprint: "website-odm-match"
blocked_by:
  - slice-632-docs-article-odm
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T18:00:00Z"
---

# ODM hub cards

## Why

ODM hubs (Guides, Features) send the reader through a card grid of linked titles, not a bare list. Learn and Reference hubs should do the same without becoming the home landing.

## Done

`/learn` and `/reference` show a card grid of their chapter or working-page links inside the docs article. Hubs still load their markdown and still are not `/`.

## Blocked by

- [[slice-632-docs-article-odm]]: cards sit in the article column.

## Non-goals

- **Changing hub markdown teaching copy beyond layout**
- **Home feature cards**: [[slice-631-home-odm-layout]]
- **Playground**

## Oracle checklist

- [ ] O1: Learn and Reference hubs render a card grid of their pages and still load hub markdown
  CHECK: pnpm --dir website exec vitest run learn-hub-nav reference-hub-pages
  EXPECT: Test Files  2 passed
  EVIDENCE: pending

## Pool

- `[[task-641-hub-cards]]`

## See also

[[location-589-public-site]] [[website-odm-match]] website/src/routes/learn.tsx website/src/routes/reference.tsx
