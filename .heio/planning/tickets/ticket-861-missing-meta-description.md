---
id: "ticket-861-missing-meta-description"
title: "Pages have no meta description"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T06:56:00Z"
updated_at: "2026-09-12T06:56:00Z"
---

# Pages have no meta description

## Signal

Playwright audit 2026-09-12. Home, Learn, and Reference documents expose a viewport tag and the title Draconic. There is no description, canonical, or Open Graph tag, so shares and search snippets have no page-specific summary.

## Fit

Unknown until triage. Distinct from open [[ticket-851-untitled-pages]] (browser title never names the page). No contract promise names meta description.

## Notes

- Live head on `/cli` and `/`: `html lang=en`, viewport `width=device-width, initial-scale=1`
- Zero `meta[name=description]`, `meta[property^=og:]`, `link[rel=canonical]`, `meta[name=theme-color]`
- Title was Draconic on every crawled path including 404

## Parent

[[Public site purpose]]
