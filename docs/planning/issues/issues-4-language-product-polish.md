---
id: issues-4
created_at: "2026-07-26"
updated_at: "2026-07-26"
area: planning
domain: toolchain
title: "Language product polish"
description: "README/status, docs, examples, CLI UX, diagnostics quality; decide on REPL/playground."
status: open
issue-type: feature-request
severity: medium
tags:
  - planning
  - issue
  - enhancement
  - needs-triage
  - docs
  - dx
  - phase-2
---

# Language product polish

## Description

Make the toolchain presentable: README/status, docs, examples, CLI UX, diagnostic quality, optional playground/REPL.

## Affected

- `README.md` (still says bootstrap in progress)
- `crates/draconic-cli`, diagnostics
- Examples / onboarding path

## Observed

Roadmap complete while product docs lag; onboarding path unclear.

## Impact

Hard for humans (and agents) to use Draconic as a language product rather than a Loop checklist.

## Proposed Fix

1. README status, build, CLI examples match reality.
2. Minimal write→parse→build js|native path documented.
3. In-repo examples (portable + dual-worlds).
4. Diagnostics spot-check on common mistakes.
5. Decision: REPL/playground now / later / never.

## Comments
