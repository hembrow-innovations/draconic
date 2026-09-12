---
id: "ticket-878-adr-0004-global-wording"
title: "What do ADR-0004 and location-218 say after dropping globalThis?"
kind: ticket
status: open
ticket_type: planning
tags: [wayfinder]
blocked_by: ["ticket-876-browser-window-self", "ticket-877-test262-absent-globalthis"]
references: ["rounds-842-global-object-identifier"]
created_at: "2026-09-12T07:03:42Z"
updated_at: "2026-09-12T07:03:42Z"
---

# What do ADR-0004 and location-218 say after dropping globalThis?

## Signal

HITL for [[rounds-842-global-object-identifier]]. Programs see `global`. No `globalThis` identifier or property. [[0004-full-ecma-262-and-embed]] currently says the destination is literally all of ECMA-262. What sentence do the reopened ADR and [[location-218-conformance]] get?

## Fit

Would rewrite a location destination. Blocked until host aliases and Test262 honesty are settled. Leave the solution off this note.

## Notes

- Do not publish slices or tasks from this round.
- Exact vault prose waits on [[ticket-876-browser-window-self]] and [[ticket-877-test262-absent-globalthis]].
