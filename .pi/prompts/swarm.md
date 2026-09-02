---
description: N parallel pi-subagents across slices, one report
argument-hint: "[brief]"
---

# Swarm Pi

Fan out N parallel children through `pi-subagents`. They may cover separate slices, race the same brief, or mix both. The parent waits, aggregates, and returns one report.

## Start

Open a todolist with one entry per phase before launching anything.

1. Frame
2. Fan out
3. Aggregate
4. Report

## Phase A: Frame

Done when the predicate, shape, N, per-worker agent, per-arm model, and write isolation are written down.

1. State the done predicate and the artifact or report the swarm must return.
2. Choose the shape. Partition into slices, race N workers on identical briefs, or mix both. For a race or mixed shape, declare `first pass`, `rank all`, or `best-of` before spawning.
3. Set N from the user or derive it from the shape. N is total workers.
4. Pick each child's agent. `scout` for recon and coverage. `reviewer` for verdicts. `worker` only when that slice writes.
5. Pick the worker model from `swarm workers` in `.pi/heio-models.md` when present. `inherit-parent` or a missing file means omit `model`. For a model race, run `subagent({ action: "models" })` and copy each arm's exact `provider/id` (thinking suffix allowed). Name every arm's model before spawn.
6. If any child writes, give it `worktree: true` and its own output path. Two writers never share a cwd. Before two or more writers launch, record a lane board: key, decision, isolation path.

Every brief stands alone. Include the goal, scope, exact slice or race arm, how to verify, and what to report. Reports use `PASS`, `ISSUES`, or `BLOCKED` with evidence. A brief that only swaps an item number, title, or file glob is not standalone.

## Phase B: Fan out

Done when one workflow has returned N or N-1 child results.

Load **unpark** with `subagent` and `subagent_wait` before spawn.

Spawn every worker in one `subagent` call. `async: true`. `context: "fresh"` on that call so `worker`'s default fork cannot leak. One `workflowScript`. Stable keys. `worktree: true` only on writers. Omit `model` when inheriting the parent. N=1 uses `return runs.run(...)`. N>=2 uses `return runs.all([...])`. `runs.all` returns an ordered array; map it back onto the keys. Helpers stay plain functions or Promise chains. Inline each brief in the script. The sandbox has no parent variables.

```js
subagent({
  async: true,
  context: "fresh",
  workflowScript: `
    const specs = [
      { key: "slice-a", agent: "scout", task: "Goal. Slice: auth. Verify. Report PASS, ISSUES, or BLOCKED with evidence." },
      { key: "slice-b", agent: "scout", task: "Goal. Slice: cli. Verify. Report PASS, ISSUES, or BLOCKED with evidence." }
    ];
    const results = await runs.all(specs);
    return results.map((result, i) => ({
      key: specs[i].key,
      ok: result.ok,
      output: result.output
    }));
  `
})
```

This prompt's deliverable is one report, so after launch call `subagent_wait({ all: true })`. Do not end the turn empty.

If unpark reports the tools are not registered, run the slices in the parent and mark `skip: no spawn runtime`. Do not invent child transcripts.

If a worker drops out, proceed with the returned set and note it.

Workers do the slice. They do not spawn children.

## Phase C: Aggregate

Done when every required slice has a result or a named dropout, and any race rule has been applied.

Read the workflow results. For coverage, every required slice needs a result. For a race, apply the selection rule declared up front. First pass still waits for the wave, then keeps the first `PASS`. Rank all and best-of score the full set. Summarize. Do not paste raw worker dumps.

Keep a compact result list, one-line evidenced issues, and explicit gaps or dropouts.

## Phase D: Report

Done when one in-chat report names the per-worker verdicts, issue one-liners, gaps or dropouts, and the race rule when used.

- **slice-a**: `PASS`. one-line evidence

$ARGUMENTS
