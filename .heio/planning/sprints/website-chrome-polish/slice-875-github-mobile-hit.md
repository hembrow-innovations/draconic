---
id: "slice-875-github-mobile-hit"
title: "Compact mobile GitHub control"
kind: slice
status: frozen
sprint: "website-chrome-polish"
blocked_by:
  - slice-865-github-above-fold
tags: [website, public-site]
created_at: "2026-09-12T07:05:00Z"
updated_at: "2026-09-12T07:05:00Z"
---

# Compact mobile GitHub control

## Why

On a small viewport the primary cluster wraps Learn and Reference into columns, and GitHub sits in a third column with stretch alignment. The link becomes a tall strip instead of a compact control. Purpose already says the side nav stacks and wraps.

## Done

At a small viewport the GitHub control is a compact tap target, not a stretched column. Keyboard focus rings the compact control. No hamburger.

## Blocked by

- [[slice-865-github-above-fold]]: place GitHub with always-visible chrome before locking wrap alignment.

## Non-goals

- **Desktop below-fold**: [[slice-865-github-above-fold]]
- **Restoring a hamburger**
- **Dropping wrap**

## Oracle checklist

- [ ] O1: small-viewport GitHub is not stretch-aligned into a tall column
  CHECK: pnpm --dir website exec vitest run mobile-a11y
  EXPECT: Test Files  1 passed
  EVIDENCE: pending

## Pool

- [[task-879-github-mobile-hit]]

## See also

[[ticket-853-github-mobile-hit-area]] [[slice-865-github-above-fold]] [[website-chrome-polish]] public-site.a11y:keyboard-small
