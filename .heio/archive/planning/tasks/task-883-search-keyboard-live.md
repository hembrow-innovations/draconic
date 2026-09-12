---
id: "task-883-search-keyboard-live"
title: "Make search results keyboardable and announced"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "website-chrome-polish"
slice: "slice-882-search-keyboard-live"
area: public-site
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T18:35:00Z"
---

# Make search results keyboardable and announced

## Blocked by

None.

## Done

Arrow Down and Enter move through and follow hits; hits and misses are announced.

## Context

SiteSearch finds Dual worlds and shows No matching pages, but the field is a plain search input. Distinct from session reset and from body-term indexing. Assert `public-site.search:keyboard-live`, then test, then code.

## Verify

`pnpm --dir website exec vitest run search` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `docs/specs/draconic/public-site/`, `website/src/components/SiteSearch/`, `website/src/tests/search.test.ts`

## Links

[[slice-882-search-keyboard-live]] [[ticket-857-search-keyboard-live]]

## Agent Brief

**Category:** bug
**Summary:** Make in-site search results keyboard-reachable and announced without changing what a query matches.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**, **website**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-882-search-keyboard-live]]; [[ticket-857-search-keyboard-live]].

**TDD:** assert a search keyboard and live-region promise, extend search tests so Arrow Down and Enter follow hits and misses are announced, then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: add and lock `public-site.search:keyboard-live`; keep `public-site.search:titles-headings`, `public-site.a11y:keyboard-small`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: edit/assert promise → test → code. Do not widen the index. Combobox wiring belongs on the search control, not a hamburger.

**Current behavior:**
Typing Dual worlds lists Learn · Dual worlds. A miss shows No matching pages. The field is a plain search input. Arrow Down leaves focus on the field. Enter does not follow the first hit. There is no combobox wiring and no live region.

**Desired behavior:**
Keyboard users can move through visible hits and activate the selected hit, including the first hit on Enter. Screen-reader users hear that results or No matching pages appeared. Dual worlds still hits. A miss still shows No matching pages. Search stays in the side nav. Title and heading match behaviour is unchanged. No Menu button.

**Key interfaces:**
- SiteSearch field and result list. Today: search input, unordered links, miss paragraph.
- Keyboard on the field: Arrow Down, Enter, and existing Escape session work stay distinct. This sitting owns moving through and announcing hits, not clearing query on navigate.
- Accessibility names: expanded or selected-hit wiring on the search control; a live region for hits and misses.

**Acceptance criteria:**
- [x] Contract lists `public-site.search:keyboard-live` with a test pointer
- [x] Tests fail if the search control has no keyboard path through hits and no live announcement of hits or misses
- [x] Dual worlds still hits and a miss still shows No matching pages
- [x] Named vitest file passes and typecheck exits 0
- [x] `public-site.search:titles-headings` and `public-site.a11y:keyboard-small` still hold

**Out of scope:**
- Body-term indexing ([[ticket-821-search-body-terms]])
- Clearing query on navigate or Escape ([[task-874-search-session-reset]])
- Fence Copy announcements ([[ticket-858-copy-announcement]])
- Result clipping; moving search out of the nav; restoring a hamburger

**Explain this part:**
This sitting is search chrome a11y. It does not change what a query matches and does not own session reset. If [[task-874-search-session-reset]] is in flight, do not collide on the same control without reading it first.

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run search` — win. Test Files  1 passed. `pnpm --dir website exec tsc --noEmit` exits 0. Diff locks `public-site.search:keyboard-live`; Dual worlds and miss copy unchanged; combobox wiring is on SiteSearch, not a hamburger.
