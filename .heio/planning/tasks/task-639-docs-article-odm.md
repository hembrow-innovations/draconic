---
id: "task-639-docs-article-odm"
title: "Restyle docs article to ODM"
kind: task
status: ready
mode: afk
blocked_by: [ "task-636-odm-tokens", "task-637-site-shell" ]
sprint: "website-odm-match"
slice: "slice-632-docs-article-odm"
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T18:00:00Z"
---

# Restyle docs article to ODM

## Blocked by

[[task-636-odm-tokens]]: prose uses tokens.
[[task-637-site-shell]]: article sits in site main.

## Done

Learn and Reference articles show kicker, heading, status badge, ODM-styled markdown, and a related-link footer. Home still does not use DocsShell.

## Context

ODM article column: section kicker (Start / Guides / Reference), one h1, optional muted lede, h2 with top border except the first, inline code and pre on code-bg, callout with 3px left bar if needed, footer of related links. Keep Badge from frontmatter. Keep markdown subset. LearnPager can be the related-link footer. Do not change fence rules. Style rendered markdown via article variants, not hex in TS, and not a new markdown language.

## Verify

`pnpm --dir website exec vitest run docs-shell markdown-render` passes. Typecheck holds.

scope: `website/src/features/docs/`, `website/src/features/learn/LearnPage/`, `website/src/features/learn/LearnPager/`, `website/src/features/reference/ReferencePage/`, `website/src/lib/content/renderMarkdown.ts`, `website/src/tests/docs-shell.test.ts`, `website/src/tests/markdown-render.test.ts`

## Links

[[slice-632-docs-article-odm]] [[task-636-odm-tokens]] [[task-637-site-shell]]

## Agent Brief

**Category:** enhancement
**Summary:** Make the docs article column match ODM kicker, type, code, and footer.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.chrome:docs-sidebar`, `public-site.markdown:subset`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]

**Current behavior:**
DocsShell is aside plus unstyled markdown HTML plus Badge. LearnPager is a prev/next row.

**Desired behavior:**
Article wrap: section kicker, h1, Badge, ODM prose (code surface, h2 rules), related-link footer. Aside may still exist until [[task-640-docs-nav-groups]]. Home and root still do not import DocsShell.

**Acceptance criteria:**
- [ ] DocsShell still aside + article + Badge
- [ ] Kicker and ODM prose styles via tokens/CVA
- [ ] markdown-render subset unchanged
- [ ] Named tests pass

**Out of scope:**
- Site nav groups; hub cards; fence pipeline; playground
