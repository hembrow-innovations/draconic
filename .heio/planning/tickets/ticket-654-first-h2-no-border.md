---
id: "ticket-654-first-h2-no-border"
title: "First article h2 never gets the ODM section rule"
kind: ticket
status: open
ticket_type: observation
blocked_by: []
tags: [website, public-site]
sprint: "website-odm-match"
created_at: "2026-09-07T21:30:00Z"
updated_at: "2026-09-07T21:30:00Z"
---

# First article h2 never gets the ODM section rule

## Signal

ODM article chrome uses a top border on section headings. Docs article CSS sets a line on `h2`, then zeroes it for `h2:first-of-type`. That selector matches the first `h2` even when an `h1` already sits above it, so the first section never gets the rule. On `/install` the only `h2` (Reproducibility) has no border. On `/cli`, Commands has no border and later `h2`s do.

## Fit

this project, later slice. Do not execute from this ticket.

## Notes

- **Promise**: none. ODM docs-article restyle is the comparison, not a locked contract id.
- **Walked**: `/install` (Reproducibility `borderTop` 0). `/cli` (Commands 0, Permissions 1px, Shebang 1px).
- **Parent**: [[slice-644-ui-audit]]
