---
id: "slice-599-markdown-render"
title: "Markdown render"
kind: slice
status: met
sprint: "website-redesign"
blocked_by: ["slice-598-markdown-loader"]
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T00:38:00Z"
---

# Markdown render

## Why

ADR-0010 kept a small markdown subset on purpose: headings, paragraphs, lists, code fences, and links, plus status frontmatter. That is what generate.drac renders and what fence-compile CI understands. The Start renderer must honor the same subset so teaching pages stay portable and we do not grow a second dialect. A unit test or a route that shows Install content proves the loader output becomes HTML or React. Promise `public-site.markdown:subset`.

## Done

A renderer turns that subset into HTML or React. A unit test or route shows Install content from `website/install.md`.

## Blocked by

`[[slice-598-markdown-loader]]`: render consumes loaded `title`, `section`, `status`, and `body`.

## Non-goals

- **[[slice-598-markdown-loader]]**: frontmatter parse already owned
- Expanding the subset (tables, MDX, shortcodes)
- Playground or in-page runners
- Next.js
- Deleting `website/generate.drac`
- `cargo test --workspace` as this slice's oracle

## Oracle checklist

- [x] O1: renderer turns headings, paragraphs, lists, fences, and links into HTML or React and Install content is visible
  CHECK: pnpm --dir website exec vitest run -t "markdown render install subset"
  EXPECT: Test Files  1 passed
  EVIDENCE: `pnpm --dir website exec vitest run -t "markdown render install subset"` → Test Files  1 passed | 10 skipped (11); commit 6d891632

## Pool

Durable links to task ids. Never drop them.

- `[[task-618-markdown-render]]`

## See also

- [[location-589-public-site]]
- [[ticket-628-public-site-redesign]]
- [[0013-public-site-tanstack-start]]
- [[0010-public-docs-draconic-ssg]]
- docs/specs/draconic/public-site/
- website/
- CONTEXT.md
- Promise: public-site.markdown:subset
