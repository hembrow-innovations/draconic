---
id: "slice-644-ui-audit"
title: "Public-site UI audit"
kind: slice
status: frozen
sprint: "website-odm-match"
blocked_by:
  - slice-631-home-odm-layout
  - slice-632-docs-article-odm
  - slice-633-docs-nav-groups
  - slice-634-hub-cards
  - slice-635-mobile-odm-wrap
tags: [website, public-site]
created_at: "2026-09-07T19:00:00Z"
updated_at: "2026-09-07T19:00:00Z"
---

# Public-site UI audit

## Why

The ODM restyle can still leave spacing, contrast, type, leftover HTML, or small-viewport gaps. A dedicated sitting walks the live chrome and files tickets. It does not silently restyle.

## Done

Home, Learn hub, one Learn chapter, Reference hub, one Reference page, and a small viewport have been walked. Each finding is its own ticket. A closeout ticket lists pages walked and the finding ticket ids, including zero findings.

## Blocked by

- [[slice-631-home-odm-layout]] [[slice-632-docs-article-odm]] [[slice-633-docs-nav-groups]] [[slice-634-hub-cards]] [[slice-635-mobile-odm-wrap]]: audit the ODM chrome, not the old top bar.

## Non-goals

- **Implementing findings in this sitting**
- **Playground, vault-as-site, Start replacement**
- **Copying ODM product copy**
- **Dropping search, theme toggle, or fence compile**
- **Rewriting CONTEXT Learn/Reference terms**

## Oracle checklist

- [ ] O1: a closeout ticket links this slice and lists pages walked plus finding ticket ids
  CHECK: rg -l "slice-644-ui-audit" .heio/planning/tickets .heio/archive/planning/tickets
  EXPECT: ticket-
  EVIDENCE: pending

## Pool

- `[[task-645-ui-audit]]`

## See also

[[ticket-643-ui-audit]] [[website-odm-match]] [[location-589-public-site]] docs/specs/draconic/public-site/
