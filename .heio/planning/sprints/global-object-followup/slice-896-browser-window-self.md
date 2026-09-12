---
id: "slice-896-browser-window-self"
title: "Record whether Programs see window or self"
kind: slice
status: frozen
sprint: "global-object-followup"
blocked_by: []
tags: [wayfinder]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Record whether Programs see window or self

## Why

After replacing globalThis with global, extra browser names would be host aliases beyond the language identifier. The sitting must record yes or no before wording or implementation.

## Done

A human answer is recorded: Programs see window as a global-object name, yes or no; Programs see self, yes or no. No compiler change.

## Blocked by

None.

## Non-goals

- **Test262 allowlists**: [[slice-898-test262-absent-globalthis]]
- **ADR or location wording**: [[slice-900-adr-0004-global-wording]]
- **Implementing window, self, or frames**
- **Bit-identical Node or V8**

## Oracle checklist

- [ ] O1: decision recorded
  CHECK: read Answer on ticket-876 Notes or [[rounds-842-global-object-identifier]]
  EXPECT: Answer names yes or no for window and for self
  EVIDENCE: pending
- [ ] O2: this sitting did not install host aliases
  CHECK: Checker builtin install still has no window and no self
  EXPECT: no window or self builtin
  EVIDENCE: pending

## Pool

- [[task-897-browser-window-self]]

## See also

[[ticket-876-browser-window-self]] [[global-object-followup]] [[rounds-842-global-object-identifier]]
