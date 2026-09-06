---
id: "slice-632-docs-article-odm"
title: "ODM docs article"
kind: slice
status: frozen
sprint: "website-odm-match"
blocked_by:
  - slice-629-odm-tokens
  - slice-630-site-shell
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T18:00:00Z"
---

# ODM docs article

## Why

ODM docs are not a bare markdown dump. The main column has a section kicker, one h1, optional muted lede, bordered h2s, code on the elevated surface, and a related-link footer. Badge status stays.

## Done

Learn and Reference articles render kicker, heading, status badge, ODM-styled markdown, and a related-link footer. DocsShell is still not used on `/`.

## Blocked by

- [[slice-629-odm-tokens]]: prose uses tokens.
- [[slice-630-site-shell]]: article sits in the site main column.

## Non-goals

- **Moving chapter lists into the site nav**: [[slice-633-docs-nav-groups]]
- **Hub card grids**: [[slice-634-hub-cards]]
- **Changing markdown subset or fence rules**
- **Playground**

## Oracle checklist

- [ ] O1: docs article has kicker, badge, and ODM prose; home still does not use DocsShell
  CHECK: pnpm --dir website exec vitest run docs-shell markdown-render
  EXPECT: Test Files  2 passed
  EVIDENCE: pending

## Pool

- `[[task-639-docs-article-odm]]`

## See also

[[location-589-public-site]] [[website-odm-match]] public-site.chrome:docs-sidebar website/src/features/docs/
