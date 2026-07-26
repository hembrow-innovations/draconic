---
id: issues-1
created_at: "2026-07-26"
updated_at: "2026-07-26"
area: planning
domain: language
title: "Roadmap honesty pass"
description: "Audit clusters marked done for thin fixtures, native stubs, and missing edge cases; reopen or split dishonest rows."
status: open
issue-type: observation
severity: high
tags:
  - planning
  - issue
  - enhancement
  - needs-triage
  - roadmap
  - phase-2
---

# Roadmap honesty pass

## Description

Audit clusters marked `done` on [[ROADMAP]] for real gaps (thin fixtures, native stubs, half-implemented builtins, missing edge cases). Reopen or split Roadmap rows where coverage is dishonest.

“Done” today means green fixtures for the Tests column, not full semantics. Blindly starting Phase 2 on a false complete baseline will hide debt.

## Affected

- `ROADMAP.md` (E / T / N / U clusters)
- `tests/conformance` fixtures
- Native backends / Runtime where stubs remain

## Observed

Mega-loop marked ~214 items done with ~199 fixtures. Parent clusters may over-claim vs item text.

## Impact

False completeness; Phase 2 work built on gaps that look closed.

## Proposed Fix

1. Walk each E/T/N/U parent cluster; note gaps vs item text.
2. For each material gap: add a child `todo` row or file a linked issue with evidence.
3. Short report: clusters trusted vs reopened.
4. No silent deletions of ECMA-262 obligations (split or `blocked` with reason only).

## Comments
