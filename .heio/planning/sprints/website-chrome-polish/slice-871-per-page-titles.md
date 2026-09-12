---
id: "slice-871-per-page-titles"
title: "Per-page document titles"
kind: slice
status: frozen
sprint: "website-chrome-polish"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-12T07:05:00Z"
updated_at: "2026-09-12T07:05:00Z"
---

# Per-page document titles

## Why

Home, Learn, Reference, and article pages all use the browser title Draconic, so tabs and history do not distinguish destinations. No contract promise names per-page titles yet.

## Done

Each public page's document title names the open page so tabs distinguish destinations. Home may remain Draconic.

## Blocked by

None.

## Non-goals

- **Favicon**: [[slice-863-favicon]]
- **Rewriting page h1 copy**
- **Playground, vault-as-site**

## Oracle checklist

- [ ] O1: document titles name the open page
  CHECK: pnpm --dir website exec vitest run document-title
  EXPECT: Test Files  1 passed
  EVIDENCE: pending

## Pool

- [[task-872-per-page-titles]]

## See also

[[ticket-851-untitled-pages]] [[website-chrome-polish]] [[Public site purpose]] [[Public site — Contract]]
