---
name: oracle
description: Completion ledger under .heio/oracles.md. Write runnable oracles before work, re-verify before reporting, abandon honestly.
disable-model-invocation: true
---

# Oracle

Make incomplete work visible. Prove outcomes against a ledger. A checked box is not evidence.

Use this ledger when a false done report is expensive: long, multi-part, or AFK work.

## 1. Write oracles before implementing

Copy `.pi/skills/oracle/templates/oracles.md` to `.heio/oracles.md`. Replace every placeholder.

One **oracle** per independently required outcome. Every oracle has a `CHECK:` command and an `EXPECT:` success-only token. The command prints that token only after its assertions have passed.

`.heio/oracles.md` is reserved at the `.heio/` root. It is not a management issue, plan, or task.

Lint without executing:

```
node .pi/skills/oracle/scripts/oracle-check.mjs --status .heio/oracles.md
```

`--status` never runs `CHECK:`. Pending evidence is unmet.

Done when `--status` parses the ledger (exit 0 or 1, not 2) and every required outcome has an oracle.

## 2. Run, then re-verify

Work until the oracles can pass. Run:

```
node .pi/skills/oracle/scripts/oracle-check.mjs .heio/oracles.md
```

That run skips oracles that already have met evidence. Re-run every non-abandoned check before reporting:

```
node .pi/skills/oracle/scripts/oracle-check.mjs --reverify .heio/oracles.md
```

A runnable oracle is met only when the process exits 0 and combined output contains the `EXPECT:` token as a literal substring. Evidence records exit, match, a short hash, and byte count. Raw output is not kept.

Each `CHECK:` may run for **10 minutes** (`600_000` ms) before the checker records `exit=timeout`. That budget is for a full `cargo test --workspace` (or similar) that is still progressing, not a hang detector at two minutes. Override with `ORACLE_CHECK_TIMEOUT_MS` (positive milliseconds). `exit=timeout` with `match=yes` usually means the suite was green so far and got killed — raise the budget or wait; do not rewrite `CHECK:` to `--lib --bins` as a fake fix.

Done when `--reverify` prints `ALL MET`.

## 3. Abandon instead of deleting

When an oracle cannot be made runnable, add `ABANDON: <non-empty reason>` and stop. The checker exits 1 with `HANDOFF REQUIRED`. That is a handoff, not completion.

Done when every impossible oracle has a reason and the report lists every abandoned id.

## Ledger

```
# Oracles: <one-line outcome>

- [ ] O1: <observable outcome>
  CHECK: <command>
  EXPECT: <success-only token>
  EVIDENCE: pending
```

Ids are `O` plus a positive integer. Duplicate ids, a missing `CHECK:` or `EXPECT:`, a blank `ABANDON:` reason, or a ledger with no oracles is an error (exit 2), not completion.

Prefer a project `verify-*` skill or an existing test command as `CHECK:`. Prefer a Node script the repo owns.
