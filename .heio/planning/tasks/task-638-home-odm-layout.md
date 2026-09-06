---
id: "task-638-home-odm-layout"
title: "Lay out home like ODM"
kind: task
status: ready
mode: afk
blocked_by: [ "task-637-site-shell" ]
sprint: "website-odm-match"
slice: "slice-631-home-odm-layout"
area: public-site
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T19:30:00Z"
---

# Lay out home like ODM

## Blocked by

[[task-637-site-shell]]: home is the main column of the shell.

## Done

Home keeps locked pitch and CTAs and uses kicker, CTA row, path steps, and feature cards.

## Context

Keep the locked home sentences and Install → `/install` plus Learn → `/learn` CTAs. Restyle HomeHero and HomeFeatures to ODM bands: uppercase letterspaced kicker, h1, lead, `cta-row` of primary and ghost links, numbered path steps, auto-fit card grid. Do not use DocsShell on `/`. Do not add playground, Get started, tutorial, or vault copy. Path steps can point at Install then Learn then Reference without changing those labels' routes.

## Verify

`pnpm --dir website exec vitest run home-hero-and-cta home-features` passes. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `website/src/features/home/`, `website/src/routes/index.tsx`, `website/src/tests/home-hero-and-cta.test.ts`, `website/src/tests/home-features.test.ts`

## Links

[[slice-631-home-odm-layout]] [[task-637-site-shell]]

## Agent Brief

**Category:** enhancement
**Summary:** Restyle the language homepage bands to ODM kicker, CTAs, path steps, and cards.

**Drain:** `/afk-task`. If [[task-637-site-shell]] is not `completed`, stop. Do not claim.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**. Public site is `website/` TanStack Start. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-631-home-odm-layout]].

**TDD:** extend home-hero and home-features tests first for kicker, CTA row, path steps, and cards. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.home:landing`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not add playground, Get started, tutorial, or vault copy

**Current behavior:**
Two bands: display hero with two CTAs; three-column feature grid.

**Desired behavior:**
Same copy and CTA targets. Layout: kicker, hero, CTA row (primary + ghost), numbered path steps, feature cards in an auto-fit grid. Still not a Learn dump. Still not DocsShell.

**Key interfaces:**
- HomeHero and HomeFeatures CVA variants
- Index route only; no DocsShell import

**Acceptance criteria:**
- [ ] Locked pitch strings still present
- [ ] Install and Learn CTAs still those hrefs
- [ ] Kicker, path steps, and cards exist
- [ ] Named tests pass and typecheck exits 0

**Out of scope:**
- Docs article; hub cards; changing IA; playground

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run home-hero-and-cta home-features` — win. `Test Files  2 passed`. Typecheck holds.
