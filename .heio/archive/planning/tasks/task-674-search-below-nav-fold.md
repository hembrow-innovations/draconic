---
id: "task-674-search-below-nav-fold"
title: "Keep search and theme toggle above the side-nav fold"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "website-odm-match"
slice: "slice-673-search-below-nav-fold"
area: public-site
tags: [ website, public-site ]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-08T12:30:00Z"
---
# Keep search and theme toggle above the side-nav fold

## Blocked by

None.

## Done

Search field and theme toggle stay in the sticky side nav and are visible without scrolling that aside at a 1280 by 720 desktop height.

## Context

Walked on `/cli`: aside scrollHeight 856, clientHeight 720, search top 724. Grouped Learn and Reference lists currently sit above SiteSearch and ThemeToggle in the primary nav cluster. Purpose keeps search in that nav. Do not drop search or the theme toggle. Do not unclip the result list here.

## Verify

`pnpm --dir website exec vitest run site-header-primary-nav search` prints `Test Files  2 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `website/src/components/SiteHeader/`, `website/src/components/SiteSearch/`, `website/src/components/ThemeToggle/`, `website/src/tests/site-header-primary-nav.test.ts`, `website/src/tests/search.test.ts`

## Links

[[slice-673-search-below-nav-fold]] [[ticket-646-search-below-nav-fold]]

## Agent Brief

**Category:** bug
**Summary:** Keep in-site search and the theme toggle visible in the sticky side nav without scrolling the aside.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-673-search-below-nav-fold]]; [[ticket-646-search-below-nav-fold]].

**TDD:** extend primary-nav or search tests first so SiteSearch and ThemeToggle are not ordered after the chapter lists. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.search:titles-headings`, `public-site.chrome:odm-shell`, `public-site.chrome:primary-nav`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: assert the existing promises in tests, then code. Do not drop search.

**Current behavior:**
On a 1280 by 720 walk of `/cli`, the sticky aside is taller than the viewport. Search sits below the fold and the theme toggle follows it. Wordmark, tagline, and grouped chapter lists fill the pane first.

**Desired behavior:**
At that desktop height, the search field and theme toggle sit inside the sticky aside's visible box without scrolling the aside. They remain in the side nav on every page. Chapter lists may scroll. Hub links, wordmark, and GitHub stay.

**Key interfaces:**
- SiteHeader primary nav cluster order
- SiteSearch and ThemeToggle as children of that nav
- Sticky aside overflow chrome already on the shell

**Acceptance criteria:**
- [x] Tests fail if SiteSearch and ThemeToggle are ordered after LearnNav and ReferenceNav
- [x] Search and theme toggle still render in the side nav
- [x] Named vitest files pass and typecheck exits 0
- [x] Promise ids listed above still hold

**Out of scope:**
- Result-list clipping ([[task-676-search-results-clipped]]); playground; vault-as-site; dropping search

**Explain this part:**
Purpose already keeps search in the side nav. This sitting only makes that field reachable at a normal desktop height.

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run site-header-primary-nav search` — win. Test Files  2 passed. Typecheck exits 0.
