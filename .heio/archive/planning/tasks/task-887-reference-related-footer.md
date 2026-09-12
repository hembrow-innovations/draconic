---
id: "task-887-reference-related-footer"
title: "Add a related-link footer on Reference articles"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "website-chrome-polish"
slice: "slice-886-reference-related-footer"
area: public-site
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T18:50:00Z"
---

# Add a related-link footer on Reference articles

## Blocked by

None.

## Done

Reference working pages render a related-link footer that follows the locked Reference sequence.

## Context

`public-site.chrome:docs-article-order` already names a related-link footer on Learn and Reference. Learn fills it. Reference working pages pass no children. Docs-shell tests are a false green. Do not add a new promise. Make the test pointer honest, then code.

## Verify

`pnpm --dir website exec vitest run reference-prev-next docs-shell` prints `Test Files  2 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `docs/specs/draconic/public-site/test.md`, `website/src/features/reference/`, `website/src/tests/reference-prev-next.test.ts`, `website/src/tests/docs-shell.test.ts`

## Links

[[slice-886-reference-related-footer]] [[ticket-859-reference-related-footer]]

## Agent Brief

**Category:** bug
**Summary:** Put a related-link footer on Reference working pages so the docs-article-order promise holds on Reference, not only Learn.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**, **website**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-886-reference-related-footer]]; [[ticket-859-reference-related-footer]].

**TDD:** do not add a new promise. Point tests at the existing related-link footer clause so they fail while ReferencePage passes no children, then implement a Reference footer only. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: keep `public-site.chrome:docs-article-order` and `public-site.ia:reference-walkable`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract already locked. Make the test pointer honest, then code. Do not rewrite LearnPager.

**Current behavior:**
Learn chapters pass a sequence pager into the docs article footer. Reference working pages use ReferencePage with no children, so DocsShell renders no footer. The Reference hub already passes hub cards as children. `docs-shell` tests allow that gap.

**Desired behavior:**
Each Reference working page ends with a related-link footer that walks CLI, types, Dual-world rules, host I/O, and packages. First and last stops match that sequence. Learn pager, Learn sequence, and Reference hub cards stay as they are. Teaching copy is unchanged.

**Key interfaces:**
- ReferencePage children into the existing DocsShell footer slot
- A Reference sequence helper and pager, parallel to Learn, not a Learn rewrite

**Acceptance criteria:**
- [x] Tests fail if ReferencePage still passes no footer children
- [x] `/cli` and the other working pages have article-footer related links; the hub is not rewritten
- [x] Named vitest files pass and typecheck exits 0
- [x] LearnPager source is not rewritten as the mechanism

**Out of scope:**
- Learn pager rewrite; hub cards; teaching copy; playground

**Explain this part:**
The promise already names the footer on Reference. This sitting closes a false green. It is not a Learn IA change.

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run reference-prev-next docs-shell` — win. Test Files  2 passed. `pnpm --dir website exec tsc --noEmit` exits 0. Diff keeps `public-site.chrome:docs-article-order` and `public-site.ia:reference-walkable`; LearnPager is untouched; hub cards stay; teaching copy is unchanged.
