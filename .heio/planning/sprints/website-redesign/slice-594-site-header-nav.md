---
id: "slice-594-site-header-nav"
title: "Site header nav"
kind: slice
status: met
sprint: "website-redesign"
blocked_by: [ "slice-593-typography-badge" ]
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T23:20:00Z"
---
# Site header nav

## Why

Primary nav is the public IA, not decoration. Every page should answer: this is Draconic, Learn is the path, Reference is the working pages, GitHub is the source. generate.drac already emitted Learn, Reference, and GitHub as a string. The Start header makes that chrome a component on `/` (and later every route) with a wordmark plus those three targets. Promise `public-site.chrome:primary-nav`.

## Done

A site header on `/` exposes four targets: wordmark, Learn, Reference, GitHub.

## Blocked by

`[[slice-593-typography-badge]]`: header type and any status chrome should use the type scale and variants, not ad-hoc spans.

## Non-goals

- **[[slice-595-site-footer-skip]]**: skip link and footer
- **[[slice-596-home-hero-cta]]**: hero pitch
- Learn or Reference section subnav
- Playground
- Next.js
- `cargo test --workspace` as this slice's oracle

## Oracle checklist

- [x] O1: `/` header includes wordmark, Learn, Reference, and GitHub
  CHECK: pnpm --dir website exec vitest run -t "site header primary nav"
  EXPECT: Test Files  1 passed
  EVIDENCE: `pnpm --dir website exec vitest run -t "site header primary nav"` → Test Files  1 passed | 3 skipped (4); Tests  1 passed | 3 skipped (4); commit 54214b5

## Pool

Durable links to task ids. Never drop them.

- `[[task-613-site-header-nav]]`

## See also

- [[location-589-public-site]]
- [[ticket-628-public-site-redesign]]
- [[0013-public-site-tanstack-start]]
- docs/specs/draconic/public-site/
- website/
- CONTEXT.md
- Promise: public-site.chrome:primary-nav
