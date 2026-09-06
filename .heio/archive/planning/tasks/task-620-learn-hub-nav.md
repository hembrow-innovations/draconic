---
id: "task-620-learn-hub-nav"
title: "Add Learn hub and chapter nav"
kind: task
status: completed
mode: afk
blocked_by: [ "task-619-docs-shell" ]
sprint: "website-redesign"
slice: "slice-601-learn-hub-nav"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T12:00:00Z"
---
# Add Learn hub and chapter nav

## Blocked by

[[task-619-docs-shell]]: Learn nav belongs in the docs aside, not a new layout.

## Done

Learn hub (`learn.md`) is walkable and the aside lists Install, from JavaScript or from systems, Dual worlds, then modules, native types, host I/O, packages.

## Context

Learn is the public path, not a beginner course and not “the docs.” Hub source is `website/learn.md`. Chapter routes themselves are [[task-621-learn-pages]]. This sitting makes the hub and the aside sequence real.

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

Learn hub renders in the docs shell. Aside order matches `website/learn.md`. Typecheck holds.

scope: Learn hub route plus Learn aside nav under `website/src/`

## Links

[[slice-601-learn-hub-nav]] [[task-619-docs-shell]]

## Agent Brief

**Category:** enhancement
**Summary:** Put the Learn hub in the docs shell and list the Learn path in the aside.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.ia:learn-walkable`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
`generate.drac` hardcodes a Learn nav of `.html` links. Start has a docs shell with no Learn IA.

**Desired behavior:**
Hub is `website/learn.md`. Aside order: Install; from JavaScript; from systems; Dual worlds; modules; native types; host I/O; packages. from-javascript and from-systems are alternate landings. Do not invent extra chapters. Do not use vault notes as Learn.

**Key interfaces:**
- Learn hub file route
- Learn aside sequence matching `website/learn.md`
- Loader + renderer already in place
- Load **frontend-development** (`start-file-routes`) and CONTEXT.md Learn entry

**Acceptance criteria:**
- [x] Learn hub renders `website/learn.md` in the docs shell
- [x] Aside lists the Learn path in hub order
- [x] No playground; no vault chapters
- [x] Teaching copy in `learn.md` is not rewritten
- [x] If [[task-619-docs-shell]] is not `completed`, stop

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- One route per chapter file ([[task-621-learn-pages]])
- Prev/next ([[task-622-learn-prev-next]])

**Explain this part:**
The Learn path is Install, then from JavaScript or from systems, joining at Dual worlds, then modules, native types, host I/O, packages. The hub is `learn.md`. That sequence is the product: two landings, one join, then one path.

## Gauntlet

- **Round 1**: `pnpm --dir website test -- learn-hub-nav` — win. `Test Files  1 passed`. Typecheck holds.
