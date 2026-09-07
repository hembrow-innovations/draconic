---
id: "task-682-current-page-visual"
title: "Give the current side-nav page a visible treatment"
kind: task
status: completed
mode: afk
blocked_by:
  - task-688-chapter-nav-touch-target
sprint: "website-odm-match"
slice: "slice-681-current-page-visual"
area: public-site
tags: [website, public-site]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-07T23:55:00Z"
---

# Give the current side-nav page a visible treatment

## Blocked by

[[task-688-chapter-nav-touch-target]]: same chapter-link variants after tap size.

## Done

The `aria-current="page"` side-nav item is visually distinct using an existing semantic token.

## Context

No contract currently names current-page color. Purpose and primary-nav only lock the four hub links plus aria-current from the docs-nav-groups slice. ODM uses mint; token `accent-2` already exists and is used on home path numbers. Do not invent a new palette. Contract-first: assert a promise, then test, then code.

## Verify

`pnpm --dir website exec vitest run site-header-primary-nav learn-hub-nav reference-hub-pages` prints `Test Files  3 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `docs/specs/draconic/public-site/contract.md`, `docs/specs/draconic/public-site/test.md`, `website/src/components/SiteHeader/`, `website/src/features/learn/LearnNav/`, `website/src/features/reference/ReferenceNav/`, matching website tests

## Links

[[slice-681-current-page-visual]] [[ticket-650-current-page-no-visual]] [[task-688-chapter-nav-touch-target]]

## Agent Brief

**Category:** enhancement
**Summary:** Make the current side-nav page visible, not only `aria-current`.

**Drain:** `/afk-task`. If [[task-688-chapter-nav-touch-target]] is not `completed`, stop. Do not claim.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-681-current-page-visual]]; [[ticket-650-current-page-no-visual]].

**TDD:** assert a contract promise for current-page chrome, point a test at it, then implement. Do not invent a new color token.

**Intent (required when product behaviour changes):**
- Promise ids: add and lock `public-site.chrome:current-page`; keep `public-site.chrome:primary-nav`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: edit/assert promise → test → code. Do not invent product rules beyond a visible current item using existing tokens.

**Current behavior:**
Open pages get `aria-current="page"` but stay `text-muted` weight 400, same as sibling chapter links. Wordmark, Learn hub, Install, Reference hub, and CLI all look unselected.

**Desired behavior:**
The current side-nav item is visually distinct from siblings via an existing semantic token (accent-2 is the available mint analog). `aria-current="page"` still marks the open page. Hub links and chapter links both get the treatment when current.

**Key interfaces:**
- SiteHeader, LearnNav, and ReferenceNav current/active link chrome
- Existing `accent-2` token; CVA variants; no hex

**Acceptance criteria:**
- [x] Contract lists `public-site.chrome:current-page` with a test pointer
- [x] Tests fail if current-page chrome is only aria-current with muted ink
- [x] Named vitest files pass and typecheck exits 0
- [x] No new color token and no ODM product copy

**Out of scope:**
- Focus rings and tap size (prior tasks); changing IA; playground

**Explain this part:**
The audit found a visual gap with no promise. This sitting is allowed to assert the promise, not to restyle from taste.

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run site-header-primary-nav learn-hub-nav reference-hub-pages` — win. `Test Files  3 passed`. Typecheck exits 0. Critic: hub and chapter current items use existing `accent-2` with `aria-current`; `public-site.chrome:current-page` locked; `public-site.chrome:primary-nav` kept; no new token; coverage "same promise" wording tightened.
