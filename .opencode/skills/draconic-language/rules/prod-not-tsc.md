---
title: TypeScript-inspired, not tsc
impact: MEDIUM
impactDescription: tsc compatibility blocks Native types and Dual worlds
tags: [product, types, tsc]
---

## TypeScript-inspired, not tsc

The Checker looks like TypeScript. It does not compile existing TypeScript projects or match tsc flag-for-flag. Untyped JS stays permissive.

**Incorrect:** emitting `.ts`, implementing `strictNullChecks` as a CLI flag, or claiming drop-in TS migration.

**Correct:** Draconic type rules in `draconic-check`. JS backend emits JavaScript. Native types are first-class, not type-erasure leftovers.

**Notes.** ADR-0005. See `js-emit-javascript` and `dual-as-boundary`.
