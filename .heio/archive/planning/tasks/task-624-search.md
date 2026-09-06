---
id: "task-624-search"
title: "Add title and heading search"
kind: task
status: completed
mode: afk
blocked_by: ["task-621-learn-pages", "task-623-reference-hub-pages"]
sprint: "website-redesign"
slice: "slice-605-search"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T16:30:00Z"
---

# Add title and heading search

## Blocked by

[[task-621-learn-pages]]: search must reach Learn chapter routes.
[[task-623-reference-hub-pages]]: search must reach Reference working pages.

## Done

Search indexes titles and headings only, and a query for Dual worlds reaches the Learn chapter.

## Context

Search is a finder, not a full-text engine and not a playground. Index titles and headings from loaded public markdown. Query “Dual worlds” must reach the Learn Dual worlds chapter.

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

Searching Dual worlds navigates to or lists the Learn Dual worlds page. Index is titles and headings only. Typecheck holds.

scope: search UI and title/heading index under `website/src/`

## Links

[[slice-605-search]] [[task-621-learn-pages]] [[task-623-reference-hub-pages]]

## Agent Brief

**Category:** enhancement
**Summary:** Index public titles and headings and make Dual worlds findable.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.search:titles-headings`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
Learn and Reference routes exist. There is no search. The old HTML dump had none.

**Desired behavior:**
Index titles and headings from public `website/*.md` pages that already have routes. Query Dual worlds reaches the Learn chapter. Do not index the vault. Do not invent accounts. Do not search full body text unless the contract already says titles and headings only — stay at titles and headings.

**Key interfaces:**
- Client search field in site chrome or docs chrome
- Static index of titles and headings
- Results link to existing Start routes
- Load **frontend-development** (`start-search-params` only if the repo already uses it)

**Acceptance criteria:**
- [x] Index is titles and headings only
- [x] Query Dual worlds reaches the Learn Dual worlds chapter
- [x] Vault `docs/` is not indexed
- [x] No playground results
- [x] If [[task-621-learn-pages]] or [[task-623-reference-hub-pages]] is not `completed`, stop

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Full-text body search
- Accounts or remote search services
- Theme toggle ([[task-625-theme-toggle]])

**Explain this part:**
Search indexes titles and headings only. That is enough to jump to a chapter without building a second corpus. A query for Dual worlds must reach the Learn chapter, because Dual worlds is the join in the public path.

## Gauntlet

- **Round 1**: `pnpm --dir website test -- search` — win. Gap: none.
