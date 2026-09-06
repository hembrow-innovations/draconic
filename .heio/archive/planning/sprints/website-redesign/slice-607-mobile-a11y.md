---
id: "slice-607-mobile-a11y"
title: "Mobile and keyboard a11y"
kind: slice
status: met
sprint: "website-redesign"
blocked_by:
  - slice-594-site-header-nav
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T14:24:36Z"
---

# Mobile and keyboard a11y

## Why

TypeScript.org is usable on a phone and from the keyboard. Primary nav that only works with a pointer, or focus that disappears, fails that bar. Small viewport, skip, and visible focus rings are how the public site stays a handbook on a phone.

## Done

Primary nav is usable at a small viewport. Skip and focus rings work.

## Blocked by

- [[slice-594-site-header-nav]]: mobile and keyboard behaviour is the header nav made usable, not a second nav.

## Non-goals

- **Docs shell, Learn, Reference, search, theme, fences**: [[slice-600-docs-shell]] [[slice-601-learn-hub-nav]] [[slice-604-reference-hub-pages]] [[slice-605-search]] [[slice-606-theme-toggle]] [[slice-608-fence-static-deploy]]
- **Playground**: in-page runners stay out
- **Next.js**: Start only
- **cargo test --workspace**: not this oracle
- **Pointer-only flyouts as the only path**: keyboard must work
- **generate.drac retirement**: stays until [[slice-608-fence-static-deploy]]

## Oracle checklist

- [x] O1: primary nav is usable at a small viewport; skip and focus rings work
  CHECK: pnpm --dir website test -- mobile-a11y
  EXPECT: Test Files  1 passed
  EVIDENCE: `pnpm --dir website test -- mobile-a11y` → Test Files  1 passed (1)

## Pool

Durable links to task ids. Never drop them.

- `[[task-626-mobile-a11y]]`

## See also

[[location-589-public-site]] [[ticket-628-public-site-redesign]] [[0013-public-site-tanstack-start]] docs/specs/draconic/public-site/ website/ CONTEXT.md public-site.a11y:keyboard-small
