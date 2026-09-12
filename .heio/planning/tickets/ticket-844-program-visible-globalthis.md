---
id: "ticket-844-program-visible-globalthis"
title: "Does a Program still see globalThis?"
kind: ticket
status: open
ticket_type: planning
tags: [wayfinder]
blocked_by: ["ticket-843-ecma-host-global-names"]
references: ["rounds-842-global-object-identifier"]
created_at: "2026-09-12T05:45:00Z"
updated_at: "2026-09-12T05:45:00Z"
---

# Does a Program still see globalThis?

## Signal

HITL for [[rounds-842-global-object-identifier]]. After research, does a Draconic Program still see `globalThis` as the ECMA-262 global object identifier?

Keep it: [[0004-full-ecma-262-and-embed]], ROADMAP E15.01, and `language.ecma:builtins` stay. Replacing `globalThis` everywhere is out.

Drop it as Program-visible: reopen ADR-0004 and rewrite E15.01 plus the builtins contract. That is a location rewrite for [[location-218-conformance]].

## Fit

Would rewrite a location destination if the answer is drop. Leave the solution off this note.

## Notes

Inbound: [[ticket-841-use-global-not-globalthis]]. Blocked by [[ticket-843-ecma-host-global-names]].
