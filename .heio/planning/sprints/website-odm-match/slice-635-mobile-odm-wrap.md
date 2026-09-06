---
id: "slice-635-mobile-odm-wrap"
title: "ODM mobile wrap"
kind: slice
status: frozen
sprint: "website-odm-match"
blocked_by:
  - slice-630-site-shell
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T18:00:00Z"
---

# ODM mobile wrap

## Why

ODM does not use a hamburger. Below about 860px the shell becomes one column and the side nav wraps. Keyboard, skip link, and focus rings stay.

## Done

At a small viewport the side nav stacks above main and its links wrap. Skip link stays first. Primary nav remains keyboard-reachable. Focus rings use tokens.

## Blocked by

- [[slice-630-site-shell]]: wrap applies to the side nav, not a restored top header.

## Non-goals

- **Changing Learn or Reference IA**
- **Dropping search or theme toggle**
- **Playground**

## Oracle checklist

- [ ] O1: small viewport stacks the side nav; skip link first; keyboard and token focus hold
  CHECK: pnpm --dir website exec vitest run mobile-a11y
  EXPECT: Test Files  1 passed
  EVIDENCE: pending

## Pool

- `[[task-642-mobile-odm-wrap]]`

## See also

[[location-589-public-site]] [[website-odm-match]] public-site.a11y:keyboard-small website/src/tests/mobile-a11y.test.ts
