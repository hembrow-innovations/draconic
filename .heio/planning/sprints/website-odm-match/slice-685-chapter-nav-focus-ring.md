---
id: "slice-685-chapter-nav-focus-ring"
title: "Chapter nav focus rings"
kind: slice
status: frozen
sprint: "website-odm-match"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-07T23:15:00Z"
---

# Chapter nav focus rings

## Why

Grouped Learn and Reference chapter links are primary nav, but they have no `focus-visible` token ring. Hub Learn, Reference, GitHub, search, theme toggle, and skip already do.

## Done

Chapter and working-page links in the side nav use the same token focus ring as hub primary-nav links.

## Blocked by

None.

## Non-goals

- **Tap-target size**: [[slice-687-chapter-nav-touch-target]]
- **Current-page color**: [[slice-681-current-page-visual]]
- **Changing Learn or Reference chapter order**

## Oracle checklist

- [ ] O1: Learn and Reference chapter link variants include token `focus-visible` ring classes
  CHECK: pnpm --dir website exec vitest run mobile-a11y learn-hub-nav reference-hub-pages
  EXPECT: Test Files  3 passed
  EVIDENCE: pending

## Pool

- `[[task-686-chapter-nav-focus-ring]]`

## See also

[[ticket-652-chapter-nav-no-focus-ring]] [[website-odm-match]] public-site.a11y:keyboard-small
