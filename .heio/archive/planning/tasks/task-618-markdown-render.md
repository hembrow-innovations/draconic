---
id: "task-618-markdown-render"
title: "Render the markdown subset"
kind: task
status: completed
mode: afk
blocked_by: ["task-617-markdown-loader"]
sprint: "website-redesign"
slice: "slice-599-markdown-render"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T14:34:53Z"
---

# Render the markdown subset

## Blocked by

[[task-617-markdown-loader]]: rendering needs loaded title, section, status, and body.

## Done

The markdown subset (headings, paragraphs, lists, fenced code, links) renders, and the Install body is proof.

## Context

Public pages are not MDX and not the vault. The locked subset is headings, paragraphs, lists, fenced code, and links. Prove it by rendering Install. Fenced `drac` compile checks wait for [[task-627-fence-static-deploy]].

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

Install body renders headings, paragraphs, lists, fences, and links. Typecheck holds.

scope: markdown renderer under `website/src/` plus proof that Install body renders

## Links

[[slice-599-markdown-render]] [[task-617-markdown-loader]]

## Agent Brief

**Category:** enhancement
**Summary:** Render the public markdown subset and prove it with the Install body.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.markdown:subset`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
`generate.drac` walks lines and emits a small HTML subset. The Start loader yields raw markdown. No renderer yet.

**Desired behavior:**
Renderer covers headings, paragraphs, lists, fenced code, and links. Install body is the proof page (route or fixture). Not the vault. Not MDX component islands unless a fence wrapper is required. Do not add in-page runners.

**Key interfaces:**
- Renderer module next to the loader
- Subset: headings, paragraphs, lists, fenced code, links
- Install body as the acceptance fixture (`website/install.md`)
- Load **frontend-development** and **spec**

**Acceptance criteria:**
- [x] Headings, paragraphs, lists, fenced code, and links render
- [x] Install body is visibly rendered (not raw markdown)
- [x] Vault `docs/` is not rendered as the site
- [x] No playground or in-page runner
- [x] If [[task-617-markdown-loader]] is not `completed`, stop

## Gauntlet

- **Round 1:** `pnpm --dir website exec vitest run -t "markdown render install subset"` plus `pnpm --dir website typecheck` — win. Critic: `src/lib/content/renderMarkdown` seam next to the loader, Install body as fixture, headings/paragraphs/lists/fences/links HTML, no docs/ vault, no playground, no docs shell.

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Docs shell ([[task-619-docs-shell]])
- Fence compile CI ([[task-627-fence-static-deploy]])
- Rewriting `website/install.md`

**Explain this part:**
The public subset is headings, paragraphs, lists, fenced code, and links. That is enough to teach and to show copy-paste samples. It is not the vault, and it is not MDX components yet unless a fence wrapper is required. Keep the surface small so shipped fences stay ordinary markdown.
