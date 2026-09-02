---
title: Slice oracles
impact: HIGH
tags: [oracles]
---

# Slice oracles

An oracle is the slice’s done, made executable. Oracles live on the slice file. There is no separate ledger path.

```
- [ ] O1: <user-visible or contract outcome>
  CHECK: <command>
  EXPECT: <success-only token>
  EVIDENCE: pending
```

`EXPECT:` freezes with the slice. `CHECK:` may be refined so the command stays runnable. `EVIDENCE:` records what the check showed.

## Grain

Oracles only for *external* truth — user-visible, contract, data invariant, “ops can tell.” A check that only covers an internal unit is too small. An oracle you can only satisfy by staring at the screen is too big.

## Abandon

`ABANDON: <reason> → <ticket-id or "drop from sprint">`. A blank reason, or a reason with no home, is incomplete. File the ticket or drop it before calling the slice done.
