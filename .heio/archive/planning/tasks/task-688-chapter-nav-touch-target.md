---
id: "task-688-chapter-nav-touch-target"
title: "Match chapter nav tap targets to hub links"
kind: task
status: completed
mode: afk
blocked_by:
  - task-686-chapter-nav-focus-ring
sprint: "website-odm-match"
slice: "slice-687-chapter-nav-touch-target"
area: public-site
tags: [ website, public-site ]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-07T23:55:00Z"
---
# Match chapter nav tap targets to hub links

## Blocked by

[[task-686-chapter-nav-focus-ring]]: same chapter-link variants; focus ring first.

## Done

Learn and Reference chapter links use `min-h-11` like hub Learn, Reference, and GitHub.

## Context

No contract names a 44px target. Hub links already use `min-h-11`. Chapter links are now primary nav on the same small-viewport walk surface. Match existing hub chrome rather than inventing a new size.

## Verify

`pnpm --dir website exec vitest run mobile-a11y` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `website/src/features/learn/LearnNav/`, `website/src/features/reference/ReferenceNav/`, `website/src/tests/mobile-a11y.test.ts`

## Links

[[slice-687-chapter-nav-touch-target]] [[ticket-653-chapter-nav-touch-target]] [[task-686-chapter-nav-focus-ring]]

## Agent Brief

**Category:** enhancement
**Summary:** Give grouped chapter links the same 44px minimum height as hub primary-nav links.

**Drain:** `/afk-task`. If [[task-686-chapter-nav-focus-ring]] is not `completed`, stop. Do not claim.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-687-chapter-nav-touch-target]]; [[ticket-653-chapter-nav-touch-target]].

**TDD:** extend mobile-a11y first so chapter link variants include `min-h-11`. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.a11y:keyboard-small`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent a new tap size; reuse the hub `min-h-11` already on primary nav

**Current behavior:**
On a 375-wide walk, hub Learn / Reference / GitHub are 44px min-height. Install chapter `minHeight` is 0.

**Desired behavior:**
Chapter and working-page links in the side nav use the same `min-h-11` as hub links. Focus rings from the previous task stay. No hamburger. Tokens and CVA only.

**Key interfaces:**
- LearnNav and ReferenceNav link CVA
- Existing hub `min-h-11` on SiteHeader links

**Acceptance criteria:**
- [x] Tests fail if chapter link variants omit `min-h-11`
- [x] Named vitest file passes and typecheck exits 0
- [x] Focus-visible rings from the prior task still hold
- [x] No new size token

**Out of scope:**
- Current-page color ([[task-682-current-page-visual]]); changing chapter order; playground

**Explain this part:**
This is consistency with chrome the hub already ships, not a new a11y product rule.

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run mobile-a11y` — win. `Test Files  1 passed`. Typecheck exits 0. Critic: diff matches hub `min-h-11` plus `inline-flex items-center` so the height applies; focus rings stay; no new token; `public-site.a11y:keyboard-small` holds; purpose out-of-scope untouched.
