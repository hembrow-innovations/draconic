---
id: "task-874-search-session-reset"
title: "Clear search on navigation and Escape"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "website-chrome-polish"
slice: "slice-873-search-session-reset"
area: public-site
tags: [website, public-site]
created_at: "2026-09-12T07:05:00Z"
updated_at: "2026-09-12T18:20:00Z"
---

# Clear search on navigation and Escape

## Blocked by

None.

## Done

Following a search hit or pressing Escape clears the query and result list so leftover hits cannot steal `aria-current="page"`.

## Context

SiteSearch keeps local query state. There is no route reset and no Escape handler. Distinct from what the index matches and from result clipping. Extend search behaviour so a hit is a navigation, not a sticky overlay.

## Verify

`pnpm --dir website exec vitest run search` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `docs/specs/draconic/public-site/`, `website/src/components/SiteSearch/`, `website/src/tests/search.test.ts`

## Links

[[slice-873-search-session-reset]] [[ticket-852-search-stays-open]] [[ticket-821-search-body-terms]]

## Agent Brief

**Category:** bug
**Summary:** Clear in-site search query and results after following a hit or pressing Escape.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**, **website**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-873-search-session-reset]]; [[ticket-852-search-stays-open]].

**TDD:** extend search tests first so query state resets on navigation and Escape, and leftover hits cannot keep `aria-current`. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: add and lock `public-site.search:session`; keep `public-site.search:titles-headings`, `public-site.chrome:current-page`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: edit/assert promise → test → code. Do not widen the index.

**Current behavior:**
Typing a query and following a hit leaves the query and result list open on the next article. A leftover hit can be the `aria-current="page"` element. Escape leaves the list open.

**Desired behavior:**
Following a result navigates and clears the query and list. Escape clears the query and list without leaving the page. After navigation, `aria-current="page"` is the side-nav current page, not a leftover hit. Search stays in the side nav. Index still matches title and heading only.

**Key interfaces:**
- SiteSearch local query state
- Router navigation / location change as the reset signal
- Keyboard Escape on the search field

**Acceptance criteria:**
- [x] Contract lists `public-site.search:session` with a test pointer
- [x] Tests fail if SiteSearch has no navigation reset and no Escape handler
- [x] Named vitest file passes and typecheck exits 0
- [x] Title and heading index behaviour is unchanged

**Out of scope:**
- Body-term indexing ([[ticket-821-search-body-terms]]); result clipping; moving search out of the nav

**Explain this part:**
This sitting is session lifecycle. It does not change what a query matches.

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run search` — win. Test Files  1 passed. `pnpm --dir website exec tsc --noEmit` exits 0. Diff locks `public-site.search:session`; titles-headings index unchanged; leftover hits unmount on location change and Escape so they cannot keep aria-current.
