---
id: "slice-603-learn-prev-next"
title: "Learn prev and next"
kind: slice
status: met
sprint: "website-redesign"
blocked_by:
  - slice-602-learn-pages
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T16:45:00Z"
---

# Learn prev and next

## Why

After Dual worlds the Learn path is a sequence. People on a narrow article column should walk that sequence without hunting the sidebar. Prev and next make the path walkable as a path, not only as a list.

## Done

Each Learn chapter has prev and next matching `website/learn.md` order: Install; from JavaScript / from systems; Dual worlds; modules; native types; host I/O; packages.

## Blocked by

- [[slice-602-learn-pages]]: prev and next only make sense once every chapter route exists.

## Non-goals

- **Reference**: [[slice-604-reference-hub-pages]]
- **Search, theme, mobile, fences**: [[slice-605-search]] [[slice-606-theme-toggle]] [[slice-607-mobile-a11y]] [[slice-608-fence-static-deploy]]
- **Playground**: in-page runners stay out
- **Next.js**: Start only
- **cargo test --workspace**: not this oracle
- **Rewriting chapter copy or inventing a second order**: follow learn.md
- **generate.drac retirement**: stays until [[slice-608-fence-static-deploy]]

## Oracle checklist

- [x] O1: each Learn chapter has prev/next matching learn.md order
  CHECK: pnpm --dir website test -- learn-prev-next
  EXPECT: Test Files  1 passed
  EVIDENCE: Test Files  1 passed (1)

## Pool

Durable links to task ids. Never drop them.

- `[[task-622-learn-prev-next]]`

## See also

[[location-589-public-site]] [[ticket-628-public-site-redesign]] [[0013-public-site-tanstack-start]] docs/specs/draconic/public-site/ website/ CONTEXT.md
