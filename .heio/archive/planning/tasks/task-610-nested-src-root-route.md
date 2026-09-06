---
id: "task-610-nested-src-root-route"
title: "Nest src and add the root route"
kind: task
status: completed
mode: afk
blocked_by: ["task-609-scaffold-start-app"]
sprint: "website-redesign"
slice: "slice-591-nested-src-root-route"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T12:51:00Z"
---

# Nest src and add the root route

## Blocked by

[[task-609-scaffold-start-app]]: the Start app must exist before routes and folders can be nested.

## Done

`website/src/` is nested folders (routes, components, styles) with a file-route root, not a flat `src/` dump.

## Context

A Start app that dumps every file at `website/src/` is unnavigable. Nested `src/` is the frontend skill bar: routes, components, and styles live in named folders. File routes map a URL to a file. Tokens and chrome wait for later sittings.

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

Root file route typechecks. `website/src/` has named subfolders. No flat dump of screens next to entries.

scope: `website/src/` only plus the generated route tree

## Links

[[slice-591-nested-src-root-route]] [[task-609-scaffold-start-app]]

## Agent Brief

**Category:** scaffolding
**Summary:** Put Start sources in nested folders and add the root file route.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none (layout; no new product promise)
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
Scaffold from [[task-609-scaffold-start-app]] may still be a Start default tree. Flat `src/` is forbidden.

**Desired behavior:**
`website/src/routes/` owns the file-route tree including the root. Components and styles live in folders, not as siblings of the entry files. The router plugin owns `routeTree.gen.ts`. Do not hand-edit that generated tree.

**Key interfaces:**
- TanStack file routes (`createFileRoute` / root route)
- Nested folders under `website/src/` (routes, components, styles)
- Generated route tree
- Load **frontend-development** (`arch-src-nested`, `start-file-routes`)

**Acceptance criteria:**
- [x] `website/src/` uses named subfolders; no flat `src/` dump
- [x] A root file route exists and typechecks
- [x] Generated route tree is not hand-edited
- [x] No theme tokens, header, or page copy in this sitting
- [x] If [[task-609-scaffold-start-app]] is not `completed`, stop

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Semantic tokens ([[task-611-semantic-tokens]])
- Teaching markdown rewrites
- Deleting `website/generate.drac`

**Explain this part:**
Nested `src/` keeps routes, components, and styles in folders so a fresh agent can find the seam. File routes map a URL to a file in that tree. A flat `src/` with `App.tsx` next to `Button.tsx` is forbidden because it hides the product shape.
