---
id: "task-866-github-above-fold"
title: "Keep GitHub above the side-nav fold"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "website-chrome-polish"
slice: "slice-865-github-above-fold"
area: public-site
tags: [website, public-site]
created_at: "2026-09-12T07:05:00Z"
updated_at: "2026-09-12T07:05:00Z"
---

# Keep GitHub above the side-nav fold

## Blocked by

None.

## Done

At a 1280 by 720 desktop height, GitHub sits in the sticky aside's visible box without scrolling that aside.

## Context

Search and theme already sit above the chapter lists. GitHub still follows those lists, so at desktop height it is off-screen until the aside scrolls. Purpose keeps GitHub in the side nav. Do not drop it. Do not fix the tall mobile hit here.

## Verify

`pnpm --dir website exec vitest run site-header-primary-nav` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `website/src/components/SiteHeader/`, `website/src/tests/site-header-primary-nav.test.ts`

## Links

[[slice-865-github-above-fold]] [[ticket-848-github-below-nav-fold]]

## Agent Brief

**Category:** bug
**Summary:** Keep the GitHub control visible in the sticky side nav without scrolling the aside at a normal desktop height.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**, **website**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-865-github-above-fold]]; [[ticket-848-github-below-nav-fold]].

**TDD:** extend primary-nav tests first so GitHub is not ordered after LearnNav and ReferenceNav. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.chrome:primary-nav`, `public-site.chrome:odm-shell`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: assert the existing promises in tests, then code. Do not drop GitHub.

**Current behavior:**
GitHub follows the always-expanded Learn and Reference chapter lists. At 1280 by 720 the control sits below the aside fold. Search and theme already sit above those lists.

**Desired behavior:**
At that desktop height, GitHub sits inside the sticky aside's visible box without scrolling the aside. It remains in the side nav on every page with the same repo href. Chapter lists may scroll. Wordmark, Learn, Reference, search, and theme stay.

**Key interfaces:**
- SiteHeader primary nav cluster order
- GitHub as a child of that nav, not after the chapter groups

**Acceptance criteria:**
- [ ] Tests fail if GitHub is ordered after LearnNav and ReferenceNav
- [ ] GitHub still renders in the side nav with the existing repo href
- [ ] Named vitest file passes and typecheck exits 0
- [ ] Promise ids listed above still hold

**Out of scope:**
- Tall mobile hit ([[task-879-github-mobile-hit]]); dropping search or theme; playground

**Explain this part:**
Same reachability cut as search-above-the-fold, now for GitHub.
