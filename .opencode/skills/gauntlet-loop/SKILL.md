---
name: gauntlet-loop
description: Bounded builder/critic loop until the bar wins or a plateau is ticketed. Use when implementing a named task, fixing a failing CHECK, or when the brief says gauntlet / loop-until-win / do not get stuck.
---

# Gauntlet Loop

A gauntlet is a builder pass plus a separate critic pass against a real bar. It loops until the critic picks a win, or it **stops on plateau**. It does not spin.

If `AGENTS.md` says this dest is unattended Hivemind: do not spawn children. Do not interview. Do not wait. Play both hats in this sitting.

## Bar

The bar must be Named, Fetchable, and Comparable. On this dest the bar is the slice `CHECK:` / `EXPECT:` plus the task Done line. Fetch those files. Compare against that output, never against a memory of it.

Soft bars (vibes, "looks good", round-count exits) are forbidden. Green tests that do not match `EXPECT:` are not a win.

## Hats

- **Builder hat** — TDD. Smallest change that could meet the bar. Load **tdd**. Load **diagnose** on round 2+.
- **Critic hat** — run the real `CHECK:` (or the task Verify command). Read the output. Win only if it matches `EXPECT:` and the Done line holds. Otherwise name **one** gap. The critic does not implement. The critic does not see builder excuses.

Do not grade your own work in the same breath you wrote it. Finish the builder edits, then run the critic command.

## Loop

One named task is one piece.

1. Builder hat implements.
2. Critic hat runs the bar command.
3. Win → stop the piece. Task Done holds.
4. Lose → feed **only** that gap into the next builder round. Do not retry the same patch, the same file hunk, or the same hypothesis.
5. **Plateau** — the same gap (textually or semantically) on two consecutive rounds → stop. Do not soften the bar.
6. **Budget** — at most **3** critic rounds this sitting. Hitting the budget is a plateau, not a win.

Write a `## Gauntlet` section on the task-pool file after every critic round: round number, command, win/lose, gap (one line). Next sittings read it so they do not repeat a dead approach.

## Anti-stuck

These rules exist so a failing loop cannot occupy the builder forever.

- Skip task-pool files with `status: blocked` or `status: completed`.
- On plateau: set that task `status: blocked`. Mint at most one live ticket at `status: ready-for-agent` with `caused-by` this slice (or task id), `intent: fix`, `failed: true`. If a live ticket already has that `caused-by`, do not mint another.
- Do not start a fourth round. Do not start a different Roadmap item. Do not keep editing "one more try."
- If every linked task is `blocked` or `completed` and any are `blocked`, set the slice `status: failed` so Reviewer/Planner can take the ticket. Do not leave the slice `active` with only blocked work.
- If the critic command hangs, kill it and treat that as a gap. Do not wait out the sitting.

## Exit

The sitting ends on **win** or **plateau**. Both are success of the loop. A plateau that is not ticketed is a stuck failure.

```
VERDICT: TASK
EVIDENCE: win <task id, check> | plateau <task id, gap, ticket id>
```
