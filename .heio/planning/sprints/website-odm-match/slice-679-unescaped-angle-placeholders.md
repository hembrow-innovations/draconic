---
id: "slice-679-unescaped-angle-placeholders"
title: "Escape angle placeholders"
kind: slice
status: frozen
sprint: "website-odm-match"
blocked_by: []
tags: [website, public-site]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-07T23:15:00Z"
---

# Escape angle placeholders

## Why

CLI teaching lists write `draconic parse <file>` and `[-o <out>]`. The markdown subset currently emits those tokens as HTML tags, so the placeholders vanish and leftover nodes appear.

## Done

List and paragraph text from `website/` sources shows angle-bracket placeholders as visible characters. `/cli` Commands still reads `draconic parse <file>` and `[-o <out>]`. Markdown links still render.

## Blocked by

None.

## Non-goals

- **Changing the markdown subset of headings, lists, fences, and links**
- **Fence compile rules**
- **Playground**

## Oracle checklist

- [ ] O1: CLI Commands list HTML keeps `<file>` and `<out>` as text, not tags
  CHECK: pnpm --dir website exec vitest run markdown-render
  EXPECT: Test Files  1 passed
  EVIDENCE: pending

## Pool

- `[[task-680-unescaped-angle-placeholders]]`

## See also

[[ticket-649-unescaped-angle-placeholders]] [[website-odm-match]] public-site.markdown:subset
