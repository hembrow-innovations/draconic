---
id: "slice-600-docs-shell"
title: "Docs shell"
kind: slice
status: met
sprint: "website-redesign"
blocked_by:
  - slice-593-typography-badge
  - slice-599-markdown-render
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T00:50:29Z"
---
# Docs shell

## Why

A language handbook is not a single-column dump. TypeScript.org docs chrome puts a chapter list on the left, the article in the main column, and a shipped or not-yet cue on the page. Learn and Reference cannot feel like a handbook until an article is wrapped that way.

## Done

A docs shell wraps an article with aside plus main plus a status badge from the page `status` frontmatter.

## Blocked by

- [[slice-593-typography-badge]]: the badge CVA and type scale are what the shell shows, not a one-off label.
- [[slice-599-markdown-render]]: the main column is rendered markdown, not placeholder copy.

## Non-goals

- **Learn hub and chapters**: [[slice-601-learn-hub-nav]] [[slice-602-learn-pages]] [[slice-603-learn-prev-next]]
- **Reference pages**: [[slice-604-reference-hub-pages]]
- **Search, theme, mobile, fences**: [[slice-605-search]] [[slice-606-theme-toggle]] [[slice-607-mobile-a11y]] [[slice-608-fence-static-deploy]]
- **Playground**: in-page runners stay out
- **Next.js**: Start only
- **cargo test --workspace**: not this oracle
- **generate.drac retirement**: stays until [[slice-608-fence-static-deploy]]

## Oracle checklist
- [x] O1: a docs shell wraps an article with aside, main, and a status badge from status
  CHECK: pnpm --dir website test -- docs-shell
  EXPECT: Test Files  1 passed
  EVIDENCE: `pnpm --dir website test -- docs-shell` → Test Files  1 passed (1); commit 8955a33e

## Pool

Durable links to task ids. Never drop them.

- `[[task-619-docs-shell]]`

## See also

[[location-589-public-site]] [[ticket-628-public-site-redesign]] [[0013-public-site-tanstack-start]] docs/specs/draconic/public-site/ website/ CONTEXT.md public-site.chrome:docs-sidebar
