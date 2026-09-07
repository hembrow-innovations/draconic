---
id: "task-690-first-h2-border"
title: "Give the first article h2 the section rule"
kind: task
status: ready
mode: afk
blocked_by:
  - task-684-badge-before-heading
sprint: "website-odm-match"
slice: "slice-689-first-h2-border"
area: public-site
tags: [website, public-site]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-07T23:15:00Z"
---

# Give the first article h2 the section rule

## Blocked by

[[task-684-badge-before-heading]]: article child order settles first.

## Done

Every article `h2` after the page `h1`, including the first section heading, has the token top border.

## Context

Docs article CVA sets a line on `h2`, then zeroes `h2:first-of-type`. That selector matches the first `h2` even when an `h1` already sits above it. Walked `/install` Reproducibility with no border; `/cli` Commands with no border and later `h2`s with a 1px line. Slice-632 already wanted bordered h2s. Existing docs-shell tests currently require the `first-of-type` zero — update those tests.

## Verify

`pnpm --dir website exec vitest run docs-shell` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `website/src/features/docs/DocsShell/`, `website/src/tests/docs-shell.test.ts`

## Links

[[slice-689-first-h2-border]] [[ticket-654-first-h2-no-border]] [[task-684-badge-before-heading]]

## Agent Brief

**Category:** enhancement
**Summary:** Apply the ODM section rule to the first article `h2` after the page heading.

**Drain:** `/afk-task`. If [[task-684-badge-before-heading]] is not `completed`, stop. Do not claim.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-689-first-h2-border]]; [[ticket-654-first-h2-no-border]]; [[slice-632-docs-article-odm]].

**TDD:** change docs-shell tests first so they no longer require zeroing `h2:first-of-type`, and instead require the section border on `h2` after `h1`. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.chrome:docs-sidebar` (article chrome). No new product color or copy.
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: this completes bordered h2s already named in the ODM docs-article slice Done. Do not invent a new heading scale.

**Current behavior:**
`h2:first-of-type` drops the top border, so the first section heading never gets the rule even when an `h1` precedes it.

**Desired behavior:**
Section `h2`s, including the first one after the article `h1`, use the existing token top border, margin, and padding. Home still does not use DocsShell. Tokens and CVA only.

**Key interfaces:**
- DocsShell article CVA heading selectors
- Existing `border-line` section rule

**Acceptance criteria:**
- [ ] Tests fail if `h2:first-of-type` zeroes the section border when an `h1` exists
- [ ] Named vitest file passes and typecheck exits 0
- [ ] Home still does not use DocsShell
- [ ] No hex

**Out of scope:**
- Fence overflow ([[task-678-fence-horizontal-overflow]]); changing heading copy; playground

**Explain this part:**
The restyle already intended bordered section headings. The first-of-type selector is why the first section never got them.

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run docs-shell` — expect `Test Files  1 passed`. Typecheck holds.
