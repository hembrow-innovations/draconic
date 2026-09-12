---
id: "ticket-845-node-global-alias"
title: "Is Node global an extra alias?"
kind: ticket
status: dropped
ticket_type: planning
tags: [wayfinder]
blocked_by: ["ticket-844-program-visible-globalthis"]
references: ["rounds-842-global-object-identifier"]
created_at: "2026-09-12T05:45:00Z"
updated_at: "2026-09-12T06:28:41Z"
---

# Is Node global an extra alias?

## Signal

HITL for [[rounds-842-global-object-identifier]]. If Programs still see `globalThis`, is Node's `global` an extra host alias, or out of scope?

Intent non-goal: bit-identical Node or V8. Location [[location-223-stdlib]] bets small honest surfaces over a Node-shaped kitchen sink.

## Fit

Blocked by [[ticket-844-program-visible-globalthis]]. If that ticket keeps `globalThis` and this answer is out of scope, [[ticket-841-use-global-not-globalthis]] may drop. Leave the solution off this note.

## Notes

Do not decide teaching bind here. That is [[ticket-846-teaching-console-bind]].

Dropped: [[ticket-844-program-visible-globalthis]] replaced Program-visible `globalThis` with `global`. Node `global` is not an extra host alias. Inverse property question is [[ticket-854-globalthis-property]].
