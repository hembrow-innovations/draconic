---
id: "task-870-distinct-nav-labels"
title: "Distinguish Learn and Reference chapter accessible names"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "website-chrome-polish"
slice: "slice-869-distinct-nav-labels"
area: public-site
tags: [website, public-site]
created_at: "2026-09-12T07:05:00Z"
updated_at: "2026-09-12T07:05:00Z"
---

# Distinguish Learn and Reference chapter accessible names

## Blocked by

None.

## Done

Chapter links that share a CONTEXT term have distinct accessible names for Learn versus Reference. Visible CONTEXT terms stay.

## Context

Side nav shows two host I/O links and two packages links with different URLs and identical visible text. Search already prefixes `Learn ·` versus `Reference ·`. Do not rewrite CONTEXT terms. Accessible names may use the same Learn versus Reference distinction search already ships.

## Verify

`pnpm --dir website exec vitest run learn-hub-nav reference-hub-pages` prints `Test Files  2 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `docs/specs/draconic/public-site/`, `website/src/features/learn/LearnNav/`, `website/src/features/reference/ReferenceNav/`, matching website tests

## Links

[[slice-869-distinct-nav-labels]] [[ticket-850-duplicate-nav-labels]]

## Agent Brief

**Category:** bug
**Summary:** Give Learn and Reference chapter links that share a CONTEXT term distinct accessible names.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**, **website**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-869-distinct-nav-labels]]; [[ticket-850-duplicate-nav-labels]].

**TDD:** assert a contract promise for distinct accessible names, point Learn and Reference nav tests at host I/O and packages pairs, then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: add and lock `public-site.a11y:distinct-nav-names`; keep `public-site.ia:learn-walkable`, `public-site.ia:reference-walkable`, `public-site.a11y:keyboard-small`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: edit/assert promise → test → code. Do not rewrite CONTEXT terms.

**Current behavior:**
Learn host I/O and Reference host I/O both expose the accessible name host I/O. Same for packages. Keyboard and screen-reader users cannot tell the destinations apart by name.

**Desired behavior:**
Each shared CONTEXT term has distinct accessible names for the Learn link versus the Reference link, in the same spirit as search labels (`Learn ·` versus `Reference ·`). Visible text may still be the CONTEXT term. URLs and IA stay.

**Key interfaces:**
- LearnNav and ReferenceNav link accessible names (`aria-label` or equivalent)
- Search hit labels already show the Learn versus Reference prefix; reuse that distinction, do not invent new chapter names

**Acceptance criteria:**
- [ ] Contract lists `public-site.a11y:distinct-nav-names` with a test pointer
- [ ] Tests fail if both host I/O links (and both packages links) share one accessible name
- [ ] Named vitest files pass and typecheck exits 0
- [ ] Visible CONTEXT terms are not renamed

**Out of scope:**
- Body-term search ([[ticket-821-search-body-terms]]); changing hrefs; playground

**Explain this part:**
The terms stay. Only the accessible name must tell Learn from Reference.
