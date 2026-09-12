---
id: "task-895-search-body-terms"
title: "Index teaching-page body terms"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "website-search-index"
slice: "slice-894-search-body-terms"
area: public-site
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T19:30:00Z"
---

# Index teaching-page body terms

## Blocked by

None.

## Done

A body-only teaching term finds its Learn or Reference page; vault phrases still miss.

## Context

The finder matches title and heading text only. Dual worlds, CLI verbs that are headings, and duplicate packages labels already work. Teaching-body terms such as FizzBuzz, localStorage, and u64 miss. Vault phrases must stay out. Edit purpose and `public-site.search:titles-headings`, then test, then code. Not chrome polish.

## Verify

`pnpm --dir website exec vitest run search` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `docs/specs/draconic/public-site/`, `website/src/lib/search/`, `website/src/components/SiteSearch/`, `website/src/tests/search.test.ts`

## Links

[[slice-894-search-body-terms]] [[ticket-821-search-body-terms]]

## Agent Brief

**Category:** enhancement
**Summary:** Widen in-site search so a visitor can find a Learn or Reference page by a term in that page’s teaching body, not only title or heading, while vault phrases still miss.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**, **website**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-894-search-body-terms]]; [[ticket-821-search-body-terms]].

**TDD:** edit purpose and the titles-headings promise first so teaching-page body terms are in scope, point search tests at a current body-only hit plus unchanged vault misses, then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: edit and lock `public-site.search:titles-headings` so a visitor can find a Learn or Reference page by title, heading, or teaching-page body text; keep `public-site.forbid-vault-as-site`
- Purpose: [[docs/specs/draconic/public-site/purpose]] — edit the in-scope search line to match; do not lift the vault-as-site or playground fences
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: edit/assert promise → test → code. Do not invent fuzz ranking or chrome behaviour.

**Current behavior:**
The finder matches title and heading text only. Dual worlds, CLI verbs that are headings, i32 in the i32 and i64 heading, and duplicate packages labels already work. A miss already shows No matching pages. Teaching-body terms that are not title or heading, such as FizzBuzz, localStorage, and u64, miss. Vault phrases miss, and that must stay true.

**Desired behavior:**
A visitor can find a routed Learn or Reference page by a term that appears in that page’s teaching body. Title and heading hits still work, including Dual worlds and heading hashes. Learn versus Reference labels still distinguish duplicate titles. A miss still shows No matching pages. Phrases from the agent vault still miss. Search stays in the side nav.

**Key interfaces:**
- Search entry shape. Today it is title, headings, href, and section. Body from routed teaching pages must participate in match, without indexing vault notes.
- Query helper. A needle present only in teaching body must return that page.
- Hit href and label. Title and heading hits keep current page and section-hash behaviour. A body-only hit may land on the page without a new fragment rule.

**Acceptance criteria:**
- [x] Purpose in-scope search line and `public-site.search:titles-headings` include teaching-page body terms
- [x] Tests fail if a current body-only teaching term such as FizzBuzz, localStorage, or u64 does not hit its page
- [x] Vault phrases still miss, including tracing GC, Ownership-only, Public site purpose, and Give someone writing a Program
- [x] Dual worlds, heading hashes, miss copy, and Learn versus Reference labels still hold
- [x] Named vitest file passes and typecheck exits 0

**Out of scope:**
- Keyboard and live-region wiring ([[ticket-857-search-keyboard-live]])
- Clearing query on navigate or Escape ([[task-874-search-session-reset]])
- Publishing `docs/` as the site, playground, fuzz ranking, rewriting CONTEXT terms

**Explain this part:**
This sitting is the match set. It is allowed to edit the existing search promise. It is not a chrome or session sitting.

## Gauntlet

- **Round 1:** `pnpm --dir website exec vitest run search` and `pnpm --dir website exec tsc --noEmit`. Win. Vitest printed `Test Files  1 passed`. tsc exited 0. Gap: none. Stale vault-miss examples tracing GC and Ownership-only now appear in teaching copy; filed [[ticket-902-stale-vault-search-misses]].
