---
id: issues-7
created_at: "2026-07-26"
updated_at: "2026-07-26"
area: planning
domain: language
title: "New Roadmap phase (Loop source of truth)"
description: "Define Phase 2 Roadmap structure so draconic-loop has fresh todos and does not spin empty."
status: open
issue-type: feature-request
severity: high
tags:
  - planning
  - issue
  - enhancement
  - needs-triage
  - roadmap
  - phase-2
---

# New Roadmap phase (Loop source of truth)

## Description

Define Phase 2 Roadmap structure (e.g. **P — Production**, **S — Spec completeness**) so **draconic-loop** has a fresh checklist and does not spin on empty `todo` rows.

## Affected

- `ROADMAP.md`
- `.agents/skills/draconic-loop/`
- README agent-loop section

## Observed

All Roadmap rows `done`; unattended loops no-op or invent ad-hoc work.

## Impact

Loop skill unusable for completeness until a new phase exists.

## Proposed Fix

1. Phase name, legend, and rules (done = tests green on applicable targets).
2. Seed initial `todo` rows from [[issues-1-roadmap-honesty-pass]]–[[issues-6-architecture-cleanup]] outputs — atomic enough for one Loop each.
3. Point draconic-loop / README at the new phase without abandoning historical B/E/T/N/U rows.
4. Explicit guard: if no `todo`, stop.
5. Pilot one Loop item under the new phase.

## Blocked by

Useful seeds from [[issues-1-roadmap-honesty-pass]] and [[issues-2-test262-deeper-conformance]]; skeleton can start unblocked.

## Comments
