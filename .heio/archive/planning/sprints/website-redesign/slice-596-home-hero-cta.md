---
id: "slice-596-home-hero-cta"
title: "Home hero and CTA"
kind: slice
status: met
sprint: "website-redesign"
blocked_by: ["slice-595-site-footer-skip"]
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T13:39:00Z"
---

# Home hero and CTA

## Why

TypeScript.org opens with a pitch and Get started, not the handbook dumped on `/`. Draconic should do the same. `website/learn.md` already has the pitch: JavaScript you already know, native types when you need them, one language two backends. CONTEXT.md names Learn as the public path, not a beginner course. `/` is a landing with that language plus CTAs to Install and Learn. It is not `Learn.md` rendered as the index. Promise `public-site.home:landing`.

## Done

`/` hero uses Learn pitch language from CONTEXT.md and `website/learn.md`. CTAs go to Install and Learn.

## Blocked by

`[[slice-595-site-footer-skip]]`: hero sits in `main` after skip and footer chrome exist.

## Non-goals

- **[[slice-597-home-features]]**: the three value-prop cards
- **[[slice-598-markdown-loader]]**: loading `learn.md` as a doc page
- Dumping `website/learn.md` as the index body
- Playground
- Next.js
- `cargo test --workspace` as this slice's oracle

## Oracle checklist

- [x] O1: `/` hero uses Learn pitch language and links to Install and Learn
  CHECK: pnpm --dir website exec vitest run -t "home hero and cta"
  EXPECT: Test Files  1 passed
  EVIDENCE: `pnpm --dir website exec vitest run -t "home hero and cta"` → Test Files 1 passed; `pnpm --dir website typecheck` green

## Pool

Durable links to task ids. Never drop them.

- `[[task-615-home-hero-cta]]`

## See also

- [[location-589-public-site]]
- [[ticket-628-public-site-redesign]]
- [[0013-public-site-tanstack-start]]
- docs/specs/draconic/public-site/
- website/
- CONTEXT.md
- Promise: public-site.home:landing
