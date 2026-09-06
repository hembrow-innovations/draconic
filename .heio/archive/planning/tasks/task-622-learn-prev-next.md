---
id: "task-622-learn-prev-next"
title: "Add Learn prev and next"
kind: task
status: completed
mode: afk
blocked_by: ["task-621-learn-pages"]
sprint: "website-redesign"
slice: "slice-603-learn-prev-next"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T16:30:00Z"
---

# Add Learn prev and next

## Blocked by

[[task-621-learn-pages]]: prev/next needs real chapter routes to point at.

## Done

Each Learn chapter offers prev/next that follow `learn.md`. from-javascript and from-systems both continue to Dual worlds.

## Context

Sequence is the product. People should not have to return to the hub after every chapter. Alternate landings (from JavaScript, from systems) both join Dual worlds, then the path is one sequence.

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

Prev/next on Learn chapters follow hub order. Both landings next to Dual worlds. Typecheck holds.

scope: Learn prev/next controls and sequence helper under `website/src/`

## Links

[[slice-603-learn-prev-next]] [[task-621-learn-pages]]

## Agent Brief

**Category:** enhancement
**Summary:** Add prev/next on Learn chapters following `learn.md`, with both landings joining Dual worlds.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.ia:learn-walkable` (sequence)
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
Chapters are routes. There is no prev/next. Old HTML had no product-quality pager.

**Desired behavior:**
Prev/next follows `website/learn.md`. Install leads into the landings. from-javascript and from-systems are alternate landings that both continue to Dual worlds. After Dual worlds: modules, native types, host I/O, packages. Do not linearize the two landings as if one precedes the other as required reading.

**Key interfaces:**
- Learn sequence derived from hub order
- Prev/next chrome in the docs article footer
- Special case: both landings next → Dual worlds
- Load **frontend-development** (`start-file-routes`)

**Acceptance criteria:**
- [x] Prev/next follows `learn.md` sequence
- [x] from-javascript and from-systems both continue to Dual worlds
- [x] No invented extra stops
- [x] If [[task-621-learn-pages]] is not `completed`, stop

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Reference prev/next
- Rewriting chapter markdown
- Search ([[task-624-search]])

**Explain this part:**
Sequence is the product. Prev/next follows `learn.md` so the path can be walked without the hub. from-javascript and from-systems are alternate landings that both continue to Dual worlds; they are not two required chapters in a forced order.

## Gauntlet

- **Round 1**: `pnpm --dir website test -- learn-prev-next` — win. `Test Files  1 passed`. Typecheck holds.
