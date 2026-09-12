---
id: "slice-892-outline-current-section"
title: "Outline current section"
kind: slice
status: frozen
sprint: "website-chrome-polish"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Outline current section

## Why

On this page already lists heading hashes, but after opening a permalink every outline link still looks like a sibling and `aria-current` stays null. The list exists; the heading in view is unmarked.

## Done

On a Learn or Reference page with an on-page outline, the outline marks the heading in view so it is distinct from sibling outline links.

## Blocked by

None.

## Non-goals

- **Sticky third-column TOC**
- **Changing side-nav `aria-current=page`**
- **Inventing heading permalinks**
- **Current-page nav contrast**: [[slice-867-current-page-contrast]]

## Oracle checklist

- [ ] O1: on-page outline marks the heading in view
  CHECK: pnpm --dir website exec vitest run docs-shell
  EXPECT: Test Files  1 passed
  EVIDENCE: pending

## Pool

- [[task-893-outline-current-section]]

## See also

[[ticket-862-outline-current-section]] [[website-chrome-polish]] [[location-589-public-site]] [[Public site — Contract]]
