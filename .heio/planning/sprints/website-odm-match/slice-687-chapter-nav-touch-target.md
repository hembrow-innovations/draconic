---
id: "slice-687-chapter-nav-touch-target"
title: "Chapter nav tap targets"
kind: slice
status: frozen
sprint: "website-odm-match"
blocked_by:
  - slice-685-chapter-nav-focus-ring
tags: [website, public-site]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-07T23:15:00Z"
---

# Chapter nav tap targets

## Why

Hub Learn, Reference, and GitHub use a 44px minimum height. Grouped chapter links in the same side nav have a zero min-height, so small-viewport primary nav is inconsistent.

## Done

Learn and Reference chapter links in the side nav use the same `min-h-11` tap target as hub primary-nav links.

## Blocked by

- [[slice-685-chapter-nav-focus-ring]]: same chapter-link variants; focus ring first.

## Non-goals

- **Current-page color**: [[slice-681-current-page-visual]]
- **Changing chapter order or IA**
- **Restoring a hamburger**

## Oracle checklist

- [ ] O1: chapter link variants include `min-h-11`; mobile-a11y still passes
  CHECK: pnpm --dir website exec vitest run mobile-a11y
  EXPECT: Test Files  1 passed
  EVIDENCE: pending

## Pool

- `[[task-688-chapter-nav-touch-target]]`

## See also

[[ticket-653-chapter-nav-touch-target]] [[website-odm-match]] public-site.a11y:keyboard-small
