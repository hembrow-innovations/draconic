---
description: N parallel Task subagents across slices, one report
argument-hint: "[brief]"
---

# Swarm

Fan out N parallel children through the Task tool. They may cover separate slices, race the same brief, or mix both. The parent waits, aggregates, and returns one report.

## Start

Open a todolist with one entry per phase before launching anything.

1. Frame
2. Fan out
3. Aggregate
4. Report

## Phase A: Frame

Done when the predicate, shape, N, per-worker agent, and write isolation are written down.

1. State the done predicate and the artifact or report the swarm must return.
2. Choose the shape. Partition into slices, race N workers on identical briefs, or mix both. For a race or mixed shape, declare `first pass`, `rank all`, or `best-of` before spawning.
3. Set N from the user or derive it from the shape. N is total workers.
4. Pick each child's agent. Use a dest agent when the brief names one. Otherwise `explore` for recon, coverage, and verdicts. `general` only when that slice writes. OpenCode `scout` is for upstream docs and dependency source, not this-repo coverage.
5. Task inherits the parent model unless the chosen dest agent already pins one. A model race needs dest agents that pin different models. Name every arm's agent before spawn. Do not pass `model` on Task.
6. If any child writes, give it its own output path. Two writers never share files. Before two or more writers launch, record a lane board: key, decision, isolation path.

Every brief stands alone. Include the goal, scope, exact slice or race arm, how to verify, and what to report. Reports use `PASS`, `ISSUES`, or `BLOCKED` with evidence. A brief that only swaps an item number, title, or file glob is not standalone.

## Phase B: Fan out

Done when the wave has returned N or N-1 child results.

Spawn every worker in one turn as parallel Task calls. Foreground. Do not set `background`. Fresh session per worker (omit `task_id`). Stable keys in `description` (3-5 words). `subagent_type` is the agent from Phase A. `command` is `swarm`. Inline each brief in `prompt`.

```
Task({ description: "slice-a", subagent_type: "explore", command: "swarm", prompt: "Goal. Slice: auth. Verify. Report PASS, ISSUES, or BLOCKED with evidence." })
Task({ description: "slice-b", subagent_type: "explore", command: "swarm", prompt: "Goal. Slice: cli. Verify. Report PASS, ISSUES, or BLOCKED with evidence." })
```

This prompt's deliverable is one report, so wait for the wave. Do not end the turn empty.

If Task is denied or missing, run the slices in the parent and mark `skip: no spawn runtime`. Do not invent child transcripts.

If a worker drops out, proceed with the returned set and note it.

Workers do the slice. They do not spawn children.

## Phase C: Aggregate

Done when every required slice has a result or a named dropout, and any race rule has been applied.

Read the Task results. For coverage, every required slice needs a result. For a race, apply the selection rule declared up front. First pass still waits for the wave, then keeps the first `PASS`. Rank all and best-of score the full set. Summarize. Do not paste raw worker dumps.

Keep a compact result list, one-line evidenced issues, and explicit gaps or dropouts.

## Phase D: Report

Done when one in-chat report names the per-worker verdicts, issue one-liners, gaps or dropouts, and the race rule when used.

- **slice-a**: `PASS`. one-line evidence

$ARGUMENTS
