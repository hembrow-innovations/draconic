---
id: "task-864-favicon"
title: "Serve a public site favicon"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "website-chrome-polish"
slice: "slice-863-favicon"
area: public-site
tags: [website, public-site]
created_at: "2026-09-12T07:05:00Z"
updated_at: "2026-09-12T07:05:00Z"
---

# Serve a public site favicon

## Blocked by

None.

## Done

The public origin serves a favicon so the browser tab shows a site icon and `/favicon.ico` no longer 404s.

## Context

Every load logs a missing favicon. No icon file ships with the Start app. Chrome polish on the public-site location, not a teaching-copy change. Contract has no favicon promise yet. Assert one, then test, then code.

## Verify

`pnpm --dir website exec vitest run site-header-primary-nav` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `docs/specs/draconic/public-site/`, `website/src/routes/__root.tsx`, `website/src/tests/site-header-primary-nav.test.ts`, `website/public/`

## Links

[[slice-863-favicon]] [[ticket-847-missing-favicon]]

## Agent Brief

**Category:** bug
**Summary:** Serve a favicon so the public site tab has a site icon and first paint does not 404 `/favicon.ico`.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**, **website**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-863-favicon]]; [[ticket-847-missing-favicon]].

**TDD:** assert a contract promise for a served favicon, point primary-nav tests at it, then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: add and lock `public-site.chrome:favicon`; keep `public-site.chrome:odm-shell`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: edit/assert promise → test → code. Do not invent extra chrome.

**Current behavior:**
The origin has no favicon. Browsers request `/favicon.ico` and log a 404. The tab shows no site icon.

**Desired behavior:**
The origin serves a favicon. The tab shows a site icon. `/favicon.ico` (or an equivalent linked icon the browser will use) is not a 404. Teaching copy is unchanged.

**Key interfaces:**
- Root document head / static public assets for the Start app
- Existing site chrome; no new marketing bar

**Acceptance criteria:**
- [ ] Contract lists `public-site.chrome:favicon` with a test pointer
- [ ] Tests fail if the app does not serve or link a favicon
- [ ] Named vitest file passes and typecheck exits 0
- [ ] Promise ids listed above still hold (or were deliberately edited)

**Out of scope:**
- Per-page document titles ([[task-872-per-page-titles]]); Learn/Reference copy; playground; vault-as-site

**Explain this part:**
This sitting is allowed to assert the missing chrome promise. It is not a visual restyle.
