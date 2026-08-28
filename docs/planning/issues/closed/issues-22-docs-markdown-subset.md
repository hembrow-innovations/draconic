---
id: issues-22
created_at: "2026-08-28"
updated_at: "2026-08-29"
area: planning
domain: language
title: "Docs SSG: markdown subset"
description: "Generator renders headings, paragraphs, lists, fenced code, and links from the markdown subset."
status: closed
issue-type: feature-request
severity: medium
tags:
  - planning
  - issue
  - enhancement
  - docs
  - closed
---

# Docs SSG: markdown subset

## Parent

[[issues-19-language-docs-site]]

## Description / What to build

Authors write a markdown subset and see headings, paragraphs, lists, fenced code, and links in the generated HTML. Enough to write Learn and Reference, not CommonMark.

## Blocked by

- [[issues-20-docs-ssg-one-page]]

## Acceptance criteria

- [x] A page using headings, paragraphs, lists, fenced code, and links renders those structures in HTML
- [x] Pipeline compiles the generator and asserts the rendered subset
- [x] No full CommonMark, MDX, or page DSL in Programs

## Agent Brief

### Goal

Implement only the locked markdown subset in the Draconic generator.

### Contract

- Subset: headings, paragraphs, lists, fenced code, links. Frontmatter already belongs to [[issues-21-docs-learn-reference-nav]] if that ticket has landed; if not, title-only is enough here.
- Generator stays a native Program.
- Pipeline seam. Do not extract or compile fences here.

### Out of scope

Fence compile policy, Learn/Reference chapter content, GitHub Pages, CommonMark completeness.

## Comments

> Child of [[issues-19-language-docs-site]]. Parallel with [[issues-21-docs-learn-reference-nav]].
>
> **2026-08-29:** Landed. `website/generate.drac` renders headings, paragraphs, lists, fenced code, and links. Native lowering: `host_docs` (file I/O plus string scan). Pipeline test: `tests/integration/tests/website_pipeline.rs` (`website_pipeline_renders_markdown_subset`). Parent [[issues-19-language-docs-site]] stays open.
