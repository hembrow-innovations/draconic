---
title: Orchestration and atomic updates
impact: MEDIUM
impactDescription: Avoid needless serialization and half-applied state
tags: [std, async, orchestration, atomicity]
---

## Orchestration and atomic updates

Treat unnecessary sequential orchestration and non-atomic updates as design smells when the cleaner structure is obvious.

- If independent work is serialized for no good reason, ask whether the flow should run in parallel instead.
- If related updates can leave state half-applied, push for a more atomic structure.
- Do not over-index on micro-optimizations, but do flag avoidable orchestration complexity that makes the implementation more brittle.
