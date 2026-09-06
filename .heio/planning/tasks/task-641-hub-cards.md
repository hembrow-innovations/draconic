---
id: "task-641-hub-cards"
title: "Card-grid Learn and Reference hubs"
kind: task
status: ready
mode: afk
blocked_by: [ "task-639-docs-article-odm" ]
sprint: "website-odm-match"
slice: "slice-634-hub-cards"
area: public-site
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T19:30:00Z"
---

# Card-grid Learn and Reference hubs

## Blocked by

[[task-639-docs-article-odm]]: cards sit in the article column.

## Done

`/learn` and `/reference` show a card grid of their pages and still load hub markdown.

## Context

ODM hubs use `div.grid` of `div.card` with linked h3s. Learn hub and Reference hub should add that grid of existing chapter/working-page links. Keep loading `learn.md` and `reference.md`. Do not turn `/` into a hub. Do not add playground.

## Verify

`pnpm --dir website exec vitest run learn-hub-nav reference-hub-pages` passes. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `website/src/routes/learn.tsx`, `website/src/routes/reference.tsx`, `website/src/features/learn/`, `website/src/features/reference/`, `website/src/tests/learn-hub-nav.test.ts`, `website/src/tests/reference-hub-pages.test.ts`

## Links

[[slice-634-hub-cards]] [[task-639-docs-article-odm]]

## Agent Brief

**Category:** enhancement
**Summary:** Present Learn and Reference hubs as ODM card grids.

**Drain:** `/afk-task`. If [[task-639-docs-article-odm]] is not `completed`, stop. Do not claim.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**. Public site is `website/` TanStack Start. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-634-hub-cards]].

**TDD:** extend hub tests first so Learn and Reference hubs render a card grid of existing links. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.ia:learn-walkable`, `public-site.ia:reference-walkable`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not turn `/` into a hub

**Current behavior:**
Hubs load markdown in DocsShell plus a text nav list.

**Desired behavior:**
Same hubs, plus a card grid of the same links. Still not the home landing.

**Key interfaces:**
- Learn and Reference hub routes
- Card grid CVA; existing chapter and working-page hrefs

**Acceptance criteria:**
- [ ] `/learn` and `/reference` render cards to each listed page
- [ ] Hub markdown still loads
- [ ] Named tests pass and typecheck exits 0

**Out of scope:**
- Home cards; changing chapter set; playground

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run learn-hub-nav reference-hub-pages` — win. `Test Files  2 passed`. Typecheck holds.
