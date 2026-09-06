---
id: "task-640-docs-nav-groups"
title: "Group Learn and Reference in the side nav"
kind: task
status: ready
mode: afk
blocked_by: [ "task-637-site-shell" ]
sprint: "website-odm-match"
slice: "slice-633-docs-nav-groups"
area: public-site
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T19:30:00Z"
---

# Group Learn and Reference in the side nav

## Blocked by

[[task-637-site-shell]]: groups hang off the site side nav.

## Done

Side nav lists Learn pages and Reference pages in hub order with aria-current. Section sidebar plus badge still hold.

## Context

ODM has one left nav: labeled groups, current page `aria-current="page"`. Put Learn chapters (Install through packages, hub order) and Reference pages (CLI through packages, hub order) in those groups. Keep wordmark, Learn hub, Reference hub, and GitHub. If DocsShell aside would duplicate the same links, fold section nav into the site aside and update `docs-shell.test.ts` so the section sidebar is that aside. Do not change chapter order or CONTEXT terms.

## Verify

`pnpm --dir website exec vitest run learn-hub-nav reference-hub-pages docs-shell` passes. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `website/src/components/SiteHeader/` or site shell nav, `website/src/features/learn/LearnNav/`, `website/src/features/reference/ReferenceNav/`, `website/src/features/docs/DocsShell/`, `website/src/tests/learn-hub-nav.test.ts`, `website/src/tests/reference-hub-pages.test.ts`, `website/src/tests/docs-shell.test.ts`

## Links

[[slice-633-docs-nav-groups]] [[task-637-site-shell]]

## Agent Brief

**Category:** enhancement
**Summary:** Make docs navigation one grouped ODM side nav.

**Drain:** `/afk-task`. If [[task-637-site-shell]] is not `completed`, stop. Do not claim.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**. Public site is `website/` TanStack Start. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-633-docs-nav-groups]].

**TDD:** extend learn-hub-nav, reference-hub-pages, and docs-shell tests first so section links live in the site side nav with aria-current. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.nav:learn-reference-status`, `public-site.chrome:docs-sidebar`, `public-site.ia:learn-walkable`, `public-site.ia:reference-walkable`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not change Learn or Reference chapter order

**Current behavior:**
Top or side chrome has Learn/Reference hubs only. Chapter lists live in DocsShell aside.

**Desired behavior:**
Site side nav groups: Home, Learn pages in hub order, Reference pages in hub order, GitHub. Current page aria-current. Badge still on the article. Tests still find section nav and status.

**Key interfaces:**
- Site side nav groups, LearnNav, ReferenceNav, DocsShell aside
- Learn and Reference hub order unchanged

**Acceptance criteria:**
- [ ] Learn sequence Install through packages in the side nav
- [ ] Reference sequence CLI through packages in the side nav
- [ ] aria-current on the open page
- [ ] Named tests pass and typecheck exits 0

**Out of scope:**
- Hub card grids; mobile wrap; changing IA order; playground

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run learn-hub-nav reference-hub-pages docs-shell` — win. `Test Files  3 passed`. Typecheck holds.
