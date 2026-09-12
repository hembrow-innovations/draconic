---
id: "slice-880-empty-404"
title: "Unknown URL recovery"
kind: slice
status: met
sprint: "website-chrome-polish"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T18:23:01Z"
---

# Unknown URL recovery

## Why

An unknown path keeps skip-link, side nav, and footer, but main is only the words Not Found. No heading, no way back, router warns that `__root__` has no `notFoundComponent`. Visitor recovery, not teaching copy.

## Done

An unknown URL keeps site chrome, main has a heading and a way back to a real page, the router default Not Found paragraph is gone, and the response is a 404.

## Blocked by

None.

## Non-goals

- **Per-page titles on real routes**: [[slice-871-per-page-titles]]
- **Fake `aria-current=page` on a missing path**
- **Learn or Reference copy**
- **Playground, vault-as-site**

## Oracle checklist

- [x] O1: unknown-URL recovery locks a heading and a way back
  CHECK: pnpm --dir website exec vitest run not-found
  EXPECT: Test Files  1 passed
  EVIDENCE: pnpm --dir website exec vitest run not-found → Test Files  1 passed (1). task-881 archived.

## Pool

- [[task-881-empty-404]]

## See also

[[ticket-856-empty-404]] [[website-chrome-polish]] [[location-589-public-site]] [[Public site — Contract]]
