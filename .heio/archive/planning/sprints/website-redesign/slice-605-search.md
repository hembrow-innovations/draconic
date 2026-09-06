---
id: "slice-605-search"
title: "Search titles and headings"
kind: slice
status: met
sprint: "website-redesign"
blocked_by:
  - slice-602-learn-pages
  - slice-604-reference-hub-pages
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T16:45:00Z"
---

# Search titles and headings

## Why

A language site that cannot find Dual worlds is a brochure. This cut indexes titles and headings only, enough to land on the Learn chapter, not a full-text fuzz index.

## Done

A visitor can search Dual worlds and land on the Learn Dual worlds chapter.

## Blocked by

- [[slice-602-learn-pages]]: Dual worlds must be a real Learn chapter route before search can land there.
- [[slice-604-reference-hub-pages]]: Reference titles and headings are in the same find-in-docs set.

## Non-goals

- **Full-text fuzz or body search**: titles and headings only
- **Prev/next, theme, mobile, fences**: [[slice-603-learn-prev-next]] [[slice-606-theme-toggle]] [[slice-607-mobile-a11y]] [[slice-608-fence-static-deploy]]
- **Playground**: in-page runners stay out
- **Next.js**: Start only
- **cargo test --workspace**: not this oracle
- **generate.drac retirement**: stays until [[slice-608-fence-static-deploy]]

## Oracle checklist

- [x] O1: searching Dual worlds lands on the Learn Dual worlds chapter
  CHECK: pnpm --dir website test -- search
  EXPECT: Test Files  1 passed
  EVIDENCE: 2026-09-07 `pnpm --dir website test -- search` → Test Files 1 passed (1); Tests 1 passed (1); Dual worlds query reaches Learn Dual worlds; index is titles and headings only.

## Pool

Durable links to task ids. Never drop them.

- `[[task-624-search]]`

## See also

[[location-589-public-site]] [[ticket-628-public-site-redesign]] [[0013-public-site-tanstack-start]] docs/specs/draconic/public-site/ website/ CONTEXT.md public-site.search:titles-headings
