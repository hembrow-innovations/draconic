---
id: "task-889-shipped-badge-contrast"
title: "Give the shipped badge sufficient light-theme contrast"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "website-chrome-polish"
slice: "slice-888-shipped-badge-contrast"
area: public-site
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T18:55:00Z"
---

# Give the shipped badge sufficient light-theme contrast

## Blocked by

None.

## Done

Light-theme shipped chip meets 4.5 to 1 at 14px; dark still passes; the word shipped stays visible.

## Context

Distinct from current-page nav green. Promise `public-site.nav:learn-reference-status` only requires a visible shipped or not-yet status. Tighten that promise, then test, then code. Do not add a new token name.

## Verify

`pnpm --dir website exec vitest run typography-and-badge` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `docs/specs/draconic/public-site/`, `website/src/styles/`, `website/src/components/Badge/`, `website/src/tests/typography-and-badge.test.ts`, `website/src/tests/theme-toggle.test.ts`

## Links

[[slice-888-shipped-badge-contrast]] [[ticket-860-shipped-badge-contrast]]

## Agent Brief

**Category:** bug
**Summary:** Make the shipped status chip meet 4.5 to 1 contrast in light theme at 14px without inventing a new token name.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**, **website**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-888-shipped-badge-contrast]]; [[ticket-860-shipped-badge-contrast]].

**TDD:** tighten `public-site.nav:learn-reference-status` so the shipped chip must meet 4.5 to 1 in light theme, then extend typography-and-badge so the current light pair at 14px cannot pass, then implement, then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.nav:learn-reference-status` (tighten shipped-chip contrast; keep visible shipped or not-yet). Keep `public-site.chrome:docs-sidebar`.
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: edit the existing promise → test → code. Do not add a new token name. Do not merge with current-page nav contrast.

**Current behavior:**
Learn and Reference paint a shipped span with accent fill and accent-foreground text. Light theme is about 4.37 to 1 at 14px. Dark theme is about 6.72 to 1. The chip is visible; light AA fails.

**Desired behavior:**
The shipped chip meets 4.5 to 1 against its fill in light theme at mono size. Dark still passes. The label remains the word shipped. Tokens and CVA only; no hex in components.

**Key interfaces:**
- Badge CVA shipped variant
- Existing accent and accent-foreground pair; optional value tweak, not a new name

**Acceptance criteria:**
- [x] Contract `public-site.nav:learn-reference-status` requires readable shipped-chip contrast, not only a visible word
- [x] Tests fail if light shipped text and fill is still the 4.37 pair
- [x] Named vitest file passes and typecheck exits 0
- [x] No new color token name

**Out of scope:**
- Current-page nav contrast ([[task-868-current-page-contrast]]); not-yet restyle; Learn/Reference copy; playground

**Explain this part:**
The chip already exists. This sitting is light-theme contrast, not a second badge invention and not nav green.

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run typography-and-badge` — win. Test Files  1 passed. `pnpm --dir website exec tsc --noEmit` exits 0. Diff keeps `public-site.nav:learn-reference-status` visible shipped or not-yet; tightens 4.5:1 at 14px; light `--color-accent` darkened to `#2c6cb3`; no new token; `public-site.chrome:docs-sidebar` and current-page nav untouched.
