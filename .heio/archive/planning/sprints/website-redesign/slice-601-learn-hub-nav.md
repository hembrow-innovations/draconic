---
id: "slice-601-learn-hub-nav"
title: "Learn hub and nav"
kind: slice
status: met
sprint: "website-redesign"
blocked_by:
  - slice-600-docs-shell
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T01:02:00Z"
---

# Learn hub and nav

## Why

Learn is the public path for people who already write JavaScript or systems code, not a tutorial dump. The hub is `website/learn.md`. A sidebar that lists Install through packages is how a visitor sees the whole walk without guessing URLs.

## Done

A Learn hub route exists. The docs sidebar links the Learn chapters: Install, from JavaScript, from systems, Dual worlds, modules, native types, host I/O, packages.

## Blocked by

- [[slice-600-docs-shell]]: the hub is a handbook page, so it needs aside plus main plus badge before chapter links have a home.

## Non-goals

- **Chapter bodies**: [[slice-602-learn-pages]]
- **Prev/next**: [[slice-603-learn-prev-next]]
- **Reference**: [[slice-604-reference-hub-pages]]
- **Search, theme, mobile, fences**: [[slice-605-search]] [[slice-606-theme-toggle]] [[slice-607-mobile-a11y]] [[slice-608-fence-static-deploy]]
- **Playground**: in-page runners stay out
- **Next.js**: Start only
- **cargo test --workspace**: not this oracle
- **Rewriting learn.md teaching copy**: keep the path; retarget `.html` links only if this hub must
- **generate.drac retirement**: stays until [[slice-608-fence-static-deploy]]

## Oracle checklist

- [x] O1: Learn hub route exists and the sidebar links Install through packages
  CHECK: pnpm --dir website test -- learn-hub-nav
  EXPECT: Test Files  1 passed
  EVIDENCE: Test Files  1 passed (1)

## Pool

Durable links to task ids. Never drop them.

- `[[task-620-learn-hub-nav]]`

## See also

[[location-589-public-site]] [[ticket-628-public-site-redesign]] [[0013-public-site-tanstack-start]] docs/specs/draconic/public-site/ website/ CONTEXT.md public-site.ia:learn-walkable
