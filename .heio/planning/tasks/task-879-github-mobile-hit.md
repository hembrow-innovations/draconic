---
id: "task-879-github-mobile-hit"
title: "Keep mobile GitHub a compact control"
kind: task
status: ready
mode: afk
blocked_by:
  - task-866-github-above-fold
sprint: "website-chrome-polish"
slice: "slice-875-github-mobile-hit"
area: public-site
tags: [website, public-site]
created_at: "2026-09-12T07:05:00Z"
updated_at: "2026-09-12T07:05:00Z"
---

# Keep mobile GitHub a compact control

## Blocked by

[[task-866-github-above-fold]]: place GitHub with always-visible chrome before locking wrap alignment.

## Done

At a small viewport GitHub is a compact tap target, not a stretched column.

## Context

Purpose already says the side nav stacks and wraps. On a 360-wide walk GitHub became about 51 by 460 CSS pixels because the cluster wraps into columns with stretch alignment. Theme toggle stayed compact. No hamburger.

## Verify

`pnpm --dir website exec vitest run mobile-a11y` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `website/src/components/SiteHeader/`, `website/src/tests/mobile-a11y.test.ts`

## Links

[[slice-875-github-mobile-hit]] [[ticket-853-github-mobile-hit-area]] [[task-866-github-above-fold]]

## Agent Brief

**Category:** bug
**Summary:** Keep the small-viewport GitHub control compact instead of stretching into a tall column.

**Drain:** `/afk-task`. If [[task-866-github-above-fold]] is not `completed`, stop. Do not claim.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**, **website**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-875-github-mobile-hit]]; [[ticket-853-github-mobile-hit-area]].

**TDD:** extend mobile-a11y first so the GitHub control and its cluster cannot use stretch alignment that grows a tall column. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.a11y:keyboard-small`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: assert the existing promise in tests, then code. Keep wrap. Do not add a hamburger.

**Current behavior:**
At about 360 by 732 the primary cluster wraps. GitHub sits in a third column with stretch alignment and becomes a tall strip. Keyboard Tab draws a focus ring around that whole strip. Theme toggle stays compact beside search.

**Desired behavior:**
GitHub is a compact control on a small viewport, comparable to hub links and the theme toggle, with `min-h-11` rather than a stretched column height. Wrap and stack stay. Focus rings the compact control. Wordmark, Learn, Reference, and GitHub stay keyboard-reachable without a Menu button.

**Key interfaces:**
- SiteHeader cluster and link CVA alignment (`items-stretch` versus compact self alignment)
- Existing hub `min-h-11` and wrap classes

**Acceptance criteria:**
- [ ] Tests fail if GitHub or its cluster stretch-aligns into a tall column
- [ ] Named vitest file passes and typecheck exits 0
- [ ] No hamburger and no drop of wrap
- [ ] Promise ids listed above still hold

**Out of scope:**
- Desktop below-fold ([[task-866-github-above-fold]] owns placement); playground

**Explain this part:**
Placement lands first. This sitting is wrap alignment, not a second GitHub move.
