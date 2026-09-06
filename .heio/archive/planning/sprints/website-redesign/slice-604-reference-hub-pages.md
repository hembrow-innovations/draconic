---
id: "slice-604-reference-hub-pages"
title: "Reference hub and pages"
kind: slice
status: met
sprint: "website-redesign"
blocked_by:
  - slice-600-docs-shell
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T01:32:00Z"
---

# Reference hub and pages

## Why

Reference is the set of pages kept open while writing a Program, not an API dump and not the Learn path. The hub is `website/reference.md`. CLI, types, Dual-world rules, host I/O, and packages are the working surfaces.

## Done

A Reference hub route exists plus those five pages, rendered from existing markdown (`reference.md`, `cli.md`, `types.md`, `dual-world-rules.md`, `reference-host-io.md`, `reference-packages.md`).

## Blocked by

- [[slice-600-docs-shell]]: Reference uses the same handbook chrome as Learn, not a second layout.

## Non-goals

- **Learn chapters and prev/next**: [[slice-601-learn-hub-nav]] [[slice-602-learn-pages]] [[slice-603-learn-prev-next]]
- **Search, theme, mobile, fences**: [[slice-605-search]] [[slice-606-theme-toggle]] [[slice-607-mobile-a11y]] [[slice-608-fence-static-deploy]]
- **Playground**: in-page runners stay out
- **Next.js**: Start only
- **cargo test --workspace**: not this oracle
- **Generated API dump or vault API notes as the site**: keep website/ markdown
- **generate.drac retirement**: stays until [[slice-608-fence-static-deploy]]

## Oracle checklist

- [x] O1: Reference hub and the five working pages render from existing markdown
  CHECK: pnpm --dir website test -- reference-hub-pages
  EXPECT: Test Files  1 passed
  EVIDENCE: Test Files  1 passed (1)

## Pool

Durable links to task ids. Never drop them.

- `[[task-623-reference-hub-pages]]`

## See also

[[location-589-public-site]] [[ticket-628-public-site-redesign]] [[0013-public-site-tanstack-start]] docs/specs/draconic/public-site/ website/ CONTEXT.md public-site.ia:reference-walkable
