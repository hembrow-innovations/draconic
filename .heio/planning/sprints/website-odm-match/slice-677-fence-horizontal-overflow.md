---
id: "slice-677-fence-horizontal-overflow"
title: "Fences stay in the small viewport"
kind: slice
status: frozen
sprint: "website-odm-match"
blocked_by:
  - slice-689-first-h2-border
tags: [website, public-site]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-07T23:15:00Z"
---

# Fences stay in the small viewport

## Why

Article reading must work at a small viewport. On `/cli` at 375 wide, fence `pre` overflow is visible, so the page grows a horizontal scrollbar.

## Done

At 375 by 812, a docs article with long fences does not widen the document past the viewport. Fences may scroll inside themselves. Fence compile rules stay.

## Blocked by

- [[slice-689-first-h2-border]]: same docs-article CVA surface.

## Non-goals

- **Dropping fence compile or changing shipped-fence rules**
- **Playground or in-page runners**
- **Changing markdown subset besides overflow chrome**

## Oracle checklist

- [ ] O1: docs article fence chrome includes horizontal containment; docs-shell tests still pass
  CHECK: pnpm --dir website exec vitest run docs-shell mobile-a11y
  EXPECT: Test Files  2 passed
  EVIDENCE: pending

## Pool

- `[[task-678-fence-horizontal-overflow]]`

## See also

[[ticket-648-fence-horizontal-overflow]] [[website-odm-match]] public-site.a11y:keyboard-small public-site.fences:shipped-must-build
