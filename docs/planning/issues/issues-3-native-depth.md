---
id: issues-3
created_at: "2026-07-26"
updated_at: "2026-07-26"
area: planning
domain: language
title: "Native depth (LLVM / Runtime)"
description: "Deepen native path: GC, performance, stdlib, dual-worlds UX, real programs — beyond fixture-green."
status: open
issue-type: feature-request
severity: medium
tags:
  - planning
  - issue
  - enhancement
  - needs-triage
  - native
  - phase-2
---

# Native depth (LLVM / Runtime)

## Description

Deepen the native path beyond fixture-green: GC quality, performance, richer stdlib, dual-worlds ergonomics, and real programs on LLVM—not just conformance stubs.

## Affected

- `crates/draconic-runtime`
- `crates/draconic-backend-llvm`
- `crates/draconic-embed`
- Dual-worlds boundary / native-only diagnostics

## Observed

N-cluster items are done with Runtime ABI + fixture observations. Production-shaped native binaries need durability and a usable std surface.

## Impact

Native target remains demo/fixture quality; hard to ship real portable+native Programs.

## Proposed Fix

1. Written gap list: GC, job queue, stdlib, Embed limits, dual-world UX, link/debug.
2. Prioritized backlog (Roadmap N-rows or issues) with measurable tests.
3. At least one improvement with benchmarks or stress tests.
4. Clear JS-only / native-only / portable policy (diagnostics, never silent wrong code).

## Comments

Related: [[issues-1-roadmap-honesty-pass]], [[issues-5-ship-real-program]]
