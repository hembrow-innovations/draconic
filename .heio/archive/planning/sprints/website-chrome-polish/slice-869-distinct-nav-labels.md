---
id: "slice-869-distinct-nav-labels"
title: "Distinct Learn and Reference chapter names"
kind: slice
status: met
sprint: "website-chrome-polish"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-12T07:05:00Z"
updated_at: "2026-09-12T17:55:00Z"
---

# Distinct Learn and Reference chapter names

## Why

The side nav exposes two links named host I/O and two named packages with different URLs. Keyboard and screen-reader users cannot tell Learn from Reference by the link name. Search already prefixes `Learn ·` versus `Reference ·`.

## Done

Each pair of chapter links that share a CONTEXT term has distinct accessible names for Learn versus Reference. Visible CONTEXT terms are not rewritten.

## Blocked by

None.

## Non-goals

- **Renaming host I/O or packages in CONTEXT**
- **Changing IA or URLs**
- **Body-term search indexing**: [[ticket-821-search-body-terms]]

## Oracle checklist

- [x] O1: Learn and Reference chapter links that share a CONTEXT term have distinct accessible names
  CHECK: pnpm --dir website exec vitest run learn-hub-nav reference-hub-pages
  EXPECT: Test Files  2 passed
  EVIDENCE: 2026-09-12 `pnpm --dir website exec vitest run learn-hub-nav reference-hub-pages` → Test Files  2 passed (2)

## Pool

- [[task-870-distinct-nav-labels]]

## See also

[[ticket-850-duplicate-nav-labels]] [[website-chrome-polish]] public-site.a11y:keyboard-small public-site.ia:learn-walkable public-site.ia:reference-walkable
