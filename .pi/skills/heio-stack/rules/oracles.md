---
title: Slice oracles
impact: HIGH
tags: [oracles]
---

# Slice oracles

An oracle is the slice’s done, made executable. The spec still says *why*. The ledger says *how we would know*.

Ledger path: `.heio/planning/sprints/<id>/slices/s-<slug>/oracles.md`. Format matches the **oracle** skill. Copy `templates/slice-oracles.md`.

```
node .pi/skills/oracle/scripts/oracle-check.mjs --status <ledger>
node .pi/skills/oracle/scripts/oracle-check.mjs --reverify <ledger>
```

`--reverify` is a command a *different* pass runs. **heio-verifier** owns that pass. A builder does not mark the slice met.

## Who writes what

Drafted with the slice, before tasks. The planner writes `EXPECT:` and the first `CHECK:`.

The implementer may refine `CHECK:` so the command stays runnable. The implementer leaves `EXPECT:` frozen.

## Grain

Oracles only for *external* truth — user-visible, contract, data invariant, “ops can tell.” Internal design stays in TDD. A unit test is too small to be an oracle. An oracle you can only satisfy by staring at the screen is too big to drive TDD.

Usual shape: TDD grows tests → those tests (or a thin wrapper) become the oracle’s `CHECK:`.

## Abandon

`ABANDON: <reason> → <ticket-id or "drop from sprint">`. A blank reason, or a reason with no home, is incomplete. The checker prints `HANDOFF REQUIRED`. File the ticket or drop it before calling the slice done.
