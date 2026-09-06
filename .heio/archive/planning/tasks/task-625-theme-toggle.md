---
id: "task-625-theme-toggle"
title: "Add theme toggle"
kind: task
status: completed
mode: afk
blocked_by: [ "task-611-semantic-tokens" ]
sprint: "website-redesign"
slice: "slice-606-theme-toggle"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T14:12:30Z"
---
# Add theme toggle

## Blocked by

[[task-611-semantic-tokens]]: light and dark are token sets; they cannot exist without role tokens.

## Done

A control switches light and dark by swapping `@theme` token sets, not a second hex palette in components.

## Context

Dark mode is the same components with a second token set. Persist only if you add a small schema. Do not invent accounts. Hex still does not belong in JSX.

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

Toggle switches canvas/ink (and related roles) between light and dark token sets. Components do not gain a second hex palette. Typecheck holds.

scope: theme token sets plus toggle control under `website/src/`

## Links

[[slice-606-theme-toggle]] [[task-611-semantic-tokens]]

## Agent Brief

**Category:** enhancement
**Summary:** Switch light and dark by swapping semantic token sets.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none named beyond presentation (do not invent a product promise)
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
Semantic tokens exist for one theme. There is no toggle. `generate.drac` had a single inlined palette.

**Desired behavior:**
Light and dark are two token sets for the same roles (canvas, ink, accent, and matching foregrounds). A toggle swaps the set. Persist only with a small local schema if you persist at all. Do not invent accounts. Do not duplicate components per theme.

**Key interfaces:**
- `@theme` plus a dark token set (class or scheme)
- Toggle control in site chrome
- Optional small localStorage schema (`client-localstorage-schema`)
- Load **frontend-development** (`tw-v4-theme`, `token-semantic-roles`, `quality-motion-theme`)

**Acceptance criteria:**
- [x] Dark/light swaps token sets, not a second hex palette in components
- [x] No accounts; persist only with a small schema if at all
- [x] No hardcoded hex in components
- [x] If [[task-611-semantic-tokens]] is not `completed`, stop

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Mobile nav ([[task-626-mobile-a11y]])
- Rewriting page copy

**Explain this part:**
Dark and light are token sets, not a second hex palette. The Badge, header, and article keep the same class names (`bg-canvas`, `text-ink`). Only the token values change. Persistence is optional and must not grow into accounts.

## Gauntlet

- **round 1**: `pnpm --dir website test -- theme-toggle` — win. `Test Files  1 passed (1)`. Typecheck `tsc --noEmit` clean. No product promise ids; presentation only.
