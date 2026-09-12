---
id: "slice-900-adr-0004-global-wording"
title: "Draft proposed ADR-0004 and location-218 sentences"
kind: slice
status: frozen
sprint: "global-object-followup"
blocked_by:
  - slice-896-browser-window-self
  - slice-898-test262-absent-globalthis
tags: [wayfinder]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Draft proposed ADR-0004 and location-218 sentences

## Why

After host aliases and Test262 honesty are answered, the map still needs proposed vault sentences. This cut records drafts. It does not apply them.

## Done

Draft proposed ADR-0004 wording, and a location-218 destination sentence only if 876 and 877 actually require one, recorded for a later map sitting. ADR-0004 and location-218 files stay unpatched.

## Blocked by

[[slice-896-browser-window-self]]: window and self answers first.

[[slice-898-test262-absent-globalthis]]: Test262 honesty mechanism first.

## Non-goals

- **Patching ADR-0004**
- **Patching location-218**
- **Rewriting a location destination during this workflow**
- **Compiler, harness, or ROADMAP edits**

## Oracle checklist

- [ ] O1: drafts recorded
  CHECK: read ticket-878 Notes after 876 and 877 answers exist
  EXPECT: proposed ADR-0004 sentence present; location-218 proposal present or explicit keep-as-is
  EVIDENCE: pending
- [ ] O2: no map rewrite landed
  CHECK: read location-218 This is working when, and ADR-0004 body
  EXPECT: location-218 still leftover ECMA-262 rows and Test262 are honest on both required targets; ADR-0004 file unchanged
  EVIDENCE: pending

## Pool

- [[task-901-adr-0004-draft-wording]]

## See also

[[ticket-878-adr-0004-global-wording]] [[global-object-followup]] [[0004-full-ecma-262-and-embed]] [[location-218-conformance]]
