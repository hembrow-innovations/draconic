---
id: "task-676-search-results-clipped"
title: "Stop clipping in-site search results"
kind: task
status: completed
mode: afk
blocked_by:
  - task-674-search-below-nav-fold
sprint: "website-odm-match"
slice: "slice-675-search-results-clipped"
area: public-site
tags: [ website, public-site ]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-07T23:15:00Z"
---
# Stop clipping in-site search results

## Blocked by

[[task-674-search-below-nav-fold]]: pin the field first.

## Done

A Dual worlds query shows its `/dual-worlds` hit without the sticky aside clipping the result list.

## Context

Result list uses absolute positioning inside an aside with overflow auto. At 1280 by 720 the list box sits below the aside bottom. Same clip at 375 by 812. Distinct from the field sitting below the fold. Index behavior (title and heading hits, body-only miss) stays.

## Verify

`pnpm --dir website exec vitest run search` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `website/src/components/SiteSearch/`, `website/src/components/SiteHeader/`, `website/src/tests/search.test.ts`

## Links

[[slice-675-search-results-clipped]] [[ticket-647-search-results-clipped]] [[task-674-search-below-nav-fold]]

## Agent Brief

**Category:** bug
**Summary:** Make search hits visible and activatable instead of clipping them inside the sticky aside.

**Drain:** `/afk-task`. If [[task-674-search-below-nav-fold]] is not `completed`, stop. Do not claim.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-675-search-results-clipped]]; [[ticket-647-search-results-clipped]].

**TDD:** extend search tests first so the result list is not an absolutely positioned box trapped in overflowing aside clip. Dual worlds must still hit `/dual-worlds`. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.search:titles-headings`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: assert the existing promise, then test, then code

**Current behavior:**
Typing Dual worlds produces a matching `/dual-worlds` link, but the list is `position: absolute` inside `overflow: auto` aside chrome, so the hit is clipped at desktop and small viewports.

**Desired behavior:**
The matching result is visible and keyboard-activatable at 1280 by 720 and 375 by 812. Search stays in the side nav. Title and heading index behavior is unchanged. Body-only and vault phrases still miss.

**Key interfaces:**
- SiteSearch result list variants
- Sticky SiteHeader aside overflow
- `querySearchIndex` / Dual worlds hit

**Acceptance criteria:**
- [x] Tests fail if the result list is absolutely positioned inside overflowing aside clip
- [x] Dual worlds still resolves to `/dual-worlds`
- [x] Named vitest file passes and typecheck exits 0
- [x] `public-site.search:titles-headings` still holds

**Out of scope:**
- Reordering search above the fold (previous task); vault search; playground

**Explain this part:**
The promise is find-by-title-or-heading. A clipped hit is a miss for the visitor.

## Gauntlet
- **Round 1**: `pnpm --dir website exec vitest run search` — win. Test Files  1 passed. Typecheck exits 0.
