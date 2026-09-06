---
id: "slice-590-scaffold-start-app"
title: "Scaffold TanStack Start app"
kind: slice
status: met
sprint: "website-redesign"
blocked_by: []
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T12:55:00Z"
---

# Scaffold TanStack Start app

## Why

The public site is still a markdown dump plus `website/generate.drac` emitting HTML. That Program is a renderer, not a language homepage. TanStack Start is the new app runtime: SSR and static output plus file routes, so pages are a real app instead of a generated catalog. pnpm and TypeScript only match the frontend stack. npm, yarn, bun, and JavaScript product files are out. Start replaces generate.drac as renderer; do not delete generate.drac until [[slice-608-fence-static-deploy]].

## Done

`website/package.json` exists. A TanStack Start app installs under `website/` and typechecks. TypeScript only. pnpm only.

## Blocked by

None.

## Non-goals

- **[[slice-591-nested-src-root-route]]**: nested `src/` discipline and proving `/` renders
- **[[slice-592-semantic-tokens]]** through [[slice-599-markdown-render]]: tokens, type, chrome, home, markdown
- Playground
- Next.js
- Deleting `website/generate.drac`
- Using the vault as the site
- `cargo test --workspace` as this slice's oracle

## Oracle checklist

- [x] O1: website Start app installs and typechecks
  CHECK: pnpm --dir website exec tsc --noEmit
  EXPECT: success with no errors
  EVIDENCE: met exit=0 no errors; website/package.json present; generate.drac kept; commit f8532934e575599185d5e3337684ea50a4ed32a4 at=2026-09-06T12:55:00Z

## Pool

Durable links to task ids. Never drop them.

- `[[task-609-scaffold-start-app]]`

## See also

- [[location-589-public-site]]
- [[ticket-628-public-site-redesign]]
- [[0013-public-site-tanstack-start]]
- docs/specs/draconic/public-site/
- website/
- CONTEXT.md
