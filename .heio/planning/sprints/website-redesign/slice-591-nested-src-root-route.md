---
id: "slice-591-nested-src-root-route"
title: "Nested src and root route"
kind: slice
status: met
sprint: "website-redesign"
blocked_by: ["slice-590-scaffold-start-app"]
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T22:52:30Z"
---

# Nested src and root route

## Why

A Start scaffold is not yet a navigable app. Nested `src/` puts routes, components, and libs in named folders so a fresh agent can find the `/` page without a flat dump of files at `src/` root. File routes own the URL tree: `createFileRoute` in the discovered `routes/` folder, not a Next.js `app/` tree and not a hand-rolled router. The teaching point is layout as IA for the codebase, the same way Learn and Reference are IA for the language.

## Done

The `/` route renders from nested `src/` (routes under `src/routes/`, other UI not dumped at `src/` root). No flat `src/` dump.

## Blocked by

`[[slice-590-scaffold-start-app]]`: the Start app must exist before routes can live in nested `src/`.

## Non-goals

- **[[slice-590-scaffold-start-app]]**: package.json and typecheck already owned
- **[[slice-592-semantic-tokens]]**: `@theme` roles
- **[[slice-594-site-header-nav]]** and [[slice-595-site-footer-skip]]: chrome
- **[[slice-596-home-hero-cta]]**: homepage pitch
- Playground
- Next.js `app/` routes
- `cargo test --workspace` as this slice's oracle

## Oracle checklist

- [x] O1: `/` renders from nested `src/routes` and `src/` is not a flat dump
  CHECK: pnpm --dir website exec vitest run -t "root route from nested src"
  EXPECT: Test Files  1 passed
  EVIDENCE: Test Files  1 passed (1); website/src/routes/index.tsx createFileRoute("/"); commit 4edf3ad20bed7157fe9c3d432d25ef691d460dc2 at=2026-09-06T22:52:30Z

## Pool

Durable links to task ids. Never drop them.

- `[[task-610-nested-src-root-route]]`

## See also

- [[location-589-public-site]]
- [[ticket-628-public-site-redesign]]
- [[0013-public-site-tanstack-start]]
- docs/specs/draconic/public-site/
- website/
- CONTEXT.md
