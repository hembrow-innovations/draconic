---
id: issues-6
created_at: "2026-07-26"
updated_at: "2026-07-26"
area: planning
domain: toolchain
title: "Architecture cleanup after mega-loop"
description: "Enforce ~1k LOC soft limit, deepen modules, tidy Loop-left mess before the next long agent loop."
status: open
issue-type: feature-request
severity: medium
tags:
  - planning
  - issue
  - enhancement
  - needs-triage
  - refactor
  - phase-2
---

# Architecture cleanup after mega-loop

## Description

Refactor after the mega-loop: soft 1_000-line `rs` file limit, deepen modules, tidy hot paths and duplicated match arms—before the next long agent Loop.

## Affected

- Large crates under `crates/`
- AGENTS.md file-size convention
- AI navigability of the monorepo

## Observed

Loop velocity favors local edits over structure; files and match arms may exceed soft limits.

## Impact

Higher agent/human cost for Phase 2; regressions harder to review.

## Proposed Fix

1. Inventory: files over ~1_000 lines; god modules; copy-paste lowering/check arms.
2. Prioritized small-step refactor plan.
3. At least one deep-module or split landed with `cargo test --workspace` green.
4. Restate conventions so future Loops do not re-bloat.

## Comments
