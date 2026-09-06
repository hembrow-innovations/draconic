---
id: "task-614-site-footer-skip"
title: "Add site footer and skip link"
kind: task
status: completed
mode: afk
blocked_by: ["task-613-site-header-nav"]
sprint: "website-redesign"
slice: "slice-595-site-footer-skip"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T20:00:00Z"
---

# Add site footer and skip link

## Blocked by

[[task-613-site-header-nav]]: skip link and footer attach to the same root chrome as the header.

## Done

The first focusable control is a skip link to main content, and the root layout has a site footer that is chrome, not a sitemap dump.

## Context

Keyboard users must not tab through the whole header before reaching the article. A skip link is that first stop. The footer is site chrome (short, quiet), not a second copy of every Learn and Reference link.

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

Skip link is first in tab order and targets main. Footer exists on the root layout. Typecheck holds.

scope: skip link, footer component, root layout under `website/src/`

## Links

[[slice-595-site-footer-skip]] [[task-613-site-header-nav]]

## Agent Brief

**Category:** enhancement
**Summary:** Add a skip-to-content link as the first focusable control and a modest site footer.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.a11y:keyboard-small` (partial; mobile disclosure is [[task-626-mobile-a11y]])
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
`generate.drac` already emits a `.skip` link. The Start header sitting does not yet restore skip or footer.

**Desired behavior:**
Skip link is the first focusable control and moves focus to main content. Footer is site chrome, not a sitemap of every chapter. Visible focus styles may wait for [[task-626-mobile-a11y]] if they are not already tokenized, but the skip link itself lands here.

**Key interfaces:**
- Skip link as first focusable in the root layout
- `main` landmark with a matching id/target
- Footer component folder
- Load **frontend-development** (`quality-states-a11y`)

**Acceptance criteria:**
- [x] Skip link is the first focusable control
- [x] Skip target is main content
- [x] Footer is site chrome, not a dump of Learn and Reference links
- [x] No playground; no vault-as-site
- [x] If [[task-613-site-header-nav]] is not `completed`, stop

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Mobile nav disclosure ([[task-626-mobile-a11y]])
- Home hero copy ([[task-615-home-hero-cta]])

**Explain this part:**
A skip link is the first focusable control so keyboard users jump past chrome into the article. The footer is site chrome, not a sitemap dump: it should not repeat every Learn chapter just because there is space at the bottom.
