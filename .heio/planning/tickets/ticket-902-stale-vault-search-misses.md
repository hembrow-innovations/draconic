---
id: "ticket-902-stale-vault-search-misses"
title: "tracing GC and Ownership-only are no longer vault-only search misses"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
sprint: "website-search-index"
created_at: "2026-09-12T19:30:00Z"
updated_at: "2026-09-12T19:30:00Z"
---

# tracing GC and Ownership-only are no longer vault-only search misses

## Signal

[[task-895-search-body-terms]] listed tracing GC and Ownership-only as vault phrases that must still miss. Those strings now appear in teaching-page body (`website/content/dual-worlds.md` and related Learn pages). After teaching-body indexing they correctly hit. True vault-only phrases Public site purpose and Give someone writing a Program still miss.

## Fit

Unknown until triage. Not a finder bug. Same class of stale example as the CLI-verb notes on [[ticket-821-search-body-terms]].

## Notes

- Finder still indexes only routed `website/content/*.md`.
- Do not add a denylist for phrases that also appear in teaching copy.

## Parent

[[slice-894-search-body-terms]]
