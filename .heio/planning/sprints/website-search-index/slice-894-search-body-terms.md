---
id: "slice-894-search-body-terms"
title: "Search teaching-page body terms"
kind: slice
status: frozen
sprint: "website-search-index"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Search teaching-page body terms

## Why

Visitors type words that live only in teaching body, so the finder misses published pages. Title-or-heading matching is already locked. Vault phrases must stay out.

## Done

A visitor can find a Learn or Reference page by a term in that page’s teaching body, not only title or heading, while vault phrases still miss.

## Blocked by

None.

## Non-goals

- **Vault-as-site**
- **Fuzz ranking**
- **Search keyboard and live region**: [[slice-882-search-keyboard-live]]
- **Search session reset**: [[slice-873-search-session-reset]]
- **Rewriting CONTEXT terms**

## Oracle checklist

- [ ] O1: teaching-body terms hit their Learn or Reference page
  CHECK: pnpm --dir website exec vitest run search
  EXPECT: Test Files  1 passed
  EVIDENCE: pending

## Pool

- [[task-895-search-body-terms]]

## See also

[[ticket-821-search-body-terms]] [[website-search-index]] [[location-589-public-site]] [[Public site — Contract]]
