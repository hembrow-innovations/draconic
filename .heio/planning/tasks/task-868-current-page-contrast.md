---
id: "task-868-current-page-contrast"
title: "Give current-page nav text sufficient contrast"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "website-chrome-polish"
slice: "slice-867-current-page-contrast"
area: public-site
tags: [website, public-site]
created_at: "2026-09-12T07:05:00Z"
updated_at: "2026-09-12T07:05:00Z"
---

# Give current-page nav text sufficient contrast

## Blocked by

None.

## Done

Current-page nav text meets 4.5:1 against the canvas, stays distinct from muted siblings, and still uses `aria-current="page"`.

## Context

Closed current-page visual shipped `accent-2` on `aria-current`. That green on light canvas is about 3:1 for body-sized nav text. Tighten the existing promise rather than adding a new token name. Prefer an existing semantic token. If no existing token meets 4.5:1 on canvas, adjust the current-page token value; do not add a new token name.

## Verify

`pnpm --dir website exec vitest run site-header-primary-nav learn-hub-nav reference-hub-pages` prints `Test Files  3 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `docs/specs/draconic/public-site/`, `website/src/styles/`, `website/src/components/SiteHeader/`, `website/src/features/learn/LearnNav/`, `website/src/features/reference/ReferenceNav/`, matching website tests

## Links

[[slice-867-current-page-contrast]] [[ticket-849-current-page-contrast]]

## Agent Brief

**Category:** bug
**Summary:** Make current-page nav text meet 4.5:1 contrast on the canvas while staying distinct from muted siblings.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**, **website**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-867-current-page-contrast]]; [[ticket-849-current-page-contrast]].

**TDD:** extend current-page tests so the 3:1 accent-2-on-canvas pair cannot pass as current-page ink. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.chrome:current-page` (tighten contrast; keep aria-current and existing-token rule)
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: edit/assert the existing promise → test → code. Do not invent a new token name.

**Current behavior:**
Open pages use `aria-current="page"` and `accent-2` (`rgb(47, 158, 111)`) on canvas (`rgb(245, 248, 252)`), about 3:1. Inactive Learn and Reference use muted ink. Distinct, not readable.

**Desired behavior:**
Current-page nav text meets 4.5:1 against canvas at body size. It stays visually distinct from muted siblings. `aria-current="page"` still marks the open page. Hub and chapter current items both get the treatment. Tokens and CVA only; no hex in components.

**Key interfaces:**
- Current-page CVA on SiteHeader, LearnNav, and ReferenceNav
- Existing semantic tokens; optional value tweak of the current-page token, not a new name

**Acceptance criteria:**
- [ ] Contract `public-site.chrome:current-page` requires sufficient contrast, not only a distinct token
- [ ] Tests fail if current-page ink is still the 3:1 accent-2-on-canvas pair
- [ ] Named vitest files pass and typecheck exits 0
- [ ] No new color token name

**Out of scope:**
- Changing which page is current; ODM product copy; playground

**Explain this part:**
Color already exists. This sitting is contrast, not a second current-page invention.
