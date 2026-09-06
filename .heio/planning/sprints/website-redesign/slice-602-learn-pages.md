---
id: "slice-602-learn-pages"
title: "Learn chapter pages"
kind: slice
status: met
sprint: "website-redesign"
blocked_by:
  - slice-601-learn-hub-nav
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T01:16:00Z"
---
# Learn chapter pages

## Why

Each Learn markdown file is a chapter people can open. The teaching copy already lives in `website/`. Rewriting that prose would fork the language story. Only retarget links that still say `.html`. Status badges tell shipped versus not-yet on each chapter.

## Done

Routes for install, from-javascript, from-systems, dual-worlds, modules, native-types, host-io, and packages render from the existing markdown. Status badges show.

## Blocked by

- [[slice-601-learn-hub-nav]]: chapter routes hang off the Learn hub and sidebar, not a pile of orphan URLs.

## Non-goals

- **Prev/next**: [[slice-603-learn-prev-next]]
- **Reference pages**: [[slice-604-reference-hub-pages]]
- **Search, theme, mobile, fences**: [[slice-605-search]] [[slice-606-theme-toggle]] [[slice-607-mobile-a11y]] [[slice-608-fence-static-deploy]]
- **Playground**: in-page runners stay out
- **Next.js**: Start only
- **cargo test --workspace**: not this oracle
- **Rewriting teaching copy**: except links that used `.html`
- **generate.drac retirement**: stays until [[slice-608-fence-static-deploy]]

## Oracle checklist

- [x] O1: Learn chapter routes render from existing markdown and show status badges
  CHECK: pnpm --dir website test -- learn-pages
  EXPECT: Test Files  1 passed
  EVIDENCE: Test Files  1 passed (1)

## Pool

Durable links to task ids. Never drop them.

- `[[task-621-learn-pages]]`

## See also

[[location-589-public-site]] [[ticket-628-public-site-redesign]] [[0013-public-site-tanstack-start]] docs/specs/draconic/public-site/ website/ CONTEXT.md
