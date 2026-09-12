---
id: "task-893-outline-current-section"
title: "Mark the heading in view on On this page"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "website-chrome-polish"
slice: "slice-892-outline-current-section"
area: public-site
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Mark the heading in view on On this page

## Blocked by

None.

## Done

The on-page outline marks the heading in view; sibling outline links stay unmarked. Hashes still work.

## Context

Promise `public-site.chrome:on-page-toc` currently requires the list only. Side nav already uses `aria-current=page` for the open article. Edit the existing promise, then test, then code. Do not invent a sticky outline column.

## Verify

`pnpm --dir website exec vitest run docs-shell` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `docs/specs/draconic/public-site/`, `website/src/features/docs/OnThisPage/`, `website/src/tests/docs-shell.test.ts`

## Links

[[slice-892-outline-current-section]] [[ticket-862-outline-current-section]]

## Agent Brief

**Category:** enhancement
**Summary:** Mark the heading in view on the On this page outline so it is not just another muted sibling link.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**, **website**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-892-outline-current-section]]; [[ticket-862-outline-current-section]].

**TDD:** edit `public-site.chrome:on-page-toc` so the outline must mark the heading in view, extend docs-shell so a matching heading hash cannot leave every outline link with `aria-current` null and the same muted class, then implement, then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.chrome:on-page-toc` (keep the heading list; require current-section marking). Keep `public-site.chrome:current-page` on side nav as `aria-current=page`.
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: edit the existing promise → test → code. Do not invent a sticky outline column.

**Current behavior:**
Install outline links From source, Zed editor, and Reproducibility all use the same muted class and `aria-current` null, including after `#zed-editor`. Hashes already scroll. The outline stays in-flow under the title.

**Desired behavior:**
The outline still lists heading fragment links. The heading in view is marked, including after opening a heading permalink. Sibling outline links are not marked. Use `aria-current` on the current outline link, not `page` (side nav already uses `page` for the open article). Keep the outline in-flow; do not add a sticky third column.

**Key interfaces:**
- OnThisPage list links and CVA
- Existing outline ids that already match heading permalinks

**Acceptance criteria:**
- [ ] Contract `public-site.chrome:on-page-toc` requires marking the heading in view, not only listing links
- [ ] Tests fail if OnThisPage never sets `aria-current` on the current heading link
- [ ] Named vitest file passes and typecheck exits 0
- [ ] Side-nav `aria-current=page` is unchanged

**Out of scope:**
- Sticky TOC; current-page nav contrast ([[task-868-current-page-contrast]]); heading permalink generation; playground

**Explain this part:**
The list already exists. This sitting is current-section state, not a second outline.
