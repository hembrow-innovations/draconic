---
id: "ticket-854-globalthis-property"
title: "Does the global object still expose globalThis?"
kind: ticket
status: open
ticket_type: planning
tags: [wayfinder]
blocked_by: []
references: ["rounds-842-global-object-identifier"]
created_at: "2026-09-12T06:28:41Z"
updated_at: "2026-09-12T06:28:41Z"
---

# Does the global object still expose globalThis?

## Signal

HITL for [[rounds-842-global-object-identifier]]. Programs no longer see a `globalThis` binding ([[ticket-844-program-visible-globalthis]]). Is `globalThis` still a property on the global object for ECMA-262 interop (`global.globalThis`), or is that property gone too?

## Fit

Would rewrite [[0004-full-ecma-262-and-embed]] and Test262 honesty on [[location-218-conformance]] if the property is gone. Leave the solution off this note.

## Notes

Surfaced after drop. Do not decide teaching bind here ([[ticket-846-teaching-console-bind]]). Do not decide Test262 allowlists here until this answer exists.
