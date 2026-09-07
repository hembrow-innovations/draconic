---
id: "task-686-chapter-nav-focus-ring"
title: "Add token focus rings to chapter nav links"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "website-odm-match"
slice: "slice-685-chapter-nav-focus-ring"
area: public-site
tags: [ website, public-site ]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-07T23:40:00Z"
---
# Add token focus rings to chapter nav links

## Blocked by

None.

## Done

Learn and Reference chapter links in the side nav show the same token `focus-visible` ring as hub primary-nav links.

## Context

Hub Learn, Reference, GitHub, search, theme toggle, and skip already include `focus-visible:ring-2 focus-visible:ring-accent`. Chapter lists are now part of primary nav. `public-site.a11y:keyboard-small` says primary nav works with keyboard. Walked Install chapter classes: body muted, no ring.

## Verify

`pnpm --dir website exec vitest run mobile-a11y learn-hub-nav reference-hub-pages` prints `Test Files  3 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `website/src/features/learn/LearnNav/`, `website/src/features/reference/ReferenceNav/`, `website/src/tests/mobile-a11y.test.ts`, `website/src/tests/learn-hub-nav.test.ts`, `website/src/tests/reference-hub-pages.test.ts`

## Links

[[slice-685-chapter-nav-focus-ring]] [[ticket-652-chapter-nav-no-focus-ring]]

## Agent Brief

**Category:** bug
**Summary:** Give grouped chapter links the same token focus ring as other primary nav.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-685-chapter-nav-focus-ring]]; [[ticket-652-chapter-nav-no-focus-ring]].

**TDD:** extend mobile-a11y (and hub nav tests if needed) first so LearnNav and ReferenceNav link variants include token `focus-visible` rings. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.a11y:keyboard-small`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: assert the existing promise, then test, then code

**Current behavior:**
Chapter and working-page links in the side nav have no `focus-visible` token ring. Hub links do.

**Desired behavior:**
Keyboard focus on those chapter links shows the same token ring as Learn, Reference, GitHub, search, theme toggle, and skip. No hex. CVA stays in variants. Chapter order unchanged.

**Key interfaces:**
- LearnNav and ReferenceNav link CVA
- Existing hub `siteHeaderLinkVariants` ring pattern

**Acceptance criteria:**
- [x] Tests fail if chapter link variants omit `focus-visible:ring-2` and `focus-visible:ring-accent`
- [x] Named vitest files pass and typecheck exits 0
- [x] `public-site.a11y:keyboard-small` still holds
- [x] No hamburger restored

**Out of scope:**
- Tap targets ([[task-688-chapter-nav-touch-target]]); current-page color; IA changes

**Explain this part:**
Chapter lists joined primary nav in the docs-nav-groups slice. Keyboard-small already covers that nav.

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run mobile-a11y learn-hub-nav reference-hub-pages` — win. `Test Files  3 passed`. Typecheck exits 0.
