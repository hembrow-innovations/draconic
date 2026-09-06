---
id: "slice-608-fence-static-deploy"
title: "Fence and static deploy"
kind: slice
status: met
sprint: "website-redesign"
blocked_by:
  - slice-599-markdown-render
  - slice-597-home-features
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T12:00:00Z"
---

# Fence and static deploy

## Why

Honesty is the contract: shipped `drac` fences must compile, and not-yet pages must not carry fences. Switching the publisher from `website/generate.drac` to a TanStack Start static build must not lie by deleting those tests. Home plus rendered markdown must already exist so the static emit is a site, not an empty adapter.

## Done

`website_pipeline` fence tests still fail and pass as today; retarget readers if HTML emit changes. `pnpm --dir website build` (or the Start static adapter) emits the site. `generate.drac` is no longer the publisher. This slice may update `scripts/generate-website.sh` and `website_pipeline` readers. The public-docs guide already cites [[0013-public-site-tanstack-start]]. Not a playground.

## Blocked by

- [[slice-599-markdown-render]]: static pages are rendered markdown, not generator HTML treated as truth.
- [[slice-597-home-features]]: the emit must include the language homepage, not Learn copied to index.

## Non-goals

- **Playground or in-page runners**: later
- **Docs chrome, Learn, Reference, search, theme, mobile**: [[slice-600-docs-shell]] [[slice-601-learn-hub-nav]] [[slice-602-learn-pages]] [[slice-603-learn-prev-next]] [[slice-604-reference-hub-pages]] [[slice-605-search]] [[slice-606-theme-toggle]] [[slice-607-mobile-a11y]]
- **Next.js**: Start only
- **cargo test --workspace**: not this oracle
- **Serving the docs/ vault as the site**
- **Deleting fence tests to go green**

## Oracle checklist

- [x] O1: website_pipeline fence tests still fail and pass as today, and `pnpm --dir website build` emits the site
  CHECK: pnpm --dir website build && cargo test -p draconic-integration-tests --test website_pipeline
  EXPECT: test result: ok.
  EVIDENCE: pass; `test result: ok.` 12 passed after Start static publish; generate.drac retired

## Pool

Durable links to task ids. Never drop them.

- `[[task-627-fence-static-deploy]]`

## See also

[[location-589-public-site]] [[ticket-628-public-site-redesign]] [[0013-public-site-tanstack-start]] docs/specs/draconic/public-site/ website/ CONTEXT.md public-site.fences:shipped-must-build public-site.fences:forbid-not-yet-fences scripts/generate-website.sh tests/integration/tests/website_pipeline.rs
