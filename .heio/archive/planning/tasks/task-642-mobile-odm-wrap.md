---
id: "task-642-mobile-odm-wrap"
title: "Wrap the side nav on small viewports"
kind: task
status: completed
mode: afk
blocked_by: [ "task-637-site-shell" ]
sprint: "website-odm-match"
slice: "slice-635-mobile-odm-wrap"
area: public-site
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T20:25:00Z"
---

# Wrap the side nav on small viewports

## Blocked by

[[task-637-site-shell]]: wrap applies to the side nav.

## Done

Small viewport stacks the side nav above main and wraps its links. Skip link stays first. Keyboard and token focus hold.

## Context

ODM at max-width 860px makes the shell one column; the side nav is relative with a bottom border; links wrap. No hamburger. Current Draconic tests lock a Menu disclosure in the header. Update `website/src/tests/mobile-a11y.test.ts` so `public-site.a11y:keyboard-small` still holds: skip link first, nav links reachable with keyboard, focus-visible uses accent token. Do not restore a top header. Keep search and theme toggle reachable.

## Verify

`pnpm --dir website exec vitest run mobile-a11y` passes. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: site shell / header variants, `website/src/tests/mobile-a11y.test.ts`, `website/src/routes/__root.tsx`

## Links

[[slice-635-mobile-odm-wrap]] [[task-637-site-shell]]

## Agent Brief

**Category:** enhancement
**Summary:** Match ODM small-viewport wrap without losing keyboard access.

**Drain:** `/afk-task`. If [[task-637-site-shell]] is not `completed`, stop. Do not claim.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**. Public site is `website/` TanStack Start. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-635-mobile-odm-wrap]].

**TDD:** rewrite mobile-a11y first so it requires stacked wrap, skip first, keyboard-reachable nav, token focus; drop Menu-button requirement. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.a11y:keyboard-small`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Keep the promise. Edit the locked test if it still requires a header Menu button.

**Current behavior:**
Header Menu button, `aria-expanded`, panel, Escape. Tests require that disclosure.

**Desired behavior:**
CSS wrap like ODM. No hamburger required. Skip link first. Side nav links keyboard-reachable. Focus rings use tokens.

**Key interfaces:**
- Site shell variants at small viewport
- mobile-a11y vitest

**Acceptance criteria:**
- [x] Small viewport is one column, side nav above main
- [x] No requirement for a Menu button in the updated test
- [x] Skip link first; focus-visible token ring
- [x] `pnpm --dir website exec vitest run mobile-a11y` → Test Files  1 passed
- [x] Typecheck exits 0

**Out of scope:**
- Changing IA; dropping search; playground

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run mobile-a11y` — win. `Test Files  1 passed`. Typecheck holds.
