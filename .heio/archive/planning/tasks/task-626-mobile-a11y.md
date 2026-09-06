---
id: "task-626-mobile-a11y"
title: "Add mobile nav and keyboard a11y"
kind: task
status: completed
mode: afk
blocked_by: ["task-613-site-header-nav"]
sprint: "website-redesign"
slice: "slice-607-mobile-a11y"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T12:00:00Z"
---

# Add mobile nav and keyboard a11y

## Blocked by

[[task-613-site-header-nav]]: small-viewport nav is a disclosure of the existing primary nav, not a second IA.

## Done

Small viewports use a keyboard-operable disclosure for primary nav, focus is visible, and menus are not pointer-only. Skip link already comes from [[task-614-site-footer-skip]].

## Context

Primary nav must work when the viewport is small and when there is no pointer. Disclosure (open/close) is the pattern, not a hover-only mega menu. Visible focus uses tokens. Do not invent a second set of destinations.

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

Narrow viewport: primary nav is a disclosure that opens and closes from keyboard. Focus rings are visible. Typecheck holds.

scope: header/nav disclosure and focus styles under `website/src/`

## Links

[[slice-607-mobile-a11y]] [[task-613-site-header-nav]] [[task-614-site-footer-skip]]

## Agent Brief

**Category:** enhancement
**Summary:** Make primary nav work on small viewports and from the keyboard.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.a11y:keyboard-small`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
Header lists Wordmark, Learn, Reference, GitHub. Skip link is a sibling sitting. Small viewports may overflow or hide nav without a keyboard path.

**Desired behavior:**
Small viewport nav is a disclosure. It is operable with keyboard. Focus is visible. No pointer-only menus. Skip link remains the first focusable control if [[task-614-site-footer-skip]] already landed; do not remove it. Same IA as desktop: Wordmark, Learn, Reference, GitHub.

**Key interfaces:**
- Header disclosure (button + panel) with keyboard support
- Visible focus styles from tokens
- Existing skip link from [[task-614-site-footer-skip]]
- Load **frontend-development** (`quality-states-a11y`)

**Acceptance criteria:**
- [x] Small viewport primary nav is a disclosure, not pointer-only
- [x] Focus is visible
- [x] Skip link is not removed
- [x] No named `max-w-sm`; no hex in components
- [x] If [[task-613-site-header-nav]] is not `completed`, stop

## Gauntlet

- **Round 1:** `pnpm --dir website test -- mobile-a11y` plus `pnpm --dir website typecheck` — win. Critic: disclosure button plus panel, token focus rings, skip link first; no out-of-scope IA or playground.

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Changing primary IA destinations
- Fence compile and deploy ([[task-627-fence-static-deploy]])

**Explain this part:**
Small-viewport nav is a disclosure: a control that opens and closes the same primary links. Focus must be visible. Skip already comes from the footer sitting. Menus must not be pointer-only, or keyboard and touch users lose Learn and Reference.
