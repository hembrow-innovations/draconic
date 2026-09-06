---
id: "slice-606-theme-toggle"
title: "Theme toggle"
kind: slice
status: met
sprint: "website-redesign"
blocked_by:
  - slice-592-semantic-tokens
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T00:14:32Z"
---
# Theme toggle

## Why

Light and dark must ride the semantic tokens already named for the site. A second palette of hex, or components that hardcode hex, splits the theme and fights TypeScript.org-quality chrome.

## Done

A toggle switches the token-backed theme. Components do not hardcode hex.

## Blocked by

- [[slice-592-semantic-tokens]]: there is nothing honest to toggle until roles live in Tailwind v4 `@theme`.

## Non-goals

- **Docs shell, Learn, Reference, search, mobile, fences**: [[slice-600-docs-shell]] [[slice-601-learn-hub-nav]] [[slice-605-search]] [[slice-607-mobile-a11y]] [[slice-608-fence-static-deploy]]
- **A second hex palette**: tokens only
- **Playground**: in-page runners stay out
- **Next.js**: Start only
- **cargo test --workspace**: not this oracle
- **generate.drac retirement**: stays until [[slice-608-fence-static-deploy]]

## Oracle checklist
- [x] O1: toggle switches the token-backed theme and components do not hardcode hex
  CHECK: pnpm --dir website test -- theme-toggle
  EXPECT: Test Files  1 passed
  EVIDENCE: `pnpm --dir website test -- theme-toggle` → Test Files  1 passed (1)

## Pool

Durable links to task ids. Never drop them.

- `[[task-625-theme-toggle]]`

## See also

[[location-589-public-site]] [[ticket-628-public-site-redesign]] [[0013-public-site-tanstack-start]] docs/specs/draconic/public-site/ website/ CONTEXT.md
