---
id: "task-617-markdown-loader"
title: "Add markdown content loader"
kind: task
status: completed
mode: afk
blocked_by: [ "task-610-nested-src-root-route" ]
sprint: "website-redesign"
slice: "slice-598-markdown-loader"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T00:30:00Z"
---
# Add markdown content loader

## Blocked by

[[task-610-nested-src-root-route]]: the loader module belongs in nested `website/src/`, not a flat dump.

## Done

A content loader reads `website/*.md` frontmatter (`title`, `section`, `status`) and can load `install.md` at minimum.

## Context

Teaching source is `website/*.md` with `title`, `section`, and `status`. `generate.drac` hardcodes a page catalog. The Start app must load files, not treat that Program’s list as the forever catalog. Rendering the subset is [[task-618-markdown-render]].

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

Loader module reads `website/install.md` including `title`, `section`, `status`. Typecheck holds.

scope: content loader module under `website/src/`

## Links

[[slice-598-markdown-loader]] [[task-610-nested-src-root-route]]

## Agent Brief

**Category:** enhancement
**Summary:** Load public markdown from `website/*.md` instead of hardcoding the generate.drac catalog.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none (loader seam; subset render is [[task-618-markdown-render]])
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
`website/generate.drac` lists pages in Program source. The vault is not the site. There is no Start loader.

**Desired behavior:**
A module under nested `website/src/` loads markdown files from `website/`. Frontmatter fields are `title`, `section`, `status`. Prove it with `install.md` at minimum. Do not read `docs/` as public pages. Do not hardcode the generate.drac page list as the forever catalog.

**Key interfaces:**
- Content loader module (deep module: small load API, parsing inside)
- Frontmatter: `title`, `section`, `status`
- `website/install.md` as the first fixture
- Load **frontend-development** (`arch-src-nested`, `arch-deep-modules`) and **spec**

**Acceptance criteria:**
- [x] Loader reads `website/install.md` and exposes title, section, status, body
- [x] Catalog is derived from markdown files, not copied from `generate.drac`
- [x] Does not load `docs/` vault notes as site pages
- [x] Does not rewrite teaching markdown
- [x] If [[task-610-nested-src-root-route]] is not `completed`, stop

## Gauntlet

- **Round 1:** `pnpm --dir website exec vitest run -t "markdown loader install.md"` plus full `markdown-loader.test.ts` plus `pnpm --dir website typecheck` — win. Critic: nested `src/lib/content` seam, catalog from `*.md` not generate.drac, install.md title/section/status/body, no docs/ vault pages, no HTML render or playground.

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- HTML rendering of the markdown subset ([[task-618-markdown-render]])
- Learn or Reference routes

**Explain this part:**
Teaching source is `website/*.md` with title, section, and status. Load those files. Do not hardcode the `generate.drac` page catalog as the forever list. When a chapter is added as markdown, the loader should be able to see it without editing a hand-maintained array copied from the old Program.
