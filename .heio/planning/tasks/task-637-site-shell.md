---
id: "task-637-site-shell"
title: "Replace top header with ODM site shell"
kind: task
status: ready
mode: afk
blocked_by: [ "task-636-odm-tokens" ]
sprint: "website-odm-match"
slice: "slice-630-site-shell"
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T18:00:00Z"
---

# Replace top header with ODM site shell

## Blocked by

[[task-636-odm-tokens]]: shell uses tokens, not hex.

## Done

Every page is skip link, sticky side nav, main column. Side nav has wordmark, Learn, Reference, GitHub. Search and theme toggle live in the side.

## Context

Current root is SkipLink, SiteHeader (top bar), main#main, SiteFooter. ODM layout is `a.skip` then `div.shell` of sticky `aside.side` plus `main.main`. Side nav has brand, short tagline, grouped links, one `aria-current="page"`. Keep TanStack Start. Keep search and theme toggle in the side nav even though ODM lacks them. Update `site-header-primary-nav` and skip-link tests so they look at the side nav, not a top header. Do not put DocsShell on `/`. Footer related links on docs pages come later; a slim site footer may remain inside main.

## Verify

`pnpm --dir website exec vitest run site-header-primary-nav site-footer-and-skip-link` passes. Typecheck holds.

scope: `website/src/routes/__root.tsx`, `website/src/components/SiteHeader/`, `website/src/components/SiteFooter/`, `website/src/components/SkipLink/`, `website/src/components/SiteSearch/`, `website/src/components/ThemeToggle/`, `website/src/tests/site-header-primary-nav.test.ts`, `website/src/tests/site-footer-and-skip-link.test.ts`

## Links

[[slice-630-site-shell]] [[task-636-odm-tokens]]

## Agent Brief

**Category:** enhancement
**Summary:** Put ODM two-column chrome on every route.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.chrome:odm-shell`, `public-site.chrome:primary-nav`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site; do not drop search

**Current behavior:**
Top header bar with wordmark, Learn, Reference, GitHub, search, theme toggle.

**Desired behavior:**
Skip to `#main`, sticky side nav, main column on every page including home. Side nav: Draconic wordmark to `/`, short tagline, Learn, Reference, GitHub, search, theme toggle. No site-wide top header. Tokens only; CVA in variants.

**Key interfaces:**
- Root layout `website/src/routes/__root.tsx`
- SiteHeader may become the side nav component
- Load **frontend-development** (`start-file-routes`, `package-component-layout`)

**Acceptance criteria:**
- [ ] Skip link is first and targets `#main`
- [ ] Sticky side nav + main on `/` and a Learn page
- [ ] Wordmark, Learn, Reference, GitHub in the side nav
- [ ] Search and theme toggle still present
- [ ] Tests named above pass

**Out of scope:**
- Home kicker/path-steps; docs article prose; Learn/Reference groups; mobile wrap; hamburger removal beyond what root tests require; Start replacement

**Explain this part:**
ODM docs and home share one shell. The top bar goes away. Promises still require those four links and search; they just live in the side.
