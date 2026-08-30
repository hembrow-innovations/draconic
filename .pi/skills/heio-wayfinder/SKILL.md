---
name: heio-wayfinder
description: High-level planning interview for intent, roadmap, or sprint shape under heio-stack.
disable-model-invocation: true
---

# Wayfinder on heio-stack

Interview the destination. Chart intent, the roadmap of bets, and the current sprint's shape. Do not generate the map. Do not dump the questions. Do not build.

This skill plans at the **intent** and **shape** layers. Slice specs, oracles, and tasks are **heio-planning** and **heio-slice**.

Load **heio-stack** before any write under `.heio/`. Load **docs** only when a settled decision should survive a clone. Load **domain-modeling** when a term or ADR belongs in the vault.

If `AGENTS.md` or `WORKSPACE.md` already names a tracker, that file wins.

## Rounds

The frontier is every decision whose prerequisites are already settled. Ask the whole frontier in one round. Number each question and give your recommended answer. Wait for the user's answers before the next round.

```
❓ **Q1** - **<question title>**: <question body>

➡️ <your recommended answer>

---

❓ **Q2** - **<question title>**: <question body>

➡️ <your recommended answer>
```

A question that depends on another still open in this round belongs later. Finding facts is your job. Decisions are the user's.

## Chart

User invokes with a loose idea, a roadmap, or a sprint.

Interview first. Do not write until the user confirms.

1. Destination first. That round includes nothing that hangs off it. Why this project exists, success looks like X, we will not do Y.
2. Bets in order. Each bet is a sprint and the decision that sprint must force. “This sprint exists to decide X,” not “ship by Friday.”
3. Current sprint. Slices in, slices out, the cut line. Name slices as outcomes, not layers. A slice you cannot demo or learn from in one sitting is two slices.
4. Fog and out of scope last.

Stop. Summarize. Wait for confirm. Then write.

## Write

Copy templates from **heio-stack**.

- `.heio/planning/intent.md` when intent is new or the user is changing it on purpose
- `.heio/planning/roadmap.md` with the ordered bets
- `.heio/planning/sprints/<id>/shape.md` for the current sprint. Status `shaping`. Slice folders wait for **heio-planning**.

Leave `spec.md`, `oracles.md`, and `tasks.md` unwritten. Builder agents will read these files. They must not have been written by a builder.

Done when intent, roadmap, and current sprint shape exist on disk, every in-slice has a one-line why, and no tasks exist.

## Loop

End with `VERDICT: TASK | TICKET | ESCALATE | VERIFY` per **heio-stack** `rules/loop.md`. Charting the way is not VERIFY. Newly surfaced work that is not this sprint is a TICKET. A rewrite of the bet is ESCALATE and waits.
