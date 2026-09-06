---
id: "slice-593-typography-badge"
title: "Typography and Badge"
kind: slice
status: met
sprint: "website-redesign"
blocked_by: ["slice-592-semantic-tokens"]
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T23:09:00Z"
---

# Typography and Badge

## Why

A language site has three type jobs: display for the pitch, body for teaching prose, mono for fences and CLI. Mixing those in ad-hoc classes makes handbook pages look like a dump. Shipped versus not-yet is the same problem: generate.drac concatenates a badge string. Here it is a CVA variant on Badge (`shipped` and `not-yet`) in a `.variants.ts` file so status is one component, not a span per page.

## Done

Type scale utilities or `@theme` tokens exist for display, body, and mono. A Badge variants file exposes `shipped` and `not-yet`.

## Blocked by

`[[slice-592-semantic-tokens]]`: Badge and type must color from semantic roles, not a private palette.

## Non-goals

- **[[slice-594-site-header-nav]]**: putting Badge in the header
- **[[slice-599-markdown-render]]**: rendering status from markdown frontmatter
- Playground
- Next.js
- Inventing extra status labels beyond shipped and not-yet
- `cargo test --workspace` as this slice's oracle

## Oracle checklist

- [x] O1: display, body, and mono type roles exist and Badge has shipped and not-yet CVA variants
  CHECK: pnpm --dir website exec vitest run -t "typography and badge"
  EXPECT: Test Files  1 passed
  EVIDENCE: `pnpm --dir website exec vitest run -t "typography and badge"` → Test Files  1 passed | 2 skipped (3); Tests  1 passed | 2 skipped (3); commit b5e077f

## Pool

Durable links to task ids. Never drop them.

- `[[task-612-typography-badge]]`

## See also

- [[location-589-public-site]]
- [[ticket-628-public-site-redesign]]
- [[0013-public-site-tanstack-start]]
- docs/specs/draconic/public-site/
- website/
- CONTEXT.md
