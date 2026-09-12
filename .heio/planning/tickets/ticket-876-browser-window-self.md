---
id: "ticket-876-browser-window-self"
title: "Does a Program see window or self?"
kind: ticket
status: open
ticket_type: planning
tags: [wayfinder]
blocked_by: []
references: ["rounds-842-global-object-identifier"]
created_at: "2026-09-12T07:03:42Z"
updated_at: "2026-09-12T07:03:42Z"
---

# Does a Program see window or self?

## Signal

HITL for [[rounds-842-global-object-identifier]]. Programs see `global`, not `globalThis`. Node `global` is not an extra alias ([[ticket-845-node-global-alias]] dropped). Do Programs also see browser `window` and/or `self` as names for the global object?

## Fit

Would add host aliases beyond the language identifier. Intent non-goal: bit-identical Node or V8. Leave the solution off this note.

## Notes

- Research [[ticket-843-ecma-host-global-names]]: browsers historically expose `window`, `self`, and `frames`; workers have `self`.
- Todo example binds `document` and `localStorage` from the global object, not `window`.
- Do not decide Test262 allowlists here ([[ticket-877-test262-absent-globalthis]]).
- Do not write ADR wording here ([[ticket-878-adr-0004-global-wording]]).
