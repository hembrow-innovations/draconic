---
id: "slice-689-first-h2-border"
title: "First section heading rule"
kind: slice
status: frozen
sprint: "website-odm-match"
blocked_by:
  - slice-683-badge-before-heading
tags: [website, public-site]
created_at: "2026-09-07T23:15:00Z"
updated_at: "2026-09-07T23:15:00Z"
---

# First section heading rule

## Why

ODM article chrome uses a top border on section headings. Docs article CSS zeroes `h2:first-of-type`, which matches the first `h2` even when an `h1` already sits above it, so the first section never gets the rule.

## Done

Every article `h2` after the page `h1`, including the first section heading, has the token top border. `/install` Reproducibility and `/cli` Commands show the rule.

## Blocked by

- [[slice-683-badge-before-heading]]: article child order settles before heading chrome.

## Non-goals

- **Fence overflow**: [[slice-677-fence-horizontal-overflow]]
- **Changing heading copy**
- **Using DocsShell on home**

## Oracle checklist

- [ ] O1: docs article CVA no longer zeroes the first `h2` after `h1`; docs-shell tests still pass
  CHECK: pnpm --dir website exec vitest run docs-shell
  EXPECT: Test Files  1 passed
  EVIDENCE: pending

## Pool

- `[[task-690-first-h2-border]]`

## See also

[[ticket-654-first-h2-no-border]] [[slice-632-docs-article-odm]] [[website-odm-match]]
