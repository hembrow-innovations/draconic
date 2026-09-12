---
id: "ticket-847-missing-favicon"
title: "Public site favicon 404s"
kind: ticket
status: open
ticket_type: bug
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T05:58:32Z"
updated_at: "2026-09-12T05:58:32Z"
---

# Public site favicon 404s

## Signal

Website swarm 2026-09-12. Every load of the live origin logs a missing favicon. The browser tab has no site icon and the console is not clean on first paint.

## Fit

Unknown until triage. Chrome polish on [[location-589-public-site]], not a Learn or Reference copy change.

## Notes

- Live: `http://localhost:3000/` and Learn pages
- Console: `[ERROR] Failed to load resource: the server responded with a status of 404 () @ http://localhost:3000/favicon.ico`
- No favicon file under `website/`

## Parent

[[Public site — Contract]]
