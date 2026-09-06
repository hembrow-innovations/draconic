---
id: "task-640-docs-nav-groups"
title: "Group Learn and Reference in the side nav"
kind: task
status: ready
mode: afk
blocked_by: [ "task-637-site-shell" ]
sprint: "website-odm-match"
slice: "slice-633-docs-nav-groups"
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T18:00:00Z"
---

# Group Learn and Reference in the side nav

## Blocked by

[[task-637-site-shell]]: groups hang off the site side nav.

## Done

Side nav lists Learn pages and Reference pages in hub order with aria-current. Section sidebar plus badge still hold.

## Context

ODM has one left nav: labeled groups, current page `aria-current="page"`. Put Learn chapters (Install through packages, hub order) and Reference pages (CLI through packages, hub order) in those groups. Keep wordmark, Learn hub, Reference hub, and GitHub. If DocsShell aside would duplicate the same links, fold section nav into the site aside and update `docs-shell.test.ts` so the section sidebar is that aside. Do not change chapter order or CONTEXT terms.

## Verify

`pnpm --dir website exec vitest run learn-hub-nav reference-hub-pages docs-shell` passes. Typecheck holds.

scope: `website/src/components/SiteHeader/` or site shell nav, `website/src/features/learn/LearnNav/`, `website/src/features/reference/ReferenceNav/`, `website/src/features/docs/DocsShell/`, `website/src/tests/learn-hub-nav.test.ts`, `website/src/tests/reference-hub-pages.test.ts`, `website/src/tests/docs-shell.test.ts`

## Links

[[slice-633-docs-nav-groups]] [[task-637-site-shell]]

## Agent Brief

**Category:** enhancement
**Summary:** Make docs navigation one grouped ODM side nav.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.nav:learn-reference-status`, `public-site.chrome:docs-sidebar`, `public-site.ia:learn-walkable`, `public-site.ia:reference-walkable`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]

**Current behavior:**
Top or side chrome has Learn/Reference hubs only. Chapter lists live in DocsShell aside.

**Desired behavior:**
Site side nav groups: Home, Learn pages in hub order, Reference pages in hub order, GitHub. Current page aria-current. Badge still on the article. Tests still find section nav and status.

**Acceptance criteria:**
- [ ] Learn sequence Install through packages in the side nav
- [ ] Reference sequence CLI through packages in the side nav
- [ ] aria-current on the open page
- [ ] Named tests pass

**Out of scope:**
- Hub card grids; mobile wrap; changing IA order; playground
