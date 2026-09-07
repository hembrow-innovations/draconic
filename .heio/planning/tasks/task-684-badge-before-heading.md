---
id: "task-684-badge-before-heading"
title: "Paint the status badge after the article heading"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "website-odm-match"
slice: "slice-683-badge-before-heading"
area: public-site
tags: [website, public-site]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-07T23:15:00Z"
---

# Paint the status badge after the article heading

## Blocked by

None.

## Done

Learn and Reference articles show kicker, then heading, then shipped or not-yet badge.

## Context

Purpose in-scope for the docs article is a section kicker, heading, status badge, and related-link footer. DocsShell currently renders kicker, Badge, then markdown that starts with `h1`. `public-site.chrome:docs-sidebar` only requires a badge from frontmatter, not that order. Lock the purpose order in contract if it is still only purpose text, then test, then code.

## Verify

`pnpm --dir website exec vitest run docs-shell` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `docs/specs/draconic/public-site/` only if asserting order, `website/src/features/docs/DocsShell/`, `website/src/tests/docs-shell.test.ts`

## Links

[[slice-683-badge-before-heading]] [[ticket-651-badge-before-heading]]

## Agent Brief

**Category:** enhancement
**Summary:** Match docs-article purpose order: kicker, heading, then status badge.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-683-badge-before-heading]]; [[ticket-651-badge-before-heading]].

**TDD:** extend docs-shell tests first so article children are kicker, heading, then Badge. If contract does not yet name that order, assert a promise first. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.chrome:docs-sidebar`; assert `public-site.chrome:docs-article-order` if order is still purpose-only
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent a different order than purpose

**Current behavior:**
On Learn and Reference pages the article children are kicker, then the shipped or not-yet Badge, then the markdown `h1`.

**Desired behavior:**
Article column is section kicker, page heading, status badge, remaining markdown, related-link footer. Home still does not use DocsShell. Badge still comes from frontmatter status.

**Key interfaces:**
- DocsShell article children versus markdown `h1`
- Badge from frontmatter `shipped` / `not-yet`

**Acceptance criteria:**
- [ ] Tests fail if Badge is rendered before the article heading
- [ ] Kicker, badge, and footer still exist
- [ ] Named vitest file passes and typecheck exits 0
- [ ] Home still does not use DocsShell

**Out of scope:**
- First `h2` border ([[task-690-first-h2-border]]); changing status words; playground

**Explain this part:**
Purpose already names the order. This sitting implements that order; it does not restyle the badge.

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run docs-shell` — expect `Test Files  1 passed`. Typecheck holds.
