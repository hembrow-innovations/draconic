---
id: "slice-629-odm-tokens"
title: "ODM semantic tokens"
kind: slice
status: met
sprint: "website-odm-match"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T21:30:00Z"
---

# ODM semantic tokens

## Why

The live site is cool TypeScript blue on light canvas. The ODM public site is dark navy with mint current-page and blue accent. Tokens have to move first so later chrome can use names, not hex.

## Done

Dark theme canvas, ink, accent, muted, and line match the ODM surfaces. A mint accent-2 token exists. Product TS still has no hex.

## Blocked by

None.

## Non-goals

- **Two-column shell**: [[slice-630-site-shell]]
- **Home bands and docs article**: later slices
- **Dropping light theme or the theme toggle**
- **Playground, vault-as-site, Start replacement**
- **cargo test --workspace**

## Oracle checklist

- [x] O1: dark canvas, ink, accent, muted, and line match ODM; accent-2 exists; no hex in product TS
  CHECK: pnpm --dir website exec vitest run semantic-tokens
  EXPECT: Test Files  1 passed
  EVIDENCE: `pnpm --dir website exec vitest run semantic-tokens` → Test Files  1 passed (1); task-636 completed in archive

## Pool

- `[[task-636-odm-tokens]]`

## See also

[[location-589-public-site]] [[website-odm-match]] [[0013-public-site-tanstack-start]] docs/specs/draconic/public-site/ website/src/styles/theme.css
