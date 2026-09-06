---
id: "slice-592-semantic-tokens"
title: "Semantic tokens"
kind: slice
status: met
sprint: "website-redesign"
blocked_by: [ "slice-591-nested-src-root-route" ]
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T13:01:17Z"
---
# Semantic tokens

## Why

A language homepage should look like TypeScript.org: canvas, ink, and accent as roles, not a rainbow of `bg-blue-500` or hex in components. Tailwind v4 reads those roles from `@theme` in CSS. Components then use token classes (`bg-canvas`, `text-ink`, `text-accent`) so light and dark and brand stay one source. Raw palette utilities drift the minute someone restyles a heading.

## Done

Tailwind v4 `@theme` tokens exist for semantic roles including canvas, ink, and accent. A rendered page uses those token classes, not raw palette utilities or hardcoded hex.

## Blocked by

`[[slice-591-nested-src-root-route]]`: a nested page must exist before it can consume token classes.

## Non-goals

- **[[slice-593-typography-badge]]**: type scale and Badge variants
- **[[slice-594-site-header-nav]]**: primary nav
- Playground
- Next.js
- A second palette or Tailwind v3 `theme.extend`
- `cargo test --workspace` as this slice's oracle

## Oracle checklist

- [x] O1: `@theme` defines canvas, ink, and accent and a page uses those classes not raw palette utilities
  CHECK: pnpm --dir website exec vitest run -t "semantic tokens"
  EXPECT: Test Files  1 passed
  EVIDENCE: met Test Files 1 passed | 1 skipped (2); theme.css @theme canvas/ink/accent; pages use bg-canvas text-ink; commit 4f7d0ed at=2026-09-06T13:01:17Z

## Pool

Durable links to task ids. Never drop them.

- `[[task-611-semantic-tokens]]`

## See also

- [[location-589-public-site]]
- [[ticket-628-public-site-redesign]]
- [[0013-public-site-tanstack-start]]
- docs/specs/draconic/public-site/
- website/
- CONTEXT.md
