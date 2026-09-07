---
id: "task-680-unescaped-angle-placeholders"
title: "Escape angle-bracket placeholders in article lists"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "website-odm-match"
slice: "slice-679-unescaped-angle-placeholders"
area: public-site
tags: [website, public-site]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-07T19:15:00Z"
---

# Escape angle-bracket placeholders in article lists

## Blocked by

None.

## Done

`website/cli.md` Commands list renders `draconic parse <file>` and `[-o <out>]` as visible text, not leftover HTML tags.

## Context

`renderInline` copies list text through without escaping, so `<file>` and `<out>` become tags. The markdown subset still includes lists and paragraphs from `website/` sources. Links must keep working.

## Verify

`pnpm --dir website exec vitest run markdown-render` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `website/src/lib/content/`, `website/src/tests/markdown-render.test.ts`, `tests/integration/tests/website_pipeline.rs` only if the subset lock needs the same escape proof

## Links

[[slice-679-unescaped-angle-placeholders]] [[ticket-649-unescaped-angle-placeholders]]

## Agent Brief

**Category:** bug
**Summary:** Show angle-bracket placeholders in public markdown lists as text.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-679-unescaped-angle-placeholders]]; [[ticket-649-unescaped-angle-placeholders]].

**TDD:** extend markdown-render tests first so CLI Commands HTML contains escaped `<file>` and `<out>` text, not raw tags. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.markdown:subset`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: assert the existing promise, then test, then code

**Current behavior:**
On `/cli`, Commands items read `draconic parse ` and `[-o ]`. `<out>` shows up as a leftover generic DOM node because list text is not escaped.

**Desired behavior:**
Headings, paragraphs, and list items encode `<`, `>`, and `&` so placeholders stay visible characters. Complete markdown links still become anchors. Fences still emit `pre`/`code`. The subset does not grow new syntax.

**Key interfaces:**
- `renderMarkdown` / `renderInline` HTML encoding for non-link text
- CLI page body as the proof source

**Acceptance criteria:**
- [x] Tests fail if `renderMarkdown` of CLI Commands drops `<file>` or `<out>`
- [x] Link rendering still produces anchors
- [x] Named vitest file passes and typecheck exits 0
- [x] `public-site.markdown:subset` still holds

**Out of scope:**
- Fence overflow chrome; changing CLI teaching copy unless required to keep placeholders; playground

**Explain this part:**
The subset already promises lists from `website/` sources. Visible placeholders are part of that text, not a new markdown feature.

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run markdown-render` — win. Output `Test Files  1 passed (1)`. `tsc --noEmit` exit 0. Diff vs `public-site.markdown:subset`: lists and paragraphs still render; placeholders encoded as text; links still anchors; fences still `pre`/`code`; subset syntax unchanged.
