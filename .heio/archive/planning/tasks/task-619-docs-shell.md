---
id: "task-619-docs-shell"
title: "Add the docs shell"
kind: task
status: completed
mode: afk
blocked_by: [ "task-612-typography-badge", "task-618-markdown-render" ]
sprint: "website-redesign"
slice: "slice-600-docs-shell"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T18:00:00Z"
---
# Add the docs shell

## Blocked by

[[task-612-typography-badge]]: the shell shows shipped/not-yet with the Badge primitive.
[[task-618-markdown-render]]: the article pane renders the markdown subset.

## Done

Learn and Reference pages can sit in handbook chrome: aside nav, article, status badge — not the marketing home.

## Context

Home is a language homepage. Learn and Reference are a handbook: sidebar, article, status. This sitting builds that shell. Filling Learn links is [[task-620-learn-hub-nav]]; filling Reference is [[task-623-reference-hub-pages]].

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

A docs layout exists with aside, article, and Badge. It is not used as the home layout. Typecheck holds.

scope: docs shell layout and aside/article structure under `website/src/`

## Links

[[slice-600-docs-shell]] [[task-612-typography-badge]] [[task-618-markdown-render]]

## Agent Brief

**Category:** enhancement
**Summary:** Add handbook chrome with aside nav, article, and status Badge.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.chrome:docs-sidebar`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
`generate.drac` injects a second nav for Learn or Reference. Start has site header plus a markdown renderer, but no handbook shell.

**Desired behavior:**
Docs shell: aside for section nav, article for rendered markdown, Badge for `shipped` / `not-yet`. This is not the marketing home. Do not copy home hero into the shell.

**Key interfaces:**
- Pathless or nested layout for Learn/Reference
- Aside + article landmarks
- Badge from [[task-612-typography-badge]]
- Renderer from [[task-618-markdown-render]]
- Load **frontend-development** (`start-file-routes`, `package-component-layout`)

**Acceptance criteria:**
- [x] Docs shell has aside nav, article, and status Badge
- [x] Shell is handbook chrome, not the home hero
- [x] Uses tokens and Badge variants; no hex in JSX
- [x] If [[task-612-typography-badge]] or [[task-618-markdown-render]] is not `completed`, stop

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Wiring every Learn href ([[task-620-learn-hub-nav]])
- Wiring every Reference href ([[task-623-reference-hub-pages]])
- Home feature grid

**Explain this part:**
Handbook chrome is aside nav, article, and a status badge. That is how a chapter is read. It is not the marketing home. Home sells the language; the docs shell walks a path and keeps working pages open.

## Gauntlet

- **Round 1**: `pnpm --dir website test -- docs-shell` — win. `Test Files  1 passed`. Typecheck holds.
