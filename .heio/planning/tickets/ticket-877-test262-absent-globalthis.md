---
id: "ticket-877-test262-absent-globalthis"
title: "How does Test262 treat absent globalThis?"
kind: ticket
status: open
ticket_type: planning
tags: [wayfinder]
blocked_by: []
references: ["rounds-842-global-object-identifier"]
created_at: "2026-09-12T07:03:42Z"
updated_at: "2026-09-12T07:03:42Z"
---

# How does Test262 treat absent globalThis?

## Signal

HITL for [[rounds-842-global-object-identifier]]. `globalThis` is gone as identifier and property ([[ticket-854-globalthis-property]]). Test262 is the external ECMA-262 bar ([[0007-test262-staged-roll-in]]). Do tests that use `globalThis` fail honestly and stay on the allowlist, or does the harness rewrite `globalThis` to `global`?

## Fit

Touches Test262 honesty on [[location-218-conformance]]. Leave the solution off this note.

## Notes

- Native `globalThis.console.log` lowering already shipped; free host `console` is decided ([[ticket-855-free-console-builtin]]). How lowering moves is a later planning sitting, not this ticket.
- Do not write ADR wording here ([[ticket-878-adr-0004-global-wording]]).
