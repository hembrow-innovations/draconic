---
id: "task-638-home-odm-layout"
title: "Lay out home like ODM"
kind: task
status: ready
mode: afk
blocked_by: [ "task-637-site-shell" ]
sprint: "website-odm-match"
slice: "slice-631-home-odm-layout"
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T18:00:00Z"
---

# Lay out home like ODM

## Blocked by

[[task-637-site-shell]]: home is the main column of the shell.

## Done

Home keeps locked pitch and CTAs and uses kicker, CTA row, path steps, and feature cards.

## Context

Keep the locked home sentences and Install → `/install` plus Learn → `/learn` CTAs. Restyle HomeHero and HomeFeatures to ODM bands: uppercase letterspaced kicker, h1, lead, `cta-row` of primary and ghost links, numbered path steps, auto-fit card grid. Do not use DocsShell on `/`. Do not add playground, Get started, tutorial, or vault copy. Path steps can point at Install then Learn then Reference without changing those labels' routes.

## Verify

`pnpm --dir website exec vitest run home-hero-and-cta home-features` passes. Typecheck holds.

scope: `website/src/features/home/`, `website/src/routes/index.tsx`, `website/src/tests/home-hero-and-cta.test.ts`, `website/src/tests/home-features.test.ts`

## Links

[[slice-631-home-odm-layout]] [[task-637-site-shell]]

## Agent Brief

**Category:** enhancement
**Summary:** Restyle the language homepage bands to ODM kicker, CTAs, path steps, and cards.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.home:landing`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]

**Current behavior:**
Two bands: display hero with two CTAs; three-column feature grid.

**Desired behavior:**
Same copy and CTA targets. Layout: kicker, hero, CTA row (primary + ghost), numbered path steps, feature cards in an auto-fit grid. Still not a Learn dump. Still not DocsShell.

**Acceptance criteria:**
- [ ] Locked pitch strings still present
- [ ] Install and Learn CTAs still those hrefs
- [ ] Kicker, path steps, and cards exist
- [ ] Home tests pass

**Out of scope:**
- Docs article; hub cards; changing IA; playground
