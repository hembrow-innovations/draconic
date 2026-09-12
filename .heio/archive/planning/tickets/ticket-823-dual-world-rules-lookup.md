---
id: "ticket-823-dual-world-rules-lookup"
title: "Dual-world rules Reference is a stub while writing a Program"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T11:05:00Z"
updated_at: "2026-09-12T11:10:00Z"
---

# Dual-world rules Reference is a stub while writing a Program

## Signal

Closed. Website swarm shipped `website/dual-world-rules.md` as a working lookup: compiling `as` and `try`/`catch` fences, plus `draconic check`. Hub and public-docs status list mark Dual-world rules shipped. Remaining not-yet pages stay prose-only.

## Fit

In scope of `public-site.ia:reference-walkable`. Shipped fences must build.

## Notes

- Dual-world rules working page: `website/dual-world-rules.md`
- Hub: `website/reference.md`
- Lock: `website/src/tests/reference-hub-pages.test.ts`
- Do not add fences on remaining not-yet pages.
- Do not invent Reference chapters.

## Parent

[[Public site — Contract]]

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run reference-hub-pages search` — win. `Test Files  2 passed (2)`. `pnpm --dir website typecheck` exit 0. Both `drac` fences `draconic build --target js` and `draconic check`.
