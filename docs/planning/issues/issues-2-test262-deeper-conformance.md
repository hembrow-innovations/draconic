---
id: issues-2
created_at: "2026-07-26"
updated_at: "2026-07-26"
area: planning
domain: language
title: "Test262 / deeper conformance"
description: "Wire official Test262 (or a curated subset) and promote failures into a new Roadmap backlog."
status: open
issue-type: feature-request
severity: high
tags:
  - planning
  - issue
  - enhancement
  - needs-triage
  - conformance
  - phase-2
---

# Test262 / deeper conformance

## Description

Wire official Test262 (or a curated subset) as the real “full ES” bar. Use failures to spawn a new E19+ (or parallel) backlog of atomic Roadmap items.

Internal fixtures (~199) prove Loop progress, not 262 completeness.

## Affected

- `tests/conformance` harness
- JS and native runners
- `ROADMAP.md` (new rows from failures)

## Observed

No Test262 integration; completeness is internal-fixture only.

## Impact

Cannot claim full ECMAScript fidelity; regressions vs the external standard are invisible.

## Proposed Fix

1. Record decision (ADR or note): full Test262 vs curated subset vs staged roll-in.
2. Harness runs selected tests on `js` and/or `native`.
3. Baseline pass/fail/skip report (checked in or CI-local script).
4. Process to promote failures into Roadmap child rows or vault issues.
5. At least one non-trivial failing area → concrete `todo` items.

## Comments

Related: [[issues-1-roadmap-honesty-pass]]
