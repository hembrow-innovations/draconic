---
id: "slice-598-markdown-loader"
title: "Markdown loader"
kind: slice
status: met
sprint: "website-redesign"
blocked_by: [ "slice-591-nested-src-root-route" ]
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T00:30:00Z"
---
# Markdown loader

## Why

`website/*.md` is the teaching source: `title`, `section`, and `status` frontmatter plus a markdown body. generate.drac hardcodes the page catalog and scans files itself. The Start app should load those files so Install and later Learn and Reference pages stay authored in markdown, not rewritten as JSX. This slice is load-and-parse, not render. Keep generate.drac until [[slice-608-fence-static-deploy]]; the loader is the new catalog path, not a delete of the old Program.

## Done

A loader reads at least `website/install.md` and returns `title`, `section`, `status`, and `body`.

## Blocked by

`[[slice-591-nested-src-root-route]]`: nested `src/` must exist so the loader lives in a lib folder, not a flat dump. Does not wait on home chrome.

## Non-goals

- **[[slice-599-markdown-render]]**: turning the subset into HTML or React
- **[[slice-596-home-hero-cta]]**: homepage copy
- Hardcoding the generate.drac page list as the forever catalog
- Playground
- Next.js
- Deleting `website/generate.drac`
- `cargo test --workspace` as this slice's oracle

## Oracle checklist

- [x] O1: loader returns title, section, status, and body from `install.md`
  CHECK: pnpm --dir website exec vitest run -t "markdown loader install.md"
  EXPECT: Test Files  1 passed
  EVIDENCE: `pnpm --dir website exec vitest run -t "markdown loader install.md"` → Test Files  1 passed | 9 skipped (10); typecheck green

## Pool

Durable links to task ids. Never drop them.

- `[[task-617-markdown-loader]]`

## See also

- [[location-589-public-site]]
- [[ticket-628-public-site-redesign]]
- [[0013-public-site-tanstack-start]]
- [[0010-public-docs-draconic-ssg]]
- docs/specs/draconic/public-site/
- website/
- CONTEXT.md
