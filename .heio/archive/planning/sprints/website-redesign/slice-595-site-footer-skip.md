---
id: "slice-595-site-footer-skip"
title: "Site footer and skip link"
kind: slice
status: met
sprint: "website-redesign"
blocked_by: ["slice-594-site-header-nav"]
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T23:27:00Z"
---

# Site footer and skip link

## Why

Skip-to-content and a footer are accessibility chrome, not a visual afterthought. Keyboard and screen-reader users must jump past the primary nav to `main`. The skip target has to focus that main landmark, not merely exist as a hidden link. The footer closes the page with durable links so the header is not the only way out. This cut is the rest of the chrome shell before home content lands inside `main`.

## Done

A skip-to-content control focuses `main`. A site footer is present on `/`.

## Blocked by

`[[slice-594-site-header-nav]]`: skip must jump past the primary nav that this slice already put on `/`.

## Non-goals

- **[[slice-594-site-header-nav]]**: wordmark and primary nav already owned
- **[[slice-596-home-hero-cta]]**: hero copy
- Playground
- Next.js
- `cargo test --workspace` as this slice's oracle

## Oracle checklist

- [x] O1: skip-to-content focuses `main` and `/` has a footer
  CHECK: pnpm --dir website exec vitest run -t "site footer and skip link"
  EXPECT: Test Files  1 passed
  EVIDENCE: `pnpm --dir website exec vitest run -t "site footer and skip link"` → Test Files 1 passed; `pnpm --dir website typecheck` green

## Pool

Durable links to task ids. Never drop them.

- `[[task-614-site-footer-skip]]`

## See also

- [[location-589-public-site]]
- [[ticket-628-public-site-redesign]]
- [[0013-public-site-tanstack-start]]
- docs/specs/draconic/public-site/
- website/
- CONTEXT.md
