---
id: "task-872-per-page-titles"
title: "Name the open page in the document title"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "website-chrome-polish"
slice: "slice-871-per-page-titles"
area: public-site
tags: [website, public-site]
created_at: "2026-09-12T07:05:00Z"
updated_at: "2026-09-12T07:05:00Z"
---

# Name the open page in the document title

## Blocked by

None.

## Done

Each public page's document title names the open page so tabs distinguish destinations. Home may remain Draconic.

## Context

Root head hardcodes the title Draconic for every route. No contract promise names per-page titles. Assert one, then test, then code. Do not rewrite page headings.

## Verify

`pnpm --dir website exec vitest run document-title` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `docs/specs/draconic/public-site/`, `website/src/routes/`, `website/src/tests/document-title.test.ts`

## Links

[[slice-871-per-page-titles]] [[ticket-851-untitled-pages]]

## Agent Brief

**Category:** enhancement
**Summary:** Set per-page document titles so tabs and history name the open page.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**, **website**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-871-per-page-titles]]; [[ticket-851-untitled-pages]].

**TDD:** assert a contract promise for per-page document titles, add `document-title` tests that fail when every route is only Draconic, then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: add and lock `public-site.chrome:document-title`
- Purpose: [[docs/specs/draconic/public-site/purpose]] (add titles to In scope if the promise needs it)
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: edit/assert promise → test → code. Do not invent extra title marketing copy.

**Current behavior:**
Home, Learn, Reference, and article pages all use the document title Draconic. Tabs and history do not distinguish destinations.

**Desired behavior:**
Each page's document title names the open page so two tabs are distinguishable. Home may remain Draconic. Article titles should use the page's existing heading or markdown title, not a new slogan. Site name Draconic may still appear with the page name.

**Key interfaces:**
- TanStack Start route `head` / document title
- Existing page titles from teaching markdown and hub headings

**Acceptance criteria:**
- [ ] Contract lists `public-site.chrome:document-title` with a test pointer
- [ ] Tests fail if Learn, Reference, and an article page all share only the title Draconic
- [ ] Named vitest file passes and typecheck exits 0
- [ ] Page h1 copy is not rewritten

**Out of scope:**
- Favicon ([[task-864-favicon]]); meta description; playground

**Explain this part:**
This sitting is allowed to assert the missing title promise. It is not a copy rewrite.
