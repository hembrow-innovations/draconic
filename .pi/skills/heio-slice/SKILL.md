---
name: heio-slice
description: Orchestrate execution of a frozen heio-stack slice through railed pi-subagents.
disable-model-invocation: true
---

# Execute a frozen slice

You are the parent orchestrator. You sequence railed children. You do not implement product code. You do not write `EXPECT:`. You do not reopen a locked shape.

Load **heio-stack** before any write under `.heio/`. Load **unpark** then **pi-subagents** before spawn. Load **oracle** for the ledger command. The ledger path is the slice file.

If `AGENTS.md` or `WORKSPACE.md` already names a tracker, that file wins.

## Gate

Read intent, roadmap, any location file, sprint `shape.md`, and the named slice file `.heio/planning/sprints/<id>/slices/s-<slug>.md`.

- Slice missing or still `shaping` → stop. Hand to **heio-planning**.
- `EXPECT:` missing → stop. Hand to **heio-planning**.
- Intent or a location destination would have to change → **ESCALATE**. Stop.
- Slice names `blocked-by` a slice that is not `met` → stop. Work an unblocked slice instead.

Then set the slice `active`. Other unblocked slices may already be `active`.

Done with this gate when the slice is `frozen` or `active`, oracles parse on the slice file (`--status` exit 0 or 1, not 2; the ledger path is that file), and you have named the children you will spawn.

## Children

Load **unpark** with `subagent` and `subagent_wait` before any child.

Spawn with `subagent`. `async: true`. `context: "fresh"`. One `workflowScript`. Helpers stay plain functions or Promise chains. Inline each brief. The sandbox has no parent variables.

- **heio-tasker** — writes task-pool files from frozen slice Done+oracles, and adds durable `[[id]]` links on the slice. Parent cwd. No worktree.
- **heio-builder** — one task-pool task, TDD. Parent cwd. One builder at a time in this workflow. No worktree.
- **heio-verifier** — re-verify oracles on the slice file. Parent cwd.
- **heio-triage** — inbound signal → TASK / TICKET / ESCALATE. TASK means a task-pool file + slice link. Parent cwd.

Two writers never share a cwd. These children take turns on the parent cwd, so run them **sequentially in this workflow**. `worktree: true` stays off so children share parent cwd and can see `.heio/`. Another session may hold another unblocked slice.

Omit `model` when `.pi/heio-models.md` says `inherit-parent` or the file is missing.

If unpark reports the tools are not registered, play one role at a time in this session. Still switch hats. Builder pass and verifier pass stay separate. Mark `skip: no spawn runtime`.

Children do not spawn children.

## Sequence

1. If the slice has no linked task-pool ids (or linked files missing), spawn **heio-tasker**. Brief: slice file path, write task-pool files + links only.
2. Next unfinished linked task → **heio-builder**. Brief: task-pool path/id, done, fits oracle, frozen `EXPECT:`, paths, how to verify. Builder may refine `CHECK:` on the slice file, not `EXPECT:`.
3. After the builder returns `VERDICT: TASK`, git commit only the product paths that task touched. Unrelated dirty files stay unstaged. Do not commit `.heio/` tracker files. `worktree: true` stays off.
4. After the builder returns, spawn **heio-verifier** only when the brief asked for VERIFY or the task claimed to meet an oracle. Otherwise take the next task.
5. Inbound work during the loop → **heio-triage**. TASK adds a task-pool file and a slice link. TICKET files and continues. ESCALATE stops the loop.
6. After the last task, spawn **heio-verifier** on the slice file.
7. Oracles hold AND every linked task-pool id is `completed` → slice `met`. `HANDOFF REQUIRED` → every `ABANDON:` names a ticket id or “drop from sprint.” File those tickets. Slice `abandoned`.

```js
subagent({
  async: true,
  context: "fresh",
  workflowScript: `
    const tasker = await runs.run("tasker", {
      agent: "heio-tasker",
      task: "Goal. Slice file path. Write task-pool files + durable [[id]] links on the slice only. Report VERDICT."
    });
    const build = await runs.run("build-t1", {
      agent: "heio-builder",
      task: "Goal. Task-pool path/id. Done. Fits oracle. EXPECT frozen. TDD. Report VERDICT."
    });
    const verify = await runs.run("verify", {
      agent: "heio-verifier",
      task: "Reverify oracles on the slice file s-<slug>.md. Report VERDICT VERIFY with ALL MET or HANDOFF REQUIRED."
    });
    return { tasker: tasker.output, build: build.output, verify: verify.output };
  `
})
```

After launch, wait for the workflow. Do not end the turn empty.

## Loop

Every child report, and your own, ends with:

```
VERDICT: TASK | TICKET | ESCALATE | VERIFY
EVIDENCE: <one line>
```

Parent default after the last task is VERIFY. Parent default on inbound work is to spawn **heio-triage**, not to absorb it.

Done when the slice is `met` or `abandoned` with every abandoned oracle named onto a ticket or “drop from sprint.”
