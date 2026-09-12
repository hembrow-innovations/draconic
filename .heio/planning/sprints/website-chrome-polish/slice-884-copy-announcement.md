---
id: "slice-884-copy-announcement"
title: "Fence copy announcement"
kind: slice
status: frozen
sprint: "website-chrome-polish"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Fence copy announcement

## Why

Learn and Reference fence Copy controls share one accessible name, announce nothing, and can stay Copied for the rest of the article. Copy-the-text already works. Home samples are a different surface.

## Done

Each Learn or Reference fence Copy control has a distinct accessible name, a successful copy is announced, and Copied does not stick for the rest of the article.

## Blocked by

None.

## Non-goals

- **Adding Copy to home samples**
- **Changing fence compile rules**
- **Rewriting `public-site.fences:copy` away from copy-the-text**
- **Playground, vault-as-site**

## Oracle checklist

- [ ] O1: fence copy announcement locks distinct names and a live confirmation
  CHECK: pnpm --dir website exec vitest run code-fence-copy
  EXPECT: Test Files  1 passed
  EVIDENCE: pending

## Pool

- [[task-885-copy-announcement]]

## See also

[[ticket-858-copy-announcement]] [[website-chrome-polish]] [[location-589-public-site]] [[Public site — Contract]]
