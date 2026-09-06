---
id: "task-616-home-features"
title: "Add home feature grid"
kind: task
status: completed
mode: afk
blocked_by: ["task-615-home-hero-cta"]
sprint: "website-redesign"
slice: "slice-597-home-features"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T13:45:30Z"
---

# Add home feature grid

## Blocked by

[[task-615-home-hero-cta]]: the feature grid belongs on the home route after the hero exists.

## Done

Home shows three facts only: compiles to JavaScript; compiles to native via LLVM; Dual worlds are JS values and native types at explicit boundaries.

## Context

Home must not grow a marketing kitchen sink. Three facts, CONTEXT.md words. Dual worlds is the glossary term: coexistence of JS values and native types in one Program at explicit boundaries. Avoid “FFI-only” or “just typed JS.”

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

Home feature grid has those three facts and no extra product claims. Typecheck holds.

scope: home feature grid only under `website/src/`

## Links

[[slice-597-home-features]] [[task-615-home-hero-cta]]

## Agent Brief

**Category:** enhancement
**Summary:** Add a three-fact grid on home using CONTEXT.md wording.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none beyond home landing already named on [[task-615-home-hero-cta]] (`public-site.home:landing`)
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
Hero sitting delivers pitch and CTAs. There is no feature grid.

**Desired behavior:**
Exactly three facts on home:
- Compiles to JavaScript
- Compiles to native via LLVM
- Dual worlds are JS values and native types at explicit boundaries

Use CONTEXT.md words. Do not add a playground teaser, a fourth card, or rewritten glossary.

**Key interfaces:**
- Home route feature grid
- Tokens for surface/ink; no hex in JSX
- Load **frontend-development** (`token-semantic-roles`, `avoid-max-w-named`)

**Acceptance criteria:**
- [x] Three facts only, with CONTEXT.md Dual worlds wording
- [x] No playground, vault-as-site, or extra backends claimed
- [x] Grid lives on the home route only
- [x] If [[task-615-home-hero-cta]] is not `completed`, stop

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Markdown loader, Learn pages, search
- Changing hero CTAs

**Explain this part:**
The grid is three facts, not a feature catalog. Compiles to JavaScript; compiles to native via LLVM; Dual worlds are JS values and native types at explicit boundaries. Those are the product. Extra cards would invent a story CONTEXT.md does not tell.
