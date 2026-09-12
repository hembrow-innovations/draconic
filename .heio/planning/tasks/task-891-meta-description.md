---
id: "task-891-meta-description"
title: "Expose page-specific meta description and share tags"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "website-chrome-polish"
slice: "slice-890-meta-description"
area: public-site
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Expose page-specific meta description and share tags

## Blocked by

None.

## Done

Public pages have a page-specific description, canonical, and Open Graph summary. Titles stay on [[slice-871-per-page-titles]].

## Context

Root head has charset, viewport, and title Draconic. No description, canonical, or Open Graph. Distinct from per-page titles. Assert `public-site.chrome:meta-description`, then test, then code.

## Verify

`pnpm --dir website exec vitest run meta-description` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `docs/specs/draconic/public-site/`, `website/src/routes/`, `website/src/lib/content/`, `website/src/tests/meta-description.test.ts`

## Links

[[slice-890-meta-description]] [[ticket-861-missing-meta-description]]

## Agent Brief

**Category:** enhancement
**Summary:** Add page-specific meta description, canonical, and Open Graph so shares and search snippets summarize the open page.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**, **website**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-890-meta-description]]; [[ticket-861-missing-meta-description]].

**TDD:** assert `public-site.chrome:meta-description`, add `meta-description` tests that fail when home, Learn, and an article share no description, canonical, or og tags, then implement, then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: add and lock `public-site.chrome:meta-description`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: edit/assert promise → test → code. Do not invent extra SEO fields or new slogans.

**Current behavior:**
Root `head` has charset, viewport, and title Draconic. Home, Learn, Reference, and articles have no description meta, no canonical link, and no Open Graph tags.

**Desired behavior:**
Each public page exposes a page-specific meta description, a canonical URL, and Open Graph tags that summarize that page. Use existing page copy (heading, lead, or first paragraph), not a new slogan. Site name Draconic may still appear with the page name in og:title. Home may use the existing homepage pitch.

**Key interfaces:**
- TanStack Start route `head` / document meta and links
- Existing teaching titles and leads; no required new frontmatter field

**Acceptance criteria:**
- [ ] Contract lists `public-site.chrome:meta-description` with a test pointer
- [ ] Tests fail if home, Learn, and an article omit description, canonical, and og tags
- [ ] Named vitest file passes and typecheck exits 0
- [ ] Page h1 copy is not rewritten

**Out of scope:**
- Document titles ([[task-872-per-page-titles]]); theme-color; Twitter cards; og:image; playground

**Explain this part:**
This sitting is allowed to assert the missing share-meta promise. It is not a title rewrite and not a copy rewrite. If [[task-872-per-page-titles]] is in flight, do not collide on route heads without reading it first.
