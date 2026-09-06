---
id: "task-642-mobile-odm-wrap"
title: "Wrap the side nav on small viewports"
kind: task
status: ready
mode: afk
blocked_by: [ "task-637-site-shell" ]
sprint: "website-odm-match"
slice: "slice-635-mobile-odm-wrap"
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T18:00:00Z"
---

# Wrap the side nav on small viewports

## Blocked by

[[task-637-site-shell]]: wrap applies to the side nav.

## Done

Small viewport stacks the side nav above main and wraps its links. Skip link stays first. Keyboard and token focus hold.

## Context

ODM at max-width 860px makes the shell one column; the side nav is relative with a bottom border; links wrap. No hamburger. Current Draconic tests lock a Menu disclosure in the header. Update `website/src/tests/mobile-a11y.test.ts` so `public-site.a11y:keyboard-small` still holds: skip link first, nav links reachable with keyboard, focus-visible uses accent token. Do not restore a top header. Keep search and theme toggle reachable.

## Verify

`pnpm --dir website exec vitest run mobile-a11y` passes. Typecheck holds.

scope: site shell / header variants, `website/src/tests/mobile-a11y.test.ts`, `website/src/routes/__root.tsx`

## Links

[[slice-635-mobile-odm-wrap]] [[task-637-site-shell]]

## Agent Brief

**Category:** enhancement
**Summary:** Match ODM small-viewport wrap without losing keyboard access.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.a11y:keyboard-small`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Edit the locked test if it still requires a header Menu button; keep the promise (keyboard + small viewport).

**Current behavior:**
Header Menu button, `aria-expanded`, panel, Escape. Tests require that disclosure.

**Desired behavior:**
CSS wrap like ODM. No hamburger required. Skip link first. Side nav links keyboard-reachable. Focus rings use tokens.

**Acceptance criteria:**
- [ ] Small viewport is one column, side nav above main
- [ ] No requirement for a Menu button in the updated test
- [ ] Skip link first; focus-visible token ring
- [ ] `pnpm --dir website exec vitest run mobile-a11y` → Test Files  1 passed

**Out of scope:**
- Changing IA; dropping search; playground
