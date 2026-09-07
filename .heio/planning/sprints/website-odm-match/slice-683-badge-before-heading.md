---
id: "slice-683-badge-before-heading"
title: "Badge after the heading"
kind: slice
status: met
sprint: "website-odm-match"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-08T19:20:00Z"
---

# Badge after the heading

## Why

Purpose names docs-article order as kicker, heading, status badge, then related-link footer. The article currently paints the badge before the markdown `h1`.

## Done

Learn and Reference articles render kicker, then the page heading, then the shipped or not-yet badge, then the rest of the body and footer.

## Blocked by

None.

## Non-goals

- **First section heading rule**: [[slice-689-first-h2-border]]
- **Changing badge copy or frontmatter status values**
- **Using DocsShell on home**

## Oracle checklist

- [x] O1: docs article source order is kicker, heading, badge; docs-shell tests still pass
  CHECK: pnpm --dir website exec vitest run docs-shell
  EXPECT: Test Files  1 passed
  EVIDENCE: vitest docs-shell — Test Files  1 passed (1); Tests  1 passed; commit 0da25482

## Pool

- `[[task-684-badge-before-heading]]`

## See also

[[ticket-651-badge-before-heading]] [[website-odm-match]] [[docs/specs/draconic/public-site/purpose]] public-site.chrome:docs-sidebar
