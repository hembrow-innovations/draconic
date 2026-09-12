---
id: "slice-898-test262-absent-globalthis"
title: "Record how Test262 treats absent globalThis"
kind: slice
status: frozen
sprint: "global-object-followup"
blocked_by: []
tags: [wayfinder]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Record how Test262 treats absent globalThis

## Why

Official Test262 is the external ECMA-262 bar. After dropping globalThis, the sitting must choose honest failure on the allowlist versus harness rewrite to global, or honesty on [[location-218-conformance]] is mush.

## Done

A human answer is recorded: tests that use globalThis fail honestly and stay on the allowlist, or the harness rewrites globalThis to global. No harness or allowlist edit in this sitting.

## Blocked by

None.

## Non-goals

- **ADR or location wording**: [[slice-900-adr-0004-global-wording]]
- **Implementing rewrite or allowlist edits**
- **How native globalThis.console.log lowering moves**
- **Rewriting the location-218 destination sentence**

## Oracle checklist

- [ ] O1: decision recorded
  CHECK: read Answer on ticket-877 Notes or [[rounds-842-global-object-identifier]]
  EXPECT: Answer names honest-fail-and-allowlist or harness-rewrite, not both
  EVIDENCE: pending
- [ ] O2: this sitting did not rewrite the harness
  CHECK: Test262 harness still names globalThis for fnGlobalObject and related shims
  EXPECT: no globalThis-to-global rewrite landed
  EVIDENCE: pending

## Pool

- [[task-899-test262-absent-globalthis]]

## See also

[[ticket-877-test262-absent-globalthis]] [[global-object-followup]] [[location-218-conformance]] [[0007-test262-staged-roll-in]]
