---
id: "ticket-851-untitled-pages"
title: "Document title never names the page"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T05:58:32Z"
updated_at: "2026-09-12T05:58:32Z"
---

# Document title never names the page

## Signal

Website swarm 2026-09-12. Home, Learn, Reference, and article pages all use the browser title Draconic, so tabs and history do not distinguish destinations.

## Fit

Unknown until triage. No contract promise names per-page titles. Root head hardcodes the string.

## Notes

- Live titles were Draconic on `/`, `/learn`, `/reference`, `/from-javascript`, `/cli`, `/types`, `/reference-packages`
- Main headings were Draconic, Learn, Reference, from JavaScript, CLI, types, packages
- Source: `website/src/routes/__root.tsx` head meta title `Draconic`

## Parent

[[Public site purpose]]
