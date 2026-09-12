---
id: "ticket-846-teaching-console-bind"
title: "Which identifier does teaching use for console bind?"
kind: ticket
status: open
ticket_type: planning
tags: [wayfinder]
blocked_by: ["ticket-844-program-visible-globalthis"]
references: ["rounds-842-global-object-identifier"]
created_at: "2026-09-12T05:45:00Z"
updated_at: "2026-09-12T05:45:00Z"
---

# Which identifier does teaching use for console bind?

## Signal

HITL for [[rounds-842-global-object-identifier]]. README, Learn, examples, and website fences currently lock `let console = globalThis.console` because free `console` is unresolved. After the identifier decision, which name does teaching show?

## Fit

Touches [[location-226-product]] and [[location-589-public-site]] presentation, not the ECMA-262 remainder on [[location-218-conformance]]. Blocked by [[ticket-844-program-visible-globalthis]]. Leave the solution off this note.

## Notes

Inbound: [[ticket-841-use-global-not-globalthis]]. Native `globalThis.console.log` lowering already shipped.
