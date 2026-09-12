---
id: "slice-863-favicon"
title: "Public site favicon"
kind: slice
status: met
sprint: "website-chrome-polish"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-12T07:05:00Z"
updated_at: "2026-09-12T07:30:00Z"
---

# Public site favicon

## Why

Every load of the live origin 404s `/favicon.ico`. The tab has no site icon and first paint is not a clean console. Chrome polish, not Learn or Reference copy.

## Done

The public origin serves a favicon so the browser tab shows a site icon and the missing-icon 404 is gone.

## Blocked by

None.

## Non-goals

- **Learn or Reference copy**
- **Per-page document titles**: [[slice-871-per-page-titles]]
- **Playground, vault-as-site, Start replacement**

## Oracle checklist

- [x] O1: primary-nav chrome locks a served favicon
  CHECK: pnpm --dir website exec vitest run site-header-primary-nav
  EXPECT: Test Files  1 passed
  EVIDENCE: 2026-09-12 `pnpm --dir website exec vitest run site-header-primary-nav` → Test Files  1 passed (1); `pnpm --dir website exec tsc --noEmit` exits 0

## Pool

- [[task-864-favicon]]

## See also

[[ticket-847-missing-favicon]] [[website-chrome-polish]] [[location-589-public-site]] [[Public site — Contract]]
