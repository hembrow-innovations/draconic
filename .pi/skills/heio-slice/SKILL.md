---
name: heio-slice
description: Orchestrate execution of a frozen heio-stack slice through railed pi-subagents.
disable-model-invocation: true
---

# Execute a frozen slice

You are the parent orchestrator. You sequence railed children. You do not implement product code. You do not write `EXPECT:`. You do not reopen a locked shape.

Load **heio-stack** before any write under `.heio/`. Load **pi-subagents** before spawn. Load **oracle** for the ledger command.

If `AGENTS.md` or `WORKSPACE.md` already names a tracker, that file wins.

## Gate

Read intent, roadmap, sprint `shape.md`, and the named slice.

- Slice missing or still `shaping` → stop. Hand to **heio-planning**.
- `EXPECT:` missing → stop. Hand to **heio-planning**.
- Intent or roadmap would have to change → **ESCALATE**. Stop.

Then set the slice `active`. Only one slice is active. Copy `templates/slice-tasks.md` only via **heio-tasker**.

Done with this gate when the slice is `frozen` or `active`, oracles parse (`--status` exit 0 or 1, not 2), and you have named the children you will spawn.

## Children

Spawn with `subagent`. `async: true`. `context: "fresh"`. One `workflowScript`. Helpers stay plain functions or Promise chains. Inline each brief. The sandbox has no parent variables.

- **heio-tasker** — writes `tasks.md` from the frozen spec + oracles. Parent cwd. No worktree.
- **heio-builder** — one task, TDD. Parent cwd. One builder at a time. No worktree (`.heio/` is gitignored; a worktree would hide the slice).
- **heio-verifier** — `--reverify` on the slice ledger. Parent cwd.
- **heio-triage** — inbound signal → TASK / TICKET / ESCALATE. Parent cwd.

Two writers never share a cwd. These children take turns on the parent cwd, so run them **sequentially**. `worktree: true` stays off.

Omit `model` when `.pi/heio-models.md` says `inherit-parent` or the file is missing.

If `subagent` is missing, play one role at a time in this session. Still switch hats. Builder pass and verifier pass stay separate. Mark `skip: no spawn runtime`.

Children do not spawn children.

## Sequence

1. If `tasks.md` is missing, spawn **heio-tasker**. Brief includes the spec path, oracles path, and “write `tasks.md` only.”
2. Next unchecked task → **heio-builder**. Brief stands alone: task id, done line, `fits:` oracle, frozen `EXPECT:` text, paths, how to verify. Builder may refine `CHECK:`, not `EXPECT:`.
3. After the builder returns `VERDICT: TASK`, git commit only the product paths that task touched. Unrelated dirty files stay unstaged. Never stage .heio/. `worktree: true` stays off.
4. After the builder returns, spawn **heio-verifier** only when the brief asked for VERIFY or the task claimed to meet an oracle. Otherwise take the next task.
5. Inbound work during the loop → **heio-triage**. TASK appends to `tasks.md`. TICKET files and continues. ESCALATE stops the loop.
6. After the last task, spawn **heio-verifier** with `--reverify`.
7. `ALL MET` → slice `met`. `HANDOFF REQUIRED` → every `ABANDON:` names a ticket id or “drop from sprint.” File those tickets. Slice `abandoned`.

```js
subagent({
  async: true,
  context: "fresh",
  workflowScript: `
    const tasker = await runs.run("tasker", {
      agent: "heio-tasker",
      task: "Goal. Slice path. Frozen spec + oracles. Write tasks.md only. Report VERDICT."
    });
    const build = await runs.run("build-t1", {
      agent: "heio-builder",
      task: "Goal. Task T1. Fits O1. EXPECT frozen. TDD. Report VERDICT."
    });
    const verify = await runs.run("verify", {
      agent: "heio-verifier",
      task: "Reverify slice ledger path. Report VERDICT VERIFY with ALL MET or HANDOFF REQUIRED."
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
