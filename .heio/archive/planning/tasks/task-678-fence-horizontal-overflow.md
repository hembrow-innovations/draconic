---
id: "task-678-fence-horizontal-overflow"
title: "Contain code fences in the small viewport"
kind: task
status: completed
mode: afk
blocked_by:
  - task-690-first-h2-border
sprint: "website-odm-match"
slice: "slice-677-fence-horizontal-overflow"
area: public-site
tags: [website, public-site]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-07T23:58:00Z"
---

# Contain code fences in the small viewport

## Blocked by

[[task-690-first-h2-border]]: same docs-article CVA surface.

## Done

At 375 by 812, docs article fences do not widen the page. Long fences may scroll inside themselves.

## Context

Walked `/cli` at 375 by 812. Document scrollWidth 461. Overflowing node is `CODE` inside a fence. Article `pre` uses visible overflow. `public-site.a11y:keyboard-small` says article reading works at a small viewport. Do not drop fence compile.

## Verify

`pnpm --dir website exec vitest run docs-shell mobile-a11y` prints `Test Files  2 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `website/src/features/docs/DocsShell/`, `website/src/tests/docs-shell.test.ts`, `website/src/tests/mobile-a11y.test.ts`

## Links

[[slice-677-fence-horizontal-overflow]] [[ticket-648-fence-horizontal-overflow]] [[task-690-first-h2-border]]

## Agent Brief

**Category:** bug
**Summary:** Keep long code fences from creating a horizontal page scrollbar at a small viewport.

**Drain:** `/afk-task`. If [[task-690-first-h2-border]] is not `completed`, stop. Do not claim.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-677-fence-horizontal-overflow]]; [[ticket-648-fence-horizontal-overflow]].

**TDD:** extend docs-shell tests first so fence `pre` chrome contains horizontal overflow. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.a11y:keyboard-small`, `public-site.fences:shipped-must-build`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: assert the existing promises, then test, then code. Do not drop fences.

**Current behavior:**
Docs article `pre` overflow is visible. On `/cli` at 375 wide the first fence's code box extends past the viewport and the page grows a horizontal scrollbar.

**Desired behavior:**
Article reading at a small viewport does not require horizontal page scroll because of fences. Fences may scroll internally. Shipped fences still compile. Tokens and CVA only.

**Key interfaces:**
- DocsShell article CVA fence (`pre` / `code`) chrome
- Small-viewport shell already stacked by mobile wrap

**Acceptance criteria:**
- [x] Tests fail if fence `pre` chrome leaves overflow visible
- [x] Named vitest files pass and typecheck exits 0
- [x] Promise ids listed above still hold
- [x] No playground and no vault-as-site

**Out of scope:**
- Markdown subset escaping ([[task-680-unescaped-angle-placeholders]]); fence compile pipeline; playground

**Explain this part:**
Keyboard-small already promises article reading at a small viewport. This sitting only stops fences from breaking that.

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run docs-shell mobile-a11y` — win. Output `Test Files  2 passed (2)`. `tsc --noEmit` exit 0. Diff vs `public-site.a11y:keyboard-small` and `public-site.fences:shipped-must-build`: article CVA fence `pre` uses `overflow-x-auto`; tests fail on visible overflow; fence compile untouched; no playground or vault-as-site.
