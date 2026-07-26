---
id: issues-5
created_at: "2026-07-26"
updated_at: "2026-07-26"
area: planning
domain: language
title: "Ship a real program"
description: "Pick a small real target and drive language/toolchain gaps from actual use."
status: open
issue-type: feature-request
severity: medium
tags:
  - planning
  - issue
  - enhancement
  - needs-triage
  - examples
  - phase-2
---

# Ship a real program

## Description

Pick a small real target (CLI tool, game loop, WASM demo, etc.) and drive language/toolchain gaps from actual use instead of checklist expansion alone.

## Affected

- New `examples/` or `apps/` tree
- Gaps filed back to Roadmap or vault issues
- JS and/or native build paths

## Observed

Conformance fixtures do not stress packaging, DX, or “would a human keep using this?”.

## Impact

Unknown product fit; wrong Phase 2 priorities.

## Proposed Fix

1. Choose one target (goal, js vs native, success demo) and write it down.
2. Program in-repo; builds via `draconic`.
3. Gap list as Roadmap todos or issues as they appear.
4. Demo path: clone → build → run → observe.
5. Short write-up: solid vs blocked.

## Comments

Related: [[issues-3-native-depth]], [[issues-4-language-product-polish]]
