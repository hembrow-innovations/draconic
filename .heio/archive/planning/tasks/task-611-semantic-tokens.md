---
id: "task-611-semantic-tokens"
title: "Add semantic theme tokens"
kind: task
status: completed
mode: afk
blocked_by: [ "task-610-nested-src-root-route" ]
sprint: "website-redesign"
slice: "slice-592-semantic-tokens"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T13:20:00Z"
---
# Add semantic theme tokens

## Blocked by

[[task-610-nested-src-root-route]]: theme CSS needs a nested styles home, not a flat `src/` dump.

## Done

Tailwind v4 `@theme` names roles such as canvas, ink, and accent so later components never say `bg-blue-500` or hardcoded hex.

## Context

The public site should feel like a language homepage (TypeScript.org-like navy/blue), but that look is tokens, not hex in JSX. Components consume role names. Light and dark later swap token sets ([[task-625-theme-toggle]]).

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

Theme CSS defines semantic roles. No page copy rewrite. Typecheck still holds.

scope: website theme CSS under nested `website/src/` (or the Start CSS entry the scaffold already uses)

## Links

[[slice-592-semantic-tokens]] [[task-610-nested-src-root-route]]

## Agent Brief

**Category:** enhancement
**Summary:** Define `@theme` role tokens so components never hardcode palette utilities.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none (presentation tokens; no new product promise)
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
`generate.drac` inlines CSS variables in a string. Components in the new app have no shared role tokens yet.

**Desired behavior:**
Tailwind v4 `@theme` exposes roles such as canvas, ink, accent, muted, and line. A TypeScript.org-like navy/blue look comes from those tokens. Components use `bg-canvas` / `text-ink` (or the role names you define), never `bg-blue-500` or `#1B1C19` in JSX. Do not add a v3 `theme.extend` palette.

**Key interfaces:**
- Start CSS entry with `@import "tailwindcss"` and `@theme`
- Semantic color roles (canvas, ink, accent, and matching foregrounds)
- Load **frontend-development** (`tw-v4-theme`, `token-semantic-roles`, `avoid-hardcoded-colors`)

**Acceptance criteria:**
- [x] `@theme` tokens name roles, not one-off hex in components
- [x] No `bg-blue-500` or hardcoded hex in component files
- [x] No named `max-w-sm` / `max-w-xl`
- [x] Page teaching copy is not rewritten
- [x] If [[task-610-nested-src-root-route]] is not `completed`, stop

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Typography scale and Badge ([[task-612-typography-badge]])
- Theme toggle persistence ([[task-625-theme-toggle]])
- Rewriting Learn or Reference markdown

**Explain this part:**
`@theme` tokens name roles (canvas, ink, accent) so a button says “paint me with accent,” not “paint me `#2563eb`.” That is how light and dark stay one component tree. Hex belongs in the token file, not in JSX.
