---
id: "ticket-855-free-console-builtin"
title: "Does a Program see free console?"
kind: ticket
status: open
ticket_type: planning
tags: [wayfinder]
blocked_by: []
references: ["rounds-842-global-object-identifier"]
created_at: "2026-09-12T06:44:27Z"
updated_at: "2026-09-12T06:44:27Z"
---

# Does a Program see free console?

## Signal

HITL for [[rounds-842-global-object-identifier]]. Conversation 2026-09-12 after L06. Free identifier `console` is unresolved today. The ask is that it hook up automatically so `console.log` works per target (`js` / `native`), without `let console = global.console`.

Keep unresolved: teaching still binds from the global object. Make it a builtin: Checker installs `console`; JS and native print without a bind. That may drop [[ticket-846-teaching-console-bind]].

## Fit

Host builtin, not an ECMA-262 remainder rewrite. Does not by itself rewrite [[location-218-conformance]]. Touches teaching on [[location-226-product]] if the bind goes away. Leave the solution off this note.

## Notes

Inbound after L06 `createLogger`. Native `globalThis.console.log` lowering already shipped. Do not decide here whether free `console` is that host object, the L06 logger, or both.
