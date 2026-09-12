---
id: "slice-886-reference-related-footer"
title: "Reference related-link footer"
kind: slice
status: frozen
sprint: "website-chrome-polish"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Reference related-link footer

## Why

Purpose and `public-site.chrome:docs-article-order` name a related-link footer on Learn and Reference. Learn has a sequence pager. Reference working pages end after the markdown.

## Done

Each Reference working page has a related-link footer that walks the locked Reference sequence. Learn pager behavior is unchanged.

## Blocked by

None.

## Non-goals

- **Learn pager rewrite**
- **Reference hub cards rewrite**
- **Teaching copy**
- **Playground, vault-as-site**

## Oracle checklist

- [ ] O1: Reference working pages lock a related-link footer
  CHECK: pnpm --dir website exec vitest run reference-prev-next docs-shell
  EXPECT: Test Files  2 passed
  EVIDENCE: pending

## Pool

- [[task-887-reference-related-footer]]

## See also

[[ticket-859-reference-related-footer]] [[website-chrome-polish]] [[location-589-public-site]] [[Public site — Contract]]
