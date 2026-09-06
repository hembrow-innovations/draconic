---
id: "task-621-learn-pages"
title: "Add Learn chapter routes"
kind: task
status: completed
mode: afk
blocked_by: [ "task-620-learn-hub-nav" ]
sprint: "website-redesign"
slice: "slice-602-learn-pages"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T15:15:00Z"
---
# Add Learn chapter routes

## Blocked by

[[task-620-learn-hub-nav]]: chapter routes hang off the Learn hub and aside.

## Done

Each existing Learn markdown file has a route, teaching copy is kept, `.html` links become app routes, and shipped/not-yet comes from frontmatter.

## Context

Learn chapters already exist as markdown. Do not rewrite teaching copy. Fix in-body `.html` links to Start routes. Show Badge from frontmatter `status`. Files: `website/install.md`, `from-javascript.md`, `from-systems.md`, `dual-worlds.md`, `modules.md`, `native-types.md`, `host-io.md`, `packages.md`, `learn.md`.

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

Each listed Learn file is reachable as an app route with status Badge. Internal links are not `.html`. Typecheck holds.

scope: Learn chapter routes under `website/src/`; link hrefs inside the listed `website/*.md` Learn files only as needed to replace `.html`

## Links

[[slice-602-learn-pages]] [[task-620-learn-hub-nav]]

## Agent Brief

**Category:** enhancement
**Summary:** Add one route per existing Learn markdown file and keep the teaching copy.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.ia:learn-walkable` (chapter routes)
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
Hub and aside exist. Chapter bodies still live only as `website/*.md` / old `.html`. Links inside markdown still point at `install.html` and friends.

**Desired behavior:**
One route per Learn file listed in scope. Keep teaching copy. Replace `.html` links with app routes. Badge reflects frontmatter `status` (`shipped` or `not-yet`). Do not add playground. Do not create chapters that have no markdown file.

**Key interfaces:**
- File routes for each Learn chapter
- Loader + renderer + docs shell + Badge
- Link rewrite from `*.html` to Start paths
- Files: `website/install.md`, `from-javascript.md`, `from-systems.md`, `dual-worlds.md`, `modules.md`, `native-types.md`, `host-io.md`, `packages.md`, `learn.md`

**Acceptance criteria:**
- [x] One route per listed Learn markdown file
- [x] Teaching copy kept; only `.html` hrefs rewritten if needed
- [x] Status Badge matches frontmatter
- [x] No new chapters; no vault pages
- [x] If [[task-620-learn-hub-nav]] is not `completed`, stop

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Prev/next controls ([[task-622-learn-prev-next]])
- Reference pages ([[task-623-reference-hub-pages]])
- Rewriting teaching prose

**Explain this part:**
One route per existing Learn markdown file. The teaching copy is already the product. This sitting makes those files addressable in the app, fixes leftover `.html` links, and shows shipped or not-yet from frontmatter so visitors know what builds today.

## Gauntlet

- **Round 1**: `pnpm --dir website test -- learn-pages` — win. `Test Files  1 passed`. Typecheck holds.
