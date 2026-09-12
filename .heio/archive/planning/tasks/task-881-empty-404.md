---
id: "task-881-empty-404"
title: "Recover unknown URLs with a not-found page"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "website-chrome-polish"
slice: "slice-880-empty-404"
area: public-site
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T08:22:12Z"
---

# Recover unknown URLs with a not-found page

## Blocked by

None.

## Done

An unknown URL keeps site chrome, main has a heading and a way back, and the router default Not Found paragraph is gone.

## Context

Unknown URLs keep skip-link, sticky side nav, and footer. Main is only the words Not Found, as a paragraph. No heading. No way back. Root has no notFoundComponent. Contract is silent. Assert `public-site.chrome:not-found`, then test, then code.

## Verify

`pnpm --dir website exec vitest run not-found` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `docs/specs/draconic/public-site/`, `website/src/routes/__root.tsx`, `website/src/components/`, `website/src/tests/not-found.test.ts`

## Links

[[slice-880-empty-404]] [[ticket-856-empty-404]]

## Agent Brief

**Category:** bug
**Summary:** Give unknown URLs a recovery main column with a heading and a way back, and stop using the router default Not Found paragraph.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**, **website**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-880-empty-404]]; [[ticket-856-empty-404]].

**TDD:** assert a contract promise for unknown-URL recovery, add `not-found` tests that fail while root has no `notFoundComponent` and main is only the default Not Found paragraph, then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: add and lock `public-site.chrome:not-found`; keep `public-site.chrome:odm-shell`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: edit/assert promise → test → code. Do not invent extra marketing chrome.

**Current behavior:**
Unknown URLs keep skip-link, sticky side nav, and footer. Main is only the words Not Found, as a paragraph. No heading. No way back. Console warns that the root route has no `notFoundComponent`. Document title stays Draconic. HTTP status is 404.

**Desired behavior:**
Unknown URLs still use skip-link, sticky side nav, and footer. Main has a heading that names the miss, plus at least one in-site link back to a real page such as Home or Learn. The router default Not Found paragraph is not the main content. The recovery view names the miss in the document title. HTTP status stays 404. Teaching copy is unchanged. Do not mark a side-nav item `aria-current=page` for a path that is not a page.

**Key interfaces:**
- Root route `notFoundComponent` on the Start app
- Existing site chrome; no new marketing bar

**Acceptance criteria:**
- [x] Contract lists `public-site.chrome:not-found` with a test pointer
- [x] Tests fail if root has no `notFoundComponent` or if recovery main is only the default Not Found paragraph
- [x] Named vitest file passes and typecheck exits 0
- [x] Promise ids listed above still hold (or were deliberately edited)

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run not-found` — win. Test Files  1 passed. `pnpm --dir website exec tsc --noEmit` exits 0. Diff locks `public-site.chrome:not-found`; keeps `public-site.chrome:odm-shell`; recovery heading and Home link; no default Not Found paragraph; no extra marketing chrome.

**Out of scope:**
- Per-page titles on real routes ([[task-872-per-page-titles]]); Learn/Reference copy; playground; vault-as-site

**Explain this part:**
This sitting is allowed to assert the missing recovery promise. It is not a visual restyle and not the real-page title slice.
