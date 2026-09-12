---
id: "slice-888-shipped-badge-contrast"
title: "Shipped badge contrast"
kind: slice
status: frozen
sprint: "website-chrome-polish"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Shipped badge contrast

## Why

The shipped chip is visible, but light-theme contrast is about 4.37 to 1 at 14px, under AA for normal text. Dark already passes. This is not current-page nav green.

## Done

The shipped status chip meets 4.5 to 1 contrast against its fill in light theme at mono size, stays a visible shipped label, and dark theme still passes.

## Blocked by

None.

## Non-goals

- **Current-page nav contrast**: [[slice-867-current-page-contrast]]
- **New color token names**
- **Changing which pages are shipped**
- **Learn or Reference copy**

## Oracle checklist

- [ ] O1: shipped chip light-theme contrast locks 4.5 to 1
  CHECK: pnpm --dir website exec vitest run typography-and-badge
  EXPECT: Test Files  1 passed
  EVIDENCE: pending

## Pool

- [[task-889-shipped-badge-contrast]]

## See also

[[ticket-860-shipped-badge-contrast]] [[website-chrome-polish]] [[location-589-public-site]] [[Public site — Contract]]
